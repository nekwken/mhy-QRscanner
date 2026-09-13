//! Passport login (password / mobile captcha). Credentials are RSA-encrypted
//! before they leave the process; plaintext password is never persisted.

use crate::device_api::MihoyoError;
use crate::ds::{create_sign, Salt};
use crate::RpcHeaders;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

/// X.509 SPKI is used by the App via `KeyFactory`; we store raw PKCS#1 SPKI body
/// after stripping PEM wrappers. The constant below is the App's public modulus
/// material (client-extracted; not a secret).
const RSA_PUBLIC_KEY_SPKI_B64: &str =
    "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDDvekdPMHN3AYhm/vktJT+YJr7cI5DcsNKqdsx5DZX0gDuWFuIjzdwButrIYPNmRJ1G8ybDIF7oDW2eEpm5sMbL9zs9ExXCdvqrn51qELbqj0XxtMTIpaCHFSI50PfPpTFV9Xt/hmyVwokoOXFlAEgCn+QCgGs52bFoYMtyi+xEQIDAQAB";

/// RSA/None/PKCS1Padding encrypt + base64 (Porte `RSAUtils.encryptByPublicKey`).
pub fn encrypt_by_public_key(plaintext: &str) -> Result<String, MihoyoError> {
    let key_bytes = B64
        .decode(RSA_PUBLIC_KEY_SPKI_B64)
        .map_err(|e| MihoyoError::InvalidResponse(format!("rsa key b64: {e}")))?;
    let key = RsaPublicKey::from_public_key_der(&key_bytes)
        .map_err(|e| MihoyoError::InvalidResponse(format!("rsa parse: {e}")))?;
    let mut rng = rand::thread_rng();
    let enc = key
        .encrypt(&mut rng, Pkcs1v15Encrypt, plaintext.as_bytes())
        .map_err(|e| MihoyoError::InvalidResponse(format!("rsa encrypt: {e}")))?;
    Ok(B64.encode(enc))
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginByPasswordRequest {
    /// RSA-encrypted account (email/phone).
    pub account: String,
    /// RSA-encrypted password.
    pub password: String,
}

/// Session-shaped account view after a successful login.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct LoginAccount {
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub mid: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    retcode: i64,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenEntity {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub token_type: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfoEntity {
    /// numeric account id (server may send string or number)
    #[serde(default, deserialize_with = "de_opt_i64_flex")]
    pub aid: Option<i64>,
    #[serde(default)]
    pub mid: String,
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub mobile: String,
}

fn de_opt_i64_flex<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_i64()),
        Some(Value::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse::<i64>().map(Some).map_err(D::Error::custom)
            }
        }
        Some(other) => Err(D::Error::custom(format!("invalid aid type: {other}"))),
    }
}

/// Real `LoginEntity` from Porte (not a nested `account` object).
#[derive(Debug, Clone, Deserialize)]
pub struct LoginEntity {
    #[serde(default)]
    pub token: Option<TokenEntity>,
    #[serde(default)]
    pub user_info: Option<UserInfoEntity>,
    #[serde(default)]
    pub login_ticket: Option<String>,
    #[serde(default)]
    pub need_realperson: Option<bool>,
    /// raw remainder (realname_info, risk fields, …)
    #[serde(flatten)]
    pub rest: Value,
}

impl LoginEntity {
    /// Flatten to the session-shaped account view used by CLI/store.
    pub fn to_login_account(&self) -> Option<LoginAccount> {
        let token = self.token.as_ref()?.token.clone();
        if token.is_empty() {
            return None;
        }
        let info = self.user_info.as_ref();
        Some(LoginAccount {
            uid: info
                .and_then(|i| i.aid)
                .map(|a| a.to_string())
                .unwrap_or_default(),
            mid: info.map(|i| i.mid.clone()).unwrap_or_default(),
            token,
            name: info.map(|i| i.account_name.clone()).unwrap_or_default(),
        })
    }

