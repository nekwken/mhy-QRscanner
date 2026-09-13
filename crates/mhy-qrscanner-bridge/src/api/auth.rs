//! Auth API: password login, SMS login, session inspection.
//!
//! Rules:
//!
//! - The password arrives as a plain argument and is used for one request only.
//!   It is never stored, logged, or echoed back.
//! - An SMS code is supplied by the user after the server sends it; this module
//!   never guesses or automates a code.
//! - A challenge (SMS / interactive risk / real-name) is returned as
//!   [`BridgeError::Challenge`] so the UI stops and asks the user.

use super::{now_secs, session_key, validate_account};
use crate::dto::{
    AigisTokenDto, LoginChallengeDto, SessionProbeDto, SessionSummary, SmsRequestDto, StepReport,
};
use crate::events::push_step;
use crate::{BridgeError, Core};
use mhy_qrscanner_core::AccountSession;
use mhy_qrscanner_mihoyo::auth::{
    challenge_from_api_error, challenge_from_entity, HttpAuthApi, LoginAccount, LoginChallengeKind,
};
use mhy_qrscanner_mihoyo::ds::Salt;
use mhy_qrscanner_mihoyo::RpcHeaders;

/// Cookie mid fallback for sessions captured before the per-account mid was
/// known. The real value is `user_info.mid`.
const FALLBACK_COOKIE_MID: &str = "0cdapswfd1_mhy";

const AUTH_STEPS: u32 = 4;

/// Record the outcome of one auth attempt.
///
/// Deliberately coarse: an account label, the action, success, and — for a
/// challenge — its kind. The server message is **not** copied in, so nothing
/// that could carry a phone number or a code reaches the log; `retcode` carries
/// the diagnostic weight instead.
fn audit<T>(core: &Core, account: &str, action: &str, result: &Result<T, BridgeError>) {
    match result {
        Ok(_) => super::settings::record(core, account, action, true, "ok", None),
        Err(e) => {
            let detail = match e.challenge() {
                Some(c) => format!("challenge {}", c.kind),
                None => match e {
                    BridgeError::AccountNotFound(_) => "account or device not found".to_string(),
                    BridgeError::Invalid(_) => "rejected before the request".to_string(),
                    BridgeError::Storage(_) => "storage error".to_string(),
                    BridgeError::Api { .. } => "server rejected".to_string(),
                    BridgeError::Challenge(_) => "challenge".to_string(),
                    BridgeError::Capture(_) | BridgeError::Qr(_) => "failed".to_string(),
                },
            };
            super::settings::record(core, account, action, false, &detail, e.api_retcode());
        }
    }
}

/// Probe the stored session against the server: does the stoken still work?
/// Used at startup (restore) and before scanning, so an expired login is
/// reported plainly instead of failing mid-race.
pub async fn session_probe(core: &Core, account: &str) -> Result<SessionProbeDto, BridgeError> {
    validate_account(account)?;
    let stored = core
        .store()
        .load::<mhy_qrscanner_core::AccountSession>(&super::session_key(account))?;
    let Some(session) = stored.filter(|s| !s.stoken.is_empty()) else {
        return Ok(SessionProbeDto {
            valid: false,
            message: "没有已保存的会话".into(),
            uid_masked: String::new(),
        });
    };
    let headers = build_headers(core, account)?;
    let api = HttpAuthApi::new(core.passport_host());
    match api
        .get_cookie_account_info_by_stoken(&headers, &Salt::Prod, &session.stoken, &session.mid)
        .await
    {
        Ok(data) => {
            let uid = data
                .to_login_account()
                .map(|a| crate::dto::mask(&a.uid))
                .unwrap_or_default();
            Ok(SessionProbeDto {
                valid: true,
                message: "登录态有效".into(),
                uid_masked: uid,
            })
        }
        Err(e) => {
            let retcode = match &e {
                mhy_qrscanner_mihoyo::device_api::MihoyoError::Api { retcode, .. } => Some(*retcode),
                _ => None,
            };
            Ok(SessionProbeDto {
                valid: false,
                message: match retcode {
                    Some(-100) => "登录态已失效（-100），请重新登录".into(),
                    Some(code) => format!("登录态探针被拒（retcode={code}），建议重新登录"),
                    None => format!("网络错误：{e}"),
                },
                uid_masked: String::new(),
            })
        }
    }
}

