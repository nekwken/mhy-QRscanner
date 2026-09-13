//! Request/response types for QR scan/confirm.
//!
//! Field shapes confirmed against official-client traffic 2026-09-11
//! (Miyoushe Android scanning PC Genshin client QR).

use serde::{Deserialize, Serialize};

/// POST `{pandaBase}/{game_biz}/combo/panda/qrcode/scan`
///
/// `app_id` is a **number** (4 for Genshin CN).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PandaQrScanRequest {
    pub passport_app_id: String,
    pub ticket: String,
    pub app_id: i64,
    /// PorteInfo.deviceID (16-hex).
    pub device: String,
    /// unix seconds
    pub ts: u64,
}

impl PandaQrScanRequest {
    pub const PATH: &'static str = "combo/panda/qrcode/scan";

    /// Full path including game biz, e.g. `hk4e_cn/combo/panda/qrcode/scan`.
    pub fn path_for(game_biz: &str) -> String {
        format!("{game_biz}/{}", Self::PATH)
    }
}

/// POST `{loginBase}account/ma-cn-passport/app/scanQRLogin`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassportQrScanRequest {
    pub ticket: String,
    /// App uses `uri.getQueryParameters("token_types")` → List<String>.
    pub token_types: Vec<String>,
}

impl PassportQrScanRequest {
    pub const PATH: &'static str = "account/ma-cn-passport/app/scanQRLogin";
}

/// POST `{loginBase}account/ma-cn-passport/app/confirmQRLogin`
///
/// Field name still under verification. We send
/// `confirm: true` explicitly so the server treats this as approval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassportQrConfirmRequest {
    pub ticket: String,
    pub token_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm: Option<bool>,
}

impl PassportQrConfirmRequest {
    pub const PATH: &'static str = "account/ma-cn-passport/app/confirmQRLogin";

    pub fn approve(ticket: String, token_types: Vec<String>) -> Self {
        Self {
            ticket,
            token_types,
            confirm: Some(true),
        }
    }
}

/// Passport scan response (`ScanQRLoginEntity`).
///
/// Example data:
/// `{"app_id":"c76ync6mutq8","app_name":"原神","client_type":3,...}`
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct QrScanData {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_name: String,
    /// Server may send client_type as number.
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub client_type: Option<i64>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub account_disp_name: String,
    #[serde(default)]
    pub app_icon: String,
    #[serde(default)]
    pub risk_note: String,
    /// panda scan may return passport_qr_url
    #[serde(default)]
    pub passport_qr_url: String,
}

fn de_opt_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match v {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        Some(_) => None,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    pub retcode: i64,
    #[serde(default)]
    pub message: String,
    pub data: Option<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panda_request_serializes_app_id_as_number() {
        let req = PandaQrScanRequest {
            passport_app_id: "bll8iq97cem8".into(),
            ticket: "tk".into(),
            app_id: 4,
            device: "0123456789abcdef".into(),
            ts: 1720000000,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["app_id"], 4);
        assert_eq!(v["passport_app_id"], "bll8iq97cem8");
        assert_eq!(
            PandaQrScanRequest::path_for("hk4e_cn"),
            "hk4e_cn/combo/panda/qrcode/scan"
        );
    }

    #[test]
    fn scan_data_parses_real_capture() {
        let raw = serde_json::json!({
            "app_id": "c76ync6mutq8",
            "app_name": "原神",
            "client_type": 3,
            "created_at": "1789058320",
            "account_disp_name": "195******50",
            "app_icon": "https://example/x.png",
            "risk_note": ""
        });
        let d: QrScanData = serde_json::from_value(raw).unwrap();
        assert_eq!(d.app_name, "原神");
        assert_eq!(d.client_type, Some(3));
    }
}