    pub fn risk_hint(&self) -> Option<String> {
        if self.need_realperson == Some(true) {
            return Some("need_realperson".into());
        }
        if !self.rest.is_null() {
            if let Some(obj) = self.rest.as_object() {
                for k in obj.keys() {
                    let lk = k.to_ascii_lowercase();
                    if lk.contains("risk") || lk.contains("aigis") || lk.contains("geetest") {
                        return Some(k.clone());
                    }
                }
            }
        }
        None
    }
}

pub type LoginData = LoginEntity;

pub struct HttpAuthApi {
    client: reqwest::Client,
    login_base: String,
}

/// Classified login result for UI/CLI handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginChallengeKind {
    /// SMS verification code required (manual input).
    SmsCaptcha,
    /// Graphic captcha (aigis / Geetest v4) required before the request will
    /// proceed. `-3101`, challenge delivered in the response header.
    Aigis,
    /// Interactive geetest / aigis / web risk check.
    InteractiveRisk,
    /// Real-name verification required.
    RealName,
    /// Unknown risk; user should retry after completing challenge in official client.
    Unknown,
}

/// Challenge info returned when login does not yield a usable stoken.
#[derive(Debug, Clone)]
pub struct LoginChallenge {
    pub kind: LoginChallengeKind,
    pub retcode: i64,
    pub message: String,
    /// Field names / notes from the response body (no secrets).
    pub hints: Vec<String>,
}

impl LoginChallenge {
    /// Human-readable manual instructions (no auto-bypass).
    pub fn manual_instructions(&self) -> &'static str {
        match self.kind {
            LoginChallengeKind::SmsCaptcha => {
                "1) Run `mhyqr auth login-sms --account <id> --login <phone>` and type the SMS code\n\
                 2) Or complete SMS login in the official Miyoushe app, then retry password login from a calmer network"
            }
            LoginChallengeKind::Aigis => {
                "Complete the graphic captcha in the opened browser page; the request is then retried\n\
                 automatically. The tool runs Geetest's own widget — the human solves, never a bypass."
            }
            LoginChallengeKind::InteractiveRisk => {
                "1) Open the official Miyoushe app or https://user.mihoyo.com and finish the risk/captcha dialog\n\
                 2) Wait a few minutes, then retry `mhyqr auth login-password`\n\
                 3) This tool will not automate geetest/aigis"
            }
            LoginChallengeKind::RealName => {
                "Complete real-name verification in the official app, then retry login"
            }
            LoginChallengeKind::Unknown => {
                "Retry later from a known-good network; or use SMS login if offered.\n\
                 This tool never bypasses account risk controls."
            }
        }
    }
}

/// Map passport retcode / body into a challenge kind.
pub fn classify_challenge(retcode: i64, message: &str, body: &Value) -> LoginChallengeKind {
    // -3235 is the new-device check: the server starts asking for an SMS code and
    // `createLoginCaptcha` + `loginByMobileCaptcha` completes it. Evidence: the
    // a live run (fresh virtual device, new-device verification on) and the
    // official client's `sent_new` response.
    // It must be tested **before** the "风险" text match below, which would
    // otherwise send the user to an interactive-risk dead end.
    if retcode == -3235 {
        return LoginChallengeKind::SmsCaptcha;
    }
    // -3101 officially means "needs aigis": the
    // server wants the Geetest v4 graphic captcha, solved in the tool's browser
    // page, before it will proceed. The misleading `请求频繁` message text is
    // NOT a rate limit — the captured flow passes right after solving.
    if retcode == -3101 {
        return LoginChallengeKind::Aigis;
    }
    // -3202/-3203/-3201: risk / ban families; message text as a fallback for
    // captcha-required wordings that arrive under other retcodes.
    if retcode == -3102
        || retcode == -3201
        || message.contains("验证码")
        || message.contains("captcha")
        || message.contains("短信")
    {
        return LoginChallengeKind::SmsCaptcha;
    }
    // -3503 is an inconsistent device identity (`请求失败，当前设备或网络环境存在风险`).
    // Unlike -3235 that one is not solved by an SMS code, so it stays interactive.
    if retcode == -3202
        || retcode == -3203
        || retcode == -3501
        || retcode == -3503
        || retcode == -100
        || message.contains("风险")
        || message.contains("aigis")
        || message.contains("geetest")
        || message.contains("验证")
    {
        return LoginChallengeKind::InteractiveRisk;
    }
    if message.contains("实名") || message.contains("realname") {
        return LoginChallengeKind::RealName;
    }
    if let Some(obj) = body.as_object() {
        for k in obj.keys() {
            let lk = k.to_ascii_lowercase();
            if lk.contains("aigis") || lk.contains("geetest") || lk.contains("risk_verify") {
                return LoginChallengeKind::InteractiveRisk;
            }
            if lk.contains("sms") || lk.contains("captcha") {
                return LoginChallengeKind::SmsCaptcha;
            }
        }
    }
    LoginChallengeKind::Unknown
}