fn build_headers(core: &Core, account: &str) -> Result<RpcHeaders, BridgeError> {
    // Auto-provision on first login: the user only ever enters a passport
    // identifier; the virtual device materialises behind the label.
    let profile = crate::api::device::ensure_profile(core, account)?;
    let registration = core
        .store()
        .load::<mhy_qrscanner_core::DeviceRegistration>(&super::registration_key(account))?;
    let device_fp = registration
        .as_ref()
        .map(|r| r.device_fp.clone())
        .unwrap_or_else(|| profile.initial_device_fp.clone());
    Ok(RpcHeaders {
        app_id: profile.client_profile.app_id.clone(),
        client_type: profile.client_profile.client_type.clone(),
        device_id: profile.device_id.clone(),
        device_fp,
        device_name: profile.device_name.clone(),
        device_model: profile.model.clone(),
        sys_version: profile.android_version.clone(),
        game_biz: profile.client_profile.game_biz.clone(),
        app_version: profile.client_profile.app_version.clone(),
        sdk_version: profile.client_profile.sdk_version.clone(),
        lifecycle_id: RpcHeaders::miyoushe_defaults("", "").lifecycle_id,
        account_version: profile.client_profile.sdk_version.clone(),
    })
}

fn new_session(acct: LoginAccount) -> AccountSession {
    AccountSession {
        account_uid: acct.uid,
        stoken: acct.token,
        mid: acct.mid,
        cookie_mid: FALLBACK_COOKIE_MID.to_string(),
        stoken_v2: String::new(),
        ltoken: String::new(),
        cookie_token: String::new(),
        updated_at: String::new(),
    }
}

/// Turn a challenge into the error the UI must handle.
fn challenge_error(
    kind: LoginChallengeKind,
    retcode: i64,
    message: String,
    hints: Vec<String>,
) -> BridgeError {
    BridgeError::Challenge(LoginChallengeDto::from_parts(kind, retcode, message, hints))
}

/// Map an API error into a challenge error when it is one.
fn classify(err: &mhy_qrscanner_mihoyo::device_api::MihoyoError) -> Option<BridgeError> {
    challenge_from_api_error(err)
        .map(|ch| challenge_error(ch.kind, ch.retcode, ch.message, ch.hints))
}

/// From a parsed response, produce a challenge error when no token arrived.
fn challenge_from_body(entity: &mhy_qrscanner_mihoyo::auth::LoginEntity) -> Option<BridgeError> {
    challenge_from_entity(entity)
        .map(|ch| challenge_error(ch.kind, ch.retcode, ch.message, ch.hints))
}

