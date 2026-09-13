//! Miyoushe QR dual-role client (Porte / panda).
//!
//! - **V1 game QR** (`qr_code_in_game.html`): panda `combo/panda/qrcode/scan`,
//!   `x-rpc-*` headers only, no DS.
//! - **V2 passport QR** (`mobile.html#/login/qr`): `scanQRLogin` / `confirmQRLogin`
//!   with DS + Cookie (`stoken=...;mid=...`).

use mhy_qrscanner_mihoyo::device_api::MihoyoError;
use mhy_qrscanner_mihoyo::ds::{create_sign, Salt};
pub use mhy_qrscanner_mihoyo::RpcHeaders;
use serde_json::Value;
use std::time::Duration;

mod cookie;
mod decode;
mod parse;
mod stability;
mod types;

pub use cookie::{passport_qr_cookie, DEFAULT_COOKIE_MID};
pub use decode::{decode_qr_file, decode_qr_image, decode_qr_luma};
pub use parse::{
    is_v1_qr_url, panda_host_for, panda_host_is_derived, panda_scan_from_v1_url, parse_v1_qr_url,
    parse_v2_qr_url, ticket_from_v1_url, HK4E_PANDA_BASE, PASSPORT_BASE,
};
pub use stability::StabilityFilter;
pub use types::{PandaQrScanRequest, PassportQrConfirmRequest, PassportQrScanRequest, QrScanData};

use types::Envelope;

/// HTTP client for both QR roles.
pub struct HttpQrApi {
    client: reqwest::Client,
    panda_base: String,
    login_base: String,
}

impl HttpQrApi {
    /// `panda_base` e.g. `https://api-sdk.mihoyo.com`
    /// `login_base` e.g. `https://passport-api.mihoyo.com`
    pub fn new(panda_base: impl Into<String>, login_base: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            client,
            panda_base: trim_slash(panda_base.into()),
            login_base: trim_slash(login_base.into()),
        }
    }

    /// V1: POST panda scan (no DS). Host is game-specific, e.g.
    /// `https://hk4e-sdk.mihoyo.com` + `hk4e_cn/combo/panda/qrcode/scan`.
    pub async fn panda_scan(
        &self,
        headers: &RpcHeaders,
        game_biz: &str,
        req: &PandaQrScanRequest,
    ) -> Result<QrScanData, MihoyoError> {
        let url = format!(
            "{}/{}",
            self.panda_base,
            PandaQrScanRequest::path_for(game_biz)
        );
        let rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3");
        let resp = headers
            .apply(rb)
            .json(req)
            .send()
            .await?
            .error_for_status()?
            .json::<Envelope<QrScanData>>()
            .await?;
        if resp.retcode != 0 {
            return Err(MihoyoError::Api {
                retcode: resp.retcode,
                message: resp.message,
            });
        }
        resp.data
            .ok_or_else(|| MihoyoError::InvalidResponse("missing data".into()))
    }

    /// V2: POST passport scanQRLogin with DS + Cookie.
    ///
    /// Signs the exact JSON bytes that are sent on the wire.
    pub async fn passport_scan(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        cookie: &str,
        req: &PassportQrScanRequest,
    ) -> Result<QrScanData, MihoyoError> {
        let body_str =
            serde_json::to_string(req).map_err(|e| MihoyoError::InvalidResponse(e.to_string()))?;
        let body_val: Value = serde_json::from_str(&body_str)
            .map_err(|e| MihoyoError::InvalidResponse(e.to_string()))?;
        let ds = create_sign(&body_val, salt.clone(), None, None);
        let url = format!("{}/{}", self.login_base, PassportQrScanRequest::PATH);
        let rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("Cookie", cookie.to_string());
        let resp = headers
            .apply(rb)
            .body(body_str)
            .send()
            .await?
            .error_for_status()?;
        let status = resp.status();
        let text = resp.text().await?;
        if let Ok(env) = serde_json::from_str::<Envelope<QrScanData>>(&text) {
            if env.retcode != 0 {
                return Err(MihoyoError::Api {
                    retcode: env.retcode,
                    message: format!(
                        "{} (http {status}) body={}",
                        env.message,
                        truncate_for_log(&text)
                    ),
                });
            }
            if let Some(data) = env.data {
                return Ok(data);
            }
        }
        Err(MihoyoError::InvalidResponse(format!(
            "passport scan unparsable (http {status}): {}",
            truncate_for_log(&text)
        )))
    }

    /// V2: POST confirmQRLogin with DS + Cookie.
    pub async fn passport_confirm(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        cookie: &str,
        req: &PassportQrConfirmRequest,
    ) -> Result<Value, MihoyoError> {
        let body =
            serde_json::to_value(req).map_err(|e| MihoyoError::InvalidResponse(e.to_string()))?;
        let ds = create_sign(&body, salt.clone(), None, None);
        let url = format!("{}/{}", self.login_base, PassportQrConfirmRequest::PATH);
        let rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("Cookie", cookie.to_string());
        let resp = headers
            .apply(rb)
            .json(req)
            .send()
            .await?
            .error_for_status()?;
        let status = resp.status();
        let text = resp.text().await?;
        if let Ok(env) = serde_json::from_str::<Envelope<Value>>(&text) {
            if env.retcode != 0 {
                return Err(MihoyoError::Api {
                    retcode: env.retcode,
                    message: format!(
                        "{} (http {status}) body={}",
                        env.message,
                        truncate_for_log(&text)
                    ),
                });
            }
            return Ok(env.data.unwrap_or(Value::Null));
        }
        Err(MihoyoError::InvalidResponse(format!(
            "confirm unparsable (http {status}): {}",
            truncate_for_log(&text)
        )))
    }

    /// V2: POST cancelQRLogin with DS + Cookie.
    pub async fn passport_cancel(
        &self,
        headers: &RpcHeaders,
        salt: &Salt,
        cookie: &str,
        ticket: &str,
        token_types: &[String],
    ) -> Result<(), MihoyoError> {
        let req = PassportQrScanRequest {
            ticket: ticket.to_string(),
            token_types: token_types.to_vec(),
        };
        let body_str =
            serde_json::to_string(&req).map_err(|e| MihoyoError::InvalidResponse(e.to_string()))?;
        let body_val: Value = serde_json::from_str(&body_str)
            .map_err(|e| MihoyoError::InvalidResponse(e.to_string()))?;
        let ds = create_sign(&body_val, salt.clone(), None, None);
        let url = format!(
            "{}/account/ma-cn-passport/app/cancelQRLogin",
            self.login_base
        );
        let rb = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "okhttp/4.9.3")
            .header("DS", ds)
            .header("Cookie", cookie.to_string());
        let resp = headers
            .apply(rb)
            .body(body_str)
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
        Ok(())
    }
}

fn trim_slash(mut s: String) -> String {
    while s.ends_with('/') {
        s.pop();
    }
    s
}

fn truncate_for_log(s: &str) -> String {
    let mut t: String = s.chars().take(400).collect();
    if s.chars().count() > 400 {
        t.push('…');
    }
    t
}