/// Detect whether a parsed LoginEntity still needs manual challenge work.
pub fn challenge_from_entity(entity: &LoginEntity) -> Option<LoginChallenge> {
    if let Some(acct) = entity.to_login_account() {
        if !acct.token.is_empty() {
            return None;
        }
    }
    let mut hints = Vec::new();
    if entity.need_realperson == Some(true) {
        hints.push("need_realperson".into());
        return Some(LoginChallenge {
            kind: LoginChallengeKind::RealName,
            retcode: 0,
            message: "need_realperson".into(),
            hints,
        });
    }
    if let Some(risk) = entity.risk_hint() {
        hints.push(risk);
    }
    if hints.is_empty() {
        // no token but no explicit risk field — still a challenge
        hints.push("missing_stoken".into());
    }
    let message = "login response has no stoken".to_string();
    Some(LoginChallenge {
        kind: classify_challenge(0, &message, &entity.rest),
        retcode: 0,
        message,
        hints,
    })
}

/// Classify a transport/API error into a manual challenge when applicable.
pub fn challenge_from_api_error(err: &MihoyoError) -> Option<LoginChallenge> {
    match err {
        MihoyoError::AigisChallenge { challenge } => {
            let parsed = crate::aigis::AigisChallenge::parse(challenge);
            let hints = match &parsed {
                Some(c) => vec![format!("gt={} risk_type={}", c.gt, c.risk_type)],
                None => vec!["unparseable aigis challenge header".into()],
            };
            Some(LoginChallenge {
                kind: LoginChallengeKind::Aigis,
                retcode: -3101,
                message: "需要图形验证码（极验）".into(),
                hints,
            })
        }
        MihoyoError::Api { retcode, message } => {
            let kind = classify_challenge(*retcode, message, &Value::Null);
            Some(LoginChallenge {
                kind,
                retcode: *retcode,
                message: message.clone(),
                hints: vec![],
            })
        }
        _ => None,
    }
}

impl HttpAuthApi {
    pub fn new(login_base: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let mut base = login_base.into();
        while base.ends_with('/') {
            base.pop();
        }
        Self {
            client,
            login_base: base,
        }
    }