/// Activate the session, then fetch cookie_token + ltoken.
async fn finalize_session(
    core: &Core,
    account: &str,
    headers: &RpcHeaders,
    api: &HttpAuthApi,
    mut session: AccountSession,
    mut steps: Vec<StepReport>,
) -> Result<SessionSummary, BridgeError> {
    // A failure here is not fatal: QR approval still works with the stoken.
    match api
        .login_verify(headers, &Salt::Prod, &session.mid, &session.stoken)
        .await
    {
        Ok(entity) => {
            let mut detail = "activate session: ok".to_string();
            if let Some(a) = entity.to_login_account() {
                if !a.token.is_empty() && a.token != session.stoken {
                    session.stoken = a.token;
                    if !a.mid.is_empty() {
                        session.mid = a.mid;
                    }
                    detail = "activate session: token refreshed".to_string();
                }
            }
            push_step(&mut steps, 3, AUTH_STEPS, "login_verify", detail, true);
        }
        Err(e) => push_step(
            &mut steps,
            3,
            AUTH_STEPS,
            "login_verify",
            format!("non-fatal: {e}"),
            false,
        ),
    }

    for (dst, label) in [(4i64, "cookie_token"), (2i64, "ltoken")] {
        match api
            .exchange_token(headers, &Salt::Prod, &session.stoken, &session.mid, dst)
            .await
        {
            Ok(v) => {
                let token = v
                    .get("token")
                    .and_then(|t| t.get("token"))
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                if dst == 4 {
                    session.cookie_token = token;
                } else {
                    session.ltoken = token;
                }
                if let Some(mid) = v
                    .get("user_info")
                    .and_then(|u| u.get("mid"))
                    .and_then(|x| x.as_str())
                {
                    if !mid.is_empty() {
                        session.mid = mid.to_string();
                    }
                }
                push_step(
                    &mut steps,
                    4,
                    AUTH_STEPS,
                    "exchange",
                    format!("{label} fetched"),
                    true,
                );
            }
            Err(e) => push_step(
                &mut steps,
                4,
                AUTH_STEPS,
                "exchange",
                format!("{label} non-fatal: {e}"),
                false,
            ),
        }
    }

    session.updated_at = now_secs();
    core.store().save(&session_key(account), &session)?;
    Ok(super::session_summary(account, &session, steps))
}

/// Password login. `password` is used for this single request and never stored.
pub async fn auth_login_password(
    core: &Core,
    account: &str,
    login: &str,
    password: &str,
    aigis_token: Option<String>,
) -> Result<SessionSummary, BridgeError> {
    let result = login_password(core, account, login, password, aigis_token).await;
    audit(core, account, "auth_login_password", &result);
    result
}

async fn login_password(
    core: &Core,
    account: &str,
    login: &str,
    password: &str,
    aigis_token: Option<String>,
) -> Result<SessionSummary, BridgeError> {
    validate_account(account)?;
    if login.trim().is_empty() {
        return Err(BridgeError::Invalid("login (account) is empty".into()));
    }
    if password.is_empty() {
        return Err(BridgeError::Invalid("password is empty".into()));
    }

    let headers = build_headers(core, account)?;
    let api = HttpAuthApi::new(core.passport_host());
    let mut steps = Vec::new();

    let sent = api
        .login_by_password(
            &headers,
            &Salt::Prod,
            login,
            password,
            aigis_token.as_deref().unwrap_or(""),
        )
        .await;
    let data = match sent {
        Ok(d) => {
            push_step(
                &mut steps,
                1,
                AUTH_STEPS,
                "password",
                "credentials accepted",
                true,
            );
            d
        }
        // Same aigis gate: start an in-app solve session and hand the URL +
        // handle to the UI.
        Err(mhy_qrscanner_mihoyo::device_api::MihoyoError::AigisChallenge { challenge }) => {
            if aigis_token.is_some() {
                return Err(BridgeError::Invalid(
                    "图形验证未通过（令牌被拒），请重试登录".into(),
                ));
            }
            push_step(
                &mut steps,
                1,
                AUTH_STEPS,
                "password",
                "aigis challenge required",
                false,
            );
            return Err(begin_aigis_challenge(challenge)?);
        }
        Err(e) => {
            push_step(
                &mut steps,
                1,
                AUTH_STEPS,
                "password",
                "credentials rejected",
                false,
            );
            return Err(classify(&e).unwrap_or_else(|| e.into()));
        }
    };

    if let Some(err) = challenge_from_body(&data) {
        return Err(err);
    }
    let acct = data
        .to_login_account()
        .filter(|a| !a.token.is_empty())
        .ok_or_else(|| BridgeError::Invalid("login returned no session token".into()))?;
    push_step(
        &mut steps,
        2,
        AUTH_STEPS,
        "session",
        "session token received",
        true,
    );
    finalize_session(core, account, &headers, &api, new_session(acct), steps).await
}

