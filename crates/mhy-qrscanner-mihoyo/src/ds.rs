//! Miyoushe passport DS2 signing (Porte `RequestUtils.createSign`).
//!
//! Salt values are client-extracted constants. Prefer injecting a custom
//! salt in tests; production values live in this module only.

use md5::{Digest, Md5};
use rand::Rng;
use serde_json::Value;

const SALT_PROD: &str = "JwYDpKvLj6MrMqqYU6jTKF17KNO2PXoS";
const SALT_DEV: &str = "IZPgfb0dRPtBeLuFkdDznSZ6f4wWt6y2";
const RANDOM_RANGE: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsEnv {
    Product,
    Dev,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Salt {
    Prod,
    Dev,
    Custom(String),
}

impl Salt {
    pub fn from_env(env: DsEnv) -> Self {
        match env {
            DsEnv::Product => Self::Prod,
            DsEnv::Dev => Self::Dev,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Prod => SALT_PROD,
            Self::Dev => SALT_DEV,
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Build the DS header value: `t,r,md5(salt=...&t=...&r=...&b=...&q=)`.
///
/// `t` and `r` may be fixed for deterministic tests; pass `None` to generate.
pub fn create_sign(params: &Value, salt: Salt, t: Option<u64>, r: Option<&str>) -> String {
    let t = t.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });
    let r_owned;
    let r: &str = match r {
        Some(s) => s,
        None => {
            let mut rng = rand::thread_rng();
            r_owned = (0..6)
                .map(|_| RANDOM_RANGE[rng.gen_range(0..RANDOM_RANGE.len())] as char)
                .collect::<String>();
            &r_owned
        }
    };
    let b = params.to_string();
    let raw = format!("salt={}&t={}&r={}&b={}&q=", salt.as_str(), t, r, b);
    let mut hasher = Md5::new();
    hasher.update(raw.as_bytes());
    let sign = format!("{:x}", hasher.finalize());
    format!("{t},{r},{sign}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn salt_env_mapping() {
        assert_eq!(Salt::from_env(DsEnv::Product), Salt::Prod);
        assert_eq!(Salt::from_env(DsEnv::Dev), Salt::Dev);
    }

    #[test]
    fn empty_params_still_signs() {
        let ds = create_sign(
            &json!({}),
            Salt::Custom("x".to_string()),
            Some(1),
            Some("rrrrrr"),
        );
        let parts: Vec<_> = ds.split(',').collect();
        assert_eq!(parts[0], "1");
        assert_eq!(parts[1], "rrrrrr");
        assert_eq!(parts[2].len(), 32);
    }
}