    async fn post_signed(
        &self,
        path: &str,
        headers: &RpcHeaders,
        salt: &Salt,
        aigis: Option<&str>,
        body: &Value,
    ) -> Result<Value, MihoyoError> {
        let ds = create_sign(body, salt.clone(), None, None);
        let url = format!("{}/{}", self.login_base, path);
        let mut rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("x-rpc-app_id", &headers.app_id)
            .header("x-rpc-client_type", &headers.client_type)
            .header("x-rpc-device_id", &headers.device_id)
            .header("x-rpc-device_fp", &headers.device_fp)
            .header("x-rpc-device_name", &headers.device_name)
            .header("x-rpc-device_model", &headers.device_model)
            .header("x-rpc-sys_version", &headers.sys_version)
            .header("x-rpc-game_biz", &headers.game_biz)
            .header("x-rpc-app_version", &headers.app_version)
            .header("x-rpc-sdk_version", &headers.sdk_version)
            .header("x-rpc-lifecycle_id", &headers.lifecycle_id)
            .header("x-rpc-account_version", &headers.account_version);
        if let Some(a) = aigis {
            rb = rb.header("x-rpc-aigis", a);
        }
        let resp = rb.json(body).send().await?.error_for_status()?;
        // On -3101 the challenge arrives in the *response header* `x-rpc-aigis`
        // (the body is empty) — confirmed against the official client +
        // captured traffic 2026-09-11 23:26. Read it before the envelope
        // consumes the response.
        let aigis_challenge = resp
            .headers()
            .get("x-rpc-aigis")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let resp = resp.json::<Envelope<Value>>().await?;
        if resp.retcode != 0 {
            if resp.retcode == -3101 {
                if let Some(challenge) = aigis_challenge {
                    return Err(MihoyoError::AigisChallenge { challenge });
                }
            }
            return Err(MihoyoError::Api {
                retcode: resp.retcode,
                message: resp.message,
            });
        }
        Ok(resp.data.unwrap_or(Value::Null))
    }

    /// POST account/ma-cn-passport/app/loginByPassword
    pub async fn login_by_password(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        account_plain: &str,
        password_plain: &str,
        aigis: &str,
    ) -> Result<LoginData, MihoyoError> {
        let body = json!({
            "account": encrypt_by_public_key(account_plain)?,
            "password": encrypt_by_public_key(password_plain)?,
        });
        let data = self
            .post_signed(
                "account/ma-cn-passport/app/loginByPassword",
                headers,
                salt,
                Some(aigis),
                &body,
            )
            .await?;
        parse_login_entity(data)
    }

    /// POST account/ma-cn-session/app/verify
    ///
    /// Porte calls this after password login when token type is STOKEN
    /// (`LoginManager.loginVerify`): body `{mid, token, refresh:true}`.
    /// Completes session activation / new-device verification handshake.
    pub async fn login_verify(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        mid: &str,
        stoken: &str,
    ) -> Result<LoginEntity, MihoyoError> {
        let body = json!({
            "mid": mid,
            "token": {
                "token": stoken,
                "token_type": 1
            },
            "refresh": true
        });
        let data = self
            .post_signed(
                "account/ma-cn-session/app/verify",
                headers,
                salt,
                None,
                &body,
            )
            .await?;
        parse_login_entity(data)
    }

    /// POST common/aigis/api/preCheck (geetest/aigis bootstrap).
    pub async fn aigis_pre_check(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
    ) -> Result<Value, MihoyoError> {
        // Body fields from App vary; empty object is accepted by preCheck in many builds.
        let body = json!({});
        self.post_signed("common/aigis/api/preCheck", headers, salt, None, &body)
            .await
    }

    /// POST account/ma-cn-session/app/exchange
    /// Swap stoken (type 1) for cookie_token (4) or ltoken (2).
    pub async fn exchange_token(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        stoken: &str,
        mid: &str,
        dst_token_type: i64,
    ) -> Result<Value, MihoyoError> {
        let body = json!({
            "src_token": {
                "token": stoken,
                "token_type": 1
            },
            "mid": mid,
            "dst_token_type": dst_token_type
        });
        self.post_signed(
            "account/ma-cn-session/app/exchange",
            headers,
            salt,
            None,
            &body,
        )
        .await
    }

    /// POST account/ma-cn-session/app/exchange
    /// Swap stoken (type 1) for cookie_token (type 4).
    pub async fn exchange_stoken_for_cookie_token(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        stoken: &str,
        mid: &str,
    ) -> Result<Value, MihoyoError> {
        self.exchange_token(headers, salt, stoken, mid, 4).await
    }