/// Ask the server to send an SMS verification code. The code is entered by the user.
pub async fn auth_request_sms(
    core: &Core,
    account: &str,
    login: &str,
    area_code: &str,
    aigis_token: Option<String>,
) -> Result<SmsRequestDto, BridgeError> {
    let result = request_sms(core, account, login, area_code, aigis_token).await;
    audit(core, account, "auth_request_sms", &result);
    result
}

/// Turn an aigis challenge header into a started in-app solve session:
/// the geetest widget is served on a localhost URL that the UI embeds in a
/// webview dialog. Returns the challenge DTO carrying url + handle.
fn begin_aigis_challenge(challenge: String) -> Result<BridgeError, BridgeError> {
    let parsed = mhy_qrscanner_mihoyo::aigis::AigisChallenge::parse(&challenge)
        .ok_or_else(|| BridgeError::Invalid("unparseable aigis challenge header".into()))?;
    let (solve_id, solve_url) = mhy_qrscanner_mihoyo::aigis::begin_interactive(&parsed)
        .map_err(|e| BridgeError::Qr(format!("aigis solver: {e}")))?;
    let mut dto = LoginChallengeDto::from_parts(
        LoginChallengeKind::Aigis,
        -3101,
        "需要图形验证码（极验）".into(),
        vec![format!("gt={} risk_type={}", parsed.gt, parsed.risk_type)],
    );
    dto.aigis_url = Some(solve_url);
    dto.aigis_handle = Some(solve_id as u64);
    Ok(BridgeError::Challenge(dto))
}

/// Fetch the solved header value from a started aigis session (blocks until
/// the in-app webview posts the result, or the timeout hits).
pub async fn aigis_take(handle: u64, timeout_secs: u64) -> Result<AigisTokenDto, BridgeError> {
    let token = tokio::task::spawn_blocking(move || {
        mhy_qrscanner_mihoyo::aigis::take_result(handle as u32, std::time::Duration::from_secs(timeout_secs))
    })
    .await
    .map_err(|e| BridgeError::Qr(format!("aigis join: {e}")))?
    .map_err(|e| BridgeError::Qr(format!("aigis: {e}")))?;
    Ok(AigisTokenDto { token })
}

/// Drop a pending aigis solve (user closed the dialog without solving).
pub fn aigis_cancel(handle: u64) {
    mhy_qrscanner_mihoyo::aigis::cancel(handle as u32);
}

