//! Shared x-rpc-* request headers (Porte `RequestUtils.getHeader`).

/// Common x-rpc headers required by passport APIs.
#[derive(Debug, Clone)]
pub struct RpcHeaders {
    pub app_id: String,
    pub client_type: String,
    pub device_id: String,
    pub device_fp: String,
    pub device_name: String,
    pub device_model: String,
    pub sys_version: String,
    pub game_biz: String,
    pub app_version: String,
    pub sdk_version: String,
    pub lifecycle_id: String,
    pub account_version: String,
}

impl RpcHeaders {
    /// Miyoushe Android 2.113.1 (`app_id=bll8iq97cem8`, `client_type=2`).
    /// Confirmed against official-client traffic 2026-09-11.
    pub fn miyoushe_defaults(device_id: impl Into<String>, device_fp: impl Into<String>) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let hex: String = (0..32)
            .map(|_| format!("{:x}", rng.gen_range(0..16)))
            .collect();
        let lifecycle_id = format!(
            "{}-{}-4{}-a{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[13..16],
            &hex[17..20],
            &hex[20..32]
        );
        Self {
            app_id: "bll8iq97cem8".into(),
            client_type: "2".into(),
            device_id: device_id.into(),
            device_fp: device_fp.into(),
            device_name: "Xiaomi 14".into(),
            device_model: "23127PN0CC".into(),
            sys_version: "14".into(),
            game_biz: "bbs_cn".into(),
            app_version: "2.113.1".into(),
            sdk_version: "2.42.0".into(),
            lifecycle_id,
            account_version: "2.42.0".into(),
        }
    }

    /// Cloud Genshin login-platform (`app_id=c76ync6mutq8`, `client_type=22`).
    pub fn cloud_genshin(device_id: impl Into<String>, device_fp: impl Into<String>) -> Self {
        Self {
            app_id: "c76ync6mutq8".into(),
            client_type: "22".into(),
            game_biz: "hk4e_cn".into(),
            ..Self::miyoushe_defaults(device_id, device_fp)
        }
    }

    pub fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("x-rpc-app_id", form_urlencode(&self.app_id))
            .header("x-rpc-client_type", &self.client_type)
            .header("x-rpc-device_id", &self.device_id)
            .header("x-rpc-device_fp", form_urlencode(&self.device_fp))
            .header("x-rpc-device_name", form_urlencode(&self.device_name))
            .header("x-rpc-device_model", form_urlencode(&self.device_model))
            .header("x-rpc-sys_version", form_urlencode(&self.sys_version))
            .header("x-rpc-game_biz", form_urlencode(&self.game_biz))
            .header("x-rpc-app_version", form_urlencode(&self.app_version))
            .header("x-rpc-sdk_version", &self.sdk_version)
            .header("x-rpc-lifecycle_id", &self.lifecycle_id)
            .header("x-rpc-account_version", &self.account_version)
    }
}

fn form_urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