    /// POST account/ma-cn-verifier/verifier/createLoginCaptcha
    ///
    /// POST account/ma-cn-verifier/verifier/createLoginCaptcha
    ///
    /// Since 2026-09-11 the server demands an aigis (Geetest v4) token: the
    /// first call sends `aigis = ""` and is refused with `-3101` plus the
    /// challenge in the *response header*; after the user solves it in the
    /// tool's browser page, retry with `aigis = build_response_header(...)`.
    /// (The App sends an empty header on the first attempt too — official client
    /// capture — and nothing on the submit call.)
    pub async fn send_login_captcha(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        area_code_plain: &str,
        mobile_plain: &str,
        aigis: &str,
    ) -> Result<Value, MihoyoError> {
        let body = json!({
            "area_code": encrypt_by_public_key(area_code_plain)?,
            "mobile": encrypt_by_public_key(mobile_plain)?,
        });
        self.post_signed(
            "account/ma-cn-verifier/verifier/createLoginCaptcha",
            headers,
            salt,
            Some(aigis),
            &body,
        )
        .await
    }

    /// POST account/ma-cn-passport/app/loginByMobileCaptcha
    pub async fn login_by_mobile_captcha(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        area_code_plain: &str,
        mobile_plain: &str,
        captcha: &str,
    ) -> Result<LoginData, MihoyoError> {
        let body = json!({
            "area_code": encrypt_by_public_key(area_code_plain)?,
            "mobile": encrypt_by_public_key(mobile_plain)?,
            "captcha": captcha,
            "action_type": "login_by_mobile_captcha"
        });
        let data = self
            .post_signed(
                "account/ma-cn-passport/app/loginByMobileCaptcha",
                headers,
                salt,
                None,
                &body,
            )
            .await?;
        parse_login_entity(data)
    }

    /// POST account/ma-cn-session/app/getTokenByGameToken
    /// Body: `{ account_type: 1, game_token }` (DS signed).
    pub async fn get_token_by_game_token(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        game_token: &str,
        account_type: i64,
    ) -> Result<LoginData, MihoyoError> {
        let body = json!({
            "account_type": account_type,
            "game_token": game_token,
        });
        let data = self
            .post_signed(
                "account/ma-cn-session/app/getTokenByGameToken",
                headers,
                salt,
                None,
                &body,
            )
            .await?;
        parse_login_entity(data)
    }

    /// POST account/ma-cn-session/app/getTokenBySToken
    /// Cookie form used by App: `stuid={uid};stoken={stoken}`.
    /// Body is empty JSON `{}`; DS signs the empty object.
    pub async fn get_token_by_stoken(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        uid: &str,
        stoken: &str,
    ) -> Result<LoginEntity, MihoyoError> {
        let body = json!({});
        let ds = create_sign(&body, salt.clone(), None, None);
        let url = format!(
            "{}/account/ma-cn-session/app/getTokenBySToken",
            self.login_base
        );
        let cookie = format!("stuid={uid};stoken={stoken}");
        let mut rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("Cookie", cookie)
            .header("x-rpc-app_id", &headers.app_id)
            .header("x-rpc-client_type", &headers.client_type)
            .header("x-rpc-device_id", &headers.device_id)
            .header("x-rpc-device_fp", &headers.device_fp)
            .header("x-rpc-sdk_version", &headers.sdk_version)
            .header("x-rpc-lifecycle_id", &headers.lifecycle_id)
            .header("x-rpc-account_version", &headers.account_version);
        if !headers.device_name.is_empty() {
            rb = rb.header("x-rpc-device_name", &headers.device_name);
        }
        if !headers.device_model.is_empty() {
            rb = rb.header("x-rpc-device_model", &headers.device_model);
        }
        if !headers.sys_version.is_empty() {
            rb = rb.header("x-rpc-sys_version", &headers.sys_version);
        }
        if !headers.game_biz.is_empty() {
            rb = rb.header("x-rpc-game_biz", &headers.game_biz);
        }
        if !headers.app_version.is_empty() {
            rb = rb.header("x-rpc-app_version", &headers.app_version);
        }
        let resp = rb
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Envelope<Value>>()
            .await?;
        if resp.retcode != 0 {
            return Err(MihoyoError::Api {
                retcode: resp.retcode,
                message: resp.message,
            });
        }
        parse_login_entity(resp.data.unwrap_or(Value::Null))
    }