async fn request_sms(
    core: &Core,
    account: &str,
    login: &str,
    area_code: &str,
    aigis_token: Option<String>,
) -> Result<SmsRequestDto, BridgeError> {
    validate_account(account)?;
    if login.trim().is_empty() {
        return Err(BridgeError::Invalid("phone number is empty".into()));
    }
    let headers = build_headers(core, account)?;
    let api = HttpAuthApi::new(core.passport_host());
    let first_token = aigis_token.clone().unwrap_or_default();
    let sent = api
        .send_login_captcha(&headers, &Salt::Prod, area_code, login, &first_token)
        .await;
    let data = match sent {
        Ok(data) => data,
        // -3101 = graphic captcha required. When the caller already carries a
        // solved token we never get here (the retry succeeded or failed
        // upstream); otherwise START an in-app solve session and hand the URL
        // + handle back so the UI can embed it in a webview dialog.
        Err(mhy_qrscanner_mihoyo::device_api::MihoyoError::AigisChallenge { challenge }) => {
            if aigis_token.is_some() {
                return Err(BridgeError::Invalid(
                    "图形验证未通过（令牌被拒），请重试获取验证码".into(),
                ));
            }
            return Err(begin_aigis_challenge(challenge)?);
        }
        // Anything else is a plain refusal (bad phone, ...). Surfacing those as
        // a challenge sent the user to the "switch to SMS login" modal from the
        // very screen that just failed; the retcode travels instead so the UI
        // can show it inline and cool down before retrying.
        Err(e) => return Err(e.into()),
    };
    Ok(SmsRequestDto {
        sent_new: data
            .get("sent_new")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        countdown: data.get("countdown").and_then(|v| v.as_u64()).unwrap_or(0),
        action_type: data
            .get("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// Complete an SMS login with a user-supplied code.
pub async fn auth_submit_sms(
    core: &Core,
    account: &str,
    login: &str,
    area_code: &str,
    code: &str,
) -> Result<SessionSummary, BridgeError> {
    let result = submit_sms(core, account, login, area_code, code).await;
    audit(core, account, "auth_submit_sms", &result);
    result
}

async fn submit_sms(
    core: &Core,
    account: &str,
    login: &str,
    area_code: &str,
    code: &str,
) -> Result<SessionSummary, BridgeError> {
    validate_account(account)?;
    let code = code.trim();
    if code.is_empty() {
        return Err(BridgeError::Invalid("verification code is empty".into()));
    }
    let headers = build_headers(core, account)?;
    let api = HttpAuthApi::new(core.passport_host());
    let mut steps = Vec::new();

    let data = match api
        .login_by_mobile_captcha(&headers, &Salt::Prod, area_code, login, code)
        .await
    {
        Ok(d) => {
            push_step(&mut steps, 1, AUTH_STEPS, "sms", "code accepted", true);
            d
        }
        Err(e) => {
            push_step(&mut steps, 1, AUTH_STEPS, "sms", "code rejected", false);
            return Err(classify(&e).unwrap_or_else(|| e.into()));
        }
    };
    if let Some(err) = challenge_from_body(&data) {
        return Err(err);
    }
    let acct = data
        .to_login_account()
        .filter(|a| !a.token.is_empty())
        .ok_or_else(|| BridgeError::Invalid("SMS login returned no session token".into()))?;
    push_step(
        &mut steps,
        2,
        AUTH_STEPS,
        "session",
        "session token received",
        true,
    );
    finalize_session(core, account, &headers, &api, new_session(acct), steps).await
}

/// Masked view of a stored session.
pub fn auth_show(core: &Core, account: &str) -> Result<SessionSummary, BridgeError> {
    validate_account(account)?;
    let session = core
        .store()
        .load::<AccountSession>(&session_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(format!("{account} (no session)")))?;
    Ok(super::session_summary(account, &session, Vec::new()))
}

/// Drop the stored session, keeping the device profile.
pub fn auth_logout(core: &Core, account: &str) -> Result<(), BridgeError> {
    validate_account(account)?;
    core.store().delete(&session_key(account))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn password_login_invalid_args_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        assert!(auth_login_password(&core, "acct-1", "", "pw", None)
            .await
            .is_err());
        assert!(auth_login_password(&core, "acct-1", "user", "", None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn password_login_stores_session_masks_it_and_reports_steps() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/loginByPassword"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0,
                "message": "OK",
                "data": {
                    "token": { "token": "v2_abcdefghijklmnop", "token_type": 1 },
                    "user_info": { "aid": "42", "mid": "03pmu45q7y_mhy" }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-session/app/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK", "data": {}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-session/app/exchange"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0,
                "message": "OK",
                "data": { "token": { "token": "ct_abcdefghijklmnop", "token_type": 4 },
                          "user_info": { "mid": "03pmu45q7y_mhy" } }
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        let summary = auth_login_password(&core, "acct-1", "user", "pw-abcdefghij", None)
            .await
            .unwrap();
        assert_eq!(summary.stoken_len, "v2_abcdefghijklmnop".len());
        assert_eq!(summary.stoken_prefix, "v2_");
        assert!(summary.mid_masked.contains("..."));
        assert!(!summary.mid_masked.contains("pmu45q7y"));
        assert!(!format!("{summary:?}").contains("stoken="));
        // password, session, login_verify, then one exchange step per token
        assert_eq!(summary.steps.len(), 5);
        assert_eq!(summary.steps[0].name, "password");
        assert_eq!(summary.steps[2].name, "login_verify");
        assert!(summary.steps.iter().all(|s| s.total == AUTH_STEPS));

        let shown = auth_show(&core, "acct-1").unwrap();
        assert_eq!(shown.stoken_len, summary.stoken_len);
        assert!(shown.steps.is_empty());
    }

    #[tokio::test]
    async fn password_login_surfaces_risk_as_challenge() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/loginByPassword"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": -3235,
                "message": "您的账号存在安全风险",
                "data": null
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        let err = auth_login_password(&core, "acct-1", "user", "pw", None)
            .await
            .unwrap_err();
        assert_eq!(err.api_retcode(), Some(-3235));
        let ch = err.challenge().expect("must be a challenge");
        // -3235 is the new-device check, which an SMS code completes. Reporting it
        // as interactive risk tells the user to go and do a web risk check, which
        // dead-ends a flow that actually works.
        assert_eq!(ch.kind, "sms_captcha");
        assert!(ch.instructions.contains("短信"));
        assert!(!ch.instructions.contains("极验"));
    }

    #[tokio::test]
    async fn sms_submit_requires_a_code() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        let err = auth_submit_sms(&core, "acct-1", "13800000000", "+86", "  ")
            .await
            .unwrap_err();
        assert!(matches!(err, BridgeError::Invalid(_)));
    }

    #[tokio::test]
    async fn request_sms_reports_the_server_countdown() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0,
                "message": "OK",
                "data": { "sent_new": true, "countdown": 60,
                          "action_type": "login_by_mobile_captcha" }
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        let dto = auth_request_sms(&core, "acct-1", "13800000000", "+86", None)
            .await
            .unwrap();
        assert!(dto.sent_new);
        assert_eq!(dto.countdown, 60);
        assert_eq!(dto.action_type, "login_by_mobile_captcha");
    }

    /// `-3101` without an aigis payload is a rate limit, not a challenge: it
    /// must surface as a plain `Api` error so the UI can show it inline and
    /// cool down instead of popping the "switch to SMS" modal on the SMS
    /// screen itself.
    #[tokio::test]
    async fn sms_request_rate_limit_is_a_plain_error_not_a_challenge() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": -3101,
                "message": "\u{8bf7}\u{6c42}\u{8fc7}\u{4e8e}\u{9891}\u{7e41}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{518d}\u{8bd5}",
                "data": null
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        let err = auth_request_sms(&core, "acct-1", "13800000000", "+86", None)
            .await
            .unwrap_err();
        match err {
            BridgeError::Api { retcode, .. } => assert_eq!(retcode, -3101),
            other => panic!("expected plain Api error, got {other:?}"),
        }
        // the audit records the refusal with its retcode, not as a challenge
        let audit = crate::api::settings::audit_recent(&core, 10).unwrap();
        assert_eq!(audit[0].detail, "server rejected");
        assert_eq!(audit[0].retcode, Some(-3101));
    }

    /// -3101 **with** the challenge response header drives the full loop:
    /// the solver gets the parsed challenge, the retry carries the produced
    /// header value, and the caller sees the success DTO. (This is the exact
    /// sequence captured from the official app on 2026-09-11 23:26.)
    /// -3101 **with** the challenge header drives the full two-phase loop:
    /// request → challenge(url+handle) → (simulated browser) POST /done →
    /// aigis_take → retry with the token → success.
    #[tokio::test]
    async fn sms_request_with_aigis_challenge_solves_in_app_and_retries() {
        let challenge = r#"{"session_id":"ba48e2bf04b64c3e8d83bddc6da6ae5b","mmt_type":1,"data":"{\"success\":1,\"gt\":\"caf244bd21555cc6ce52ceca524340b8\",\"new_captcha\":1,\"use_v4\":true,\"risk_type\":\"icon\"}"}"#;
        let solved_token = {
            let parsed = mhy_qrscanner_mihoyo::aigis::AigisChallenge::parse(challenge).unwrap();
            parsed
                .build_response_header(
                    r#"{"lot_number":"L","captcha_output":"C","pass_token":"P","gen_time":"1"}"#,
                )
                .unwrap()
        };
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
            .and(header("x-rpc-aigis", solved_token.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK",
                "data": { "sent_new": true, "countdown": 60, "action_type": "login_by_mobile_captcha" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "retcode": -3101, "message": "fallback", "data": null
                    }))
                    .insert_header("x-rpc-aigis", challenge),
            )
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        // 第一段：拿到挑战（带应用内求解 URL 与句柄）
        let err = auth_request_sms(&core, "acct-1", "13800000000", "+86", None)
            .await
            .unwrap_err();
        let BridgeError::Challenge(ch) = &err else {
            panic!("expected Challenge, got {err:?}");
        };
        assert_eq!(ch.kind, "aigis");
        let url = ch.aigis_url.clone().expect("solver url");
        let handle = ch.aigis_handle.expect("solver handle");

        // 模拟求解页：把极验结果 POST 到本地服务器（blocking 客户端必须
        // 在 spawn_blocking 里用，其内部 runtime 不能在 async 上下文 drop）
        let done = tokio::task::spawn_blocking(move || {
            let http = reqwest::blocking::Client::new();
            http.post(format!("{url}/done"))
                .json(&serde_json::json!({
                    "lot_number": "L", "captcha_output": "C", "pass_token": "P", "gen_time": "1"
                }))
                .send()
                .unwrap()
        })
        .await
        .unwrap();
        assert!(done.status().is_success());

        // 第二段：取令牌 → 带令牌重发 → 成功
        let token = aigis_take(handle, 5).await.unwrap().token;
        assert_eq!(token, solved_token);
        let dto = auth_request_sms(&core, "acct-1", "13800000000", "+86", Some(token))
            .await
            .unwrap();
        assert!(dto.sent_new);
    }

    /// The audit trail is what makes a failed login diagnosable after the fact,
    /// so it must record the attempt **and** stay free of server prose.
    #[tokio::test]
    async fn auth_attempts_are_audited_without_leaking_the_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/loginByPassword"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": -3235,
                "message": "您的账号存在安全风险，请验证手机号 189******79",
                "data": null
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();

        let _ = auth_login_password(&core, "acct-1", "user", "pw", None).await;

        let audit = crate::api::settings::audit_recent(&core, 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].action, "auth_login_password");
        assert!(!audit[0].ok);
        assert_eq!(audit[0].detail, "challenge sms_captcha");
        assert_eq!(audit[0].retcode, Some(-3235));
        // neither the server's wording nor the masked phone number is copied in
        assert!(!audit[0].detail.contains("风险"));
        assert!(!audit[0].detail.contains("189"));
    }

    #[tokio::test]
    async fn a_successful_sms_request_is_audited_too() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK", "data": { "sent_new": true, "countdown": 60 }
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path()).with_passport_host(server.uri());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        auth_request_sms(&core, "acct-1", "13800000000", "+86", None)
            .await
            .unwrap();

        let audit = crate::api::settings::audit_recent(&core, 10).unwrap();
        assert_eq!(audit[0].action, "auth_request_sms");
        assert!(audit[0].ok);
    }

    #[test]
    fn logout_removes_session_only() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        auth_logout(&core, "acct-1").unwrap();
        assert!(crate::api::device::device_show(&core, "acct-1").is_ok());
        assert!(auth_show(&core, "acct-1").is_err());
    }
}