    /// GET account/auth/api/getCookieAccountInfoBySToken (DS + Cookie).
    pub async fn get_cookie_account_info_by_stoken(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        stoken: &str,
        mid: &str,
    ) -> Result<LoginEntity, MihoyoError> {
        let empty = json!({});
        let ds = create_sign(&empty, salt.clone(), None, None);
        let url = format!(
            "{}/account/auth/api/getCookieAccountInfoBySToken",
            self.login_base
        );
        let cookie = format!("stoken={stoken};mid={mid}");
        let rb = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("Cookie", cookie)
            .header("x-rpc-app_id", &headers.app_id)
            .header("x-rpc-client_type", &headers.client_type)
            .header("x-rpc-device_id", &headers.device_id)
            .header("x-rpc-device_fp", &headers.device_fp)
            .header("x-rpc-sdk_version", &headers.sdk_version)
            .header("x-rpc-lifecycle_id", &headers.lifecycle_id)
            .header("x-rpc-account_version", &headers.account_version);
        let resp = rb
            .send()
            .await?
            .error_for_status()?
            .json::<Envelope<Value>>()
            .await?;
        if resp.retcode != 0 {
            return Err(MihoyoError::Api {
                retcode: resp.retcode,
                message: resp.message,
            });
        }
        parse_login_entity(resp.data.unwrap_or(Value::Null))
    }
}

fn parse_login_entity(data: Value) -> Result<LoginEntity, MihoyoError> {
    if data.is_null() {
        return Err(MihoyoError::InvalidResponse(
            "login data is null (risk/challenge likely)".into(),
        ));
    }
    // If the server already returned a flattened account, adapt it.
    if let Some(acct) = data.get("account") {
        if acct.get("token").is_some() || acct.get("uid").is_some() {
            let token = acct
                .get("token")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let uid = acct
                .get("uid")
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => String::new(),
                })
                .unwrap_or_default();
            let mid = acct
                .get("mid")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            return Ok(LoginEntity {
                token: Some(TokenEntity {
                    token,
                    token_type: 0,
                }),
                user_info: Some(UserInfoEntity {
                    aid: uid.parse().ok(),
                    mid,
                    account_name: String::new(),
                    email: String::new(),
                    mobile: String::new(),
                }),
                login_ticket: None,
                need_realperson: None,
                rest: data,
            });
        }
    }
    serde_json::from_value(data)
        .map_err(|e| MihoyoError::InvalidResponse(format!("login entity parse: {e}")))
        .inspect(|e: &LoginEntity| {
            if std::env::var("MHYQR_DEBUG").is_ok() {
                eprintln!(
                    "login_entity token_type={:?} login_ticket_len={:?} rest_keys={:?}",
                    e.token.as_ref().map(|t| t.token_type),
                    e.login_ticket.as_ref().map(|s| s.len()),
                    e.rest
                        .as_object()
                        .map(|o| o.keys().cloned().collect::<Vec<_>>())
                );
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsa_encrypt_is_base64_and_nondeterministic() {
        let a = encrypt_by_public_key("hello").unwrap();
        let b = encrypt_by_public_key("hello").unwrap();
        assert!(!a.is_empty());
        assert!(B64.decode(&a).is_ok());
        // PKCS1 v1.5 uses random padding
        assert_ne!(a, b);
    }

    #[test]
    fn classify_sms_and_risk_retcodes() {
        // -3101 officially means "needs aigis";
        // its 请求频繁 message text is a decoy, not a rate limit.
        assert_eq!(
            classify_challenge(-3101, "请求频繁，请稍后再试", &Value::Null),
            LoginChallengeKind::Aigis
        );
        // new-device verification: solvable with an SMS code, not a web risk check
        assert_eq!(
            classify_challenge(-3235, "您的账号存在安全风险", &Value::Null),
            LoginChallengeKind::SmsCaptcha
        );
        // inconsistent device identity: an SMS code does not help here
        assert_eq!(
            classify_challenge(-3503, "请求失败，当前设备或网络环境存在风险", &Value::Null),
            LoginChallengeKind::InteractiveRisk
        );
        assert_eq!(
            classify_challenge(-100, "登录状态失效，请重新登录", &Value::Null),
            LoginChallengeKind::InteractiveRisk
        );
        let body = json!({"aigis": "xxx"});
        assert_eq!(
            classify_challenge(0, "ok", &body),
            LoginChallengeKind::InteractiveRisk
        );
    }

    #[test]
    fn password_request_serializes() {
        let r = LoginByPasswordRequest {
            account: "enc-a".into(),
            password: "enc-p".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["account"], "enc-a");
    }

    #[test]
    fn login_entity_accepts_string_aid() {
        let raw = serde_json::json!({
            "token": { "token": "st", "token_type": 1 },
            "user_info": { "aid": "317466433", "mid": "m", "account_name": "n" }
        });
        let e: LoginEntity = serde_json::from_value(raw).unwrap();
        let a = e.to_login_account().unwrap();
        assert_eq!(a.uid, "317466433");
        assert_eq!(a.mid, "m");
    }

    /// The SMS wire contract, pinned against official-client traffic.
    ///
    /// `createLoginCaptcha` takes only the RSA-encrypted pair and sends an empty
    /// `x-rpc-aigis`; `loginByMobileCaptcha` adds a plaintext code and a
    /// **string** `action_type`. Getting the type wrong here is invisible until
    /// a live login fails, so it is asserted on the actual request bytes.
    mod sms_wire {
        use super::*;
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        fn headers() -> RpcHeaders {
            RpcHeaders::miyoushe_defaults("dev-1", "fp-1")
        }

        async fn call(path: &str, submit: bool) -> wiremock::Request {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "retcode": 0, "message": "OK",
                    "data": { "sent_new": true, "countdown": 60 }
                })))
                .mount(&server)
                .await;
            let api = HttpAuthApi::new(server.uri());
            if submit {
                let _ = api
                    .login_by_mobile_captcha(
                        &headers(),
                        &Salt::Prod,
                        "+86",
                        "13800000000",
                        "123456",
                    )
                    .await;
            } else {
                let _ = api
                    .send_login_captcha(&headers(), &Salt::Prod, "+86", "13800000000", "")
                    .await;
            }
            let received = server.received_requests().await.unwrap();
            let request = received
                .iter()
                .find(|r| r.url.path() == path)
                .expect("expected request");
            request.clone()
        }

        #[tokio::test]
        async fn create_login_captcha_sends_only_the_encrypted_pair() {
            let request = call("/account/ma-cn-verifier/verifier/createLoginCaptcha", false).await;
            let body: Value = request.body_json().unwrap();
            let keys: Vec<&str> = body
                .as_object()
                .unwrap()
                .keys()
                .map(|k| k.as_str())
                .collect();
            assert_eq!(keys.len(), 2, "unexpected fields: {keys:?}");
            assert!(body.get("area_code").is_some());
            assert!(body.get("mobile").is_some());
            assert!(body.get("action_type").is_none());
            // the encrypted values must not be the plaintext we passed in
            assert_ne!(body["area_code"], "+86");
            assert_ne!(body["mobile"], "13800000000");
            // the App sends this header present-but-empty on this endpoint
            let aigis = request
                .headers
                .get("x-rpc-aigis")
                .expect("x-rpc-aigis header must be present");
            assert_eq!(aigis.to_str().unwrap(), "");
        }

        #[tokio::test]
        async fn login_by_mobile_captcha_uses_a_string_action_type() {
            let request = call("/account/ma-cn-passport/app/loginByMobileCaptcha", true).await;
            let body: Value = request.body_json().unwrap();
            assert_eq!(body["captcha"], "123456");
            assert_eq!(body["action_type"], "login_by_mobile_captcha");
            assert!(body["action_type"].is_string());
            // and no aigis header on the submit call, matching the capture
            assert!(!request.headers.contains_key("x-rpc-aigis"));
        }
    }
}
