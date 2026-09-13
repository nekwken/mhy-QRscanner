use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AccountId(String);

impl AccountId {
    /// Validated constructor: the id becomes part of the store path
    /// (`accounts/<id>/...`), so it must be non-empty after trimming and free
    /// of path separators (`/`, `\`), `..`, and `:` (Windows drive syntax).
    /// This is the single source of the rule; the bridge's `validate_account`
    /// and `EncryptedStore::path_for` remain downstream guards.
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let trimmed = value.into();
        let trimmed = trimmed.trim();
        if trimmed.is_empty()
            || trimmed.contains(['/', '\\'])
            || trimmed.contains("..")
            || trimmed.contains(':')
        {
            return Err(CoreError::InvalidAccountId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameBiz {
    #[serde(rename = "hk4e_cn")]
    GenshinCn,
    #[serde(rename = "hk4e_global")]
    GenshinGlobal,
    #[serde(rename = "bh3_cn")]
    HonkaiImpact3Cn,
    #[serde(rename = "hkrpg_cn")]
    StarRailCn,
    #[serde(rename = "nap_cn")]
    ZenlessCn,
}

impl GameBiz {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GenshinCn => "hk4e_cn",
            Self::GenshinGlobal => "hk4e_global",
            Self::HonkaiImpact3Cn => "bh3_cn",
            Self::StarRailCn => "hkrpg_cn",
            Self::ZenlessCn => "nap_cn",
        }
    }

    /// Numeric game id inside the game QR URL (`app_id=4` for Genshin).
    ///
    /// Named `game_id`, not `app_id`, so it cannot be confused with
    /// [`ClientProfile::app_id`] — the passport App id, which is a string.
    pub const fn game_id(self) -> u32 {
        match self {
            Self::GenshinCn | Self::GenshinGlobal => 4,
            Self::HonkaiImpact3Cn => 1,
            Self::StarRailCn => 8,
            Self::ZenlessCn => 12,
        }
    }

    /// Parse a `game_biz` string (e.g. `hkrpg_cn`).
    pub fn from_biz(s: &str) -> Option<Self> {
        match s {
            "hk4e_cn" => Some(Self::GenshinCn),
            "hk4e_global" => Some(Self::GenshinGlobal),
            "bh3_cn" => Some(Self::HonkaiImpact3Cn),
            "hkrpg_cn" => Some(Self::StarRailCn),
            "nap_cn" => Some(Self::ZenlessCn),
            _ => None,
        }
    }

    /// All CN official games.
    pub const CN_ALL: [Self; 4] = [
        Self::GenshinCn,
        Self::HonkaiImpact3Cn,
        Self::StarRailCn,
        Self::ZenlessCn,
    ];
}

/// Default is Genshin CN, the only title with verified protocol behaviour.
impl Default for GameBiz {
    fn default() -> Self {
        Self::GenshinCn
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientProfile {
    pub app_version: String,
    pub sdk_version: String,
    pub app_id: String,
    pub client_type: String,
    pub game_biz: String,
}

/// Missing client fields fall back to the verified Miyoushe 2.113.1 identity
/// rather than to empty strings.
impl Default for ClientProfile {
    fn default() -> Self {
        Self::miyoushe_2_113_1()
    }
}

impl ClientProfile {
    /// Miyoushe Android 2.113.1 (verified against official-client traffic, 2026-09-11).
    /// `app_id` is the passport App id, **not** the cloud-game `c76ync6mutq8`.
    pub fn miyoushe_2_113_1() -> Self {
        Self {
            app_version: "2.113.1".to_string(),
            sdk_version: "2.42.0".to_string(),
            app_id: "bll8iq97cem8".to_string(),
            client_type: "2".to_string(),
            game_biz: "bbs_cn".to_string(),
        }
    }
}

/// One stable virtual Android device per account.
///
/// `#[serde(default)]` at the struct level means a stored profile still loads
/// after a field is added (missing fields fall back to `Default::default()`).
/// Semantic changes still require a `profile_version` bump plus a migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceProfile {
    pub profile_version: u32,
    pub account_id: AccountId,
    pub game_biz: GameBiz,
    pub client_profile: ClientProfile,
    pub device_id: String,
    pub seed_id: String,
    pub seed_time: String,
    pub initial_device_fp: String,
    pub bbs_device_id: String,
    pub model: String,
    pub brand: String,
    pub manufacturer: String,
    pub product_name: String,
    pub device_name: String,
    pub android_version: String,
    pub sdk_version: String,
    pub abi: String,
    pub screen_size: String,
    pub display: String,
    pub ram_capacity: String,
    pub rom_capacity: String,
    pub network_type: String,
    pub ext_fields: BTreeMap<String, String>,
}

/// Registration result for a device profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceRegistration {
    pub device_id: String,
    pub device_fp: String,
    pub seed_id: String,
    pub seed_time: String,
    pub bbs_device_id: String,
    pub registered_at: String,
}

/// Miyoushe passport session (stoken family). Never persists passwords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSession {
    pub account_uid: String,
    pub stoken: String,
    /// Passport account mid from login (`user_info.mid`).
    pub mid: String,
    /// Device/BBS mid used in Cookie for scanQRLogin (e.g. `0cdapswfd1_mhy`).
    #[serde(default = "default_cookie_mid")]
    pub cookie_mid: String,
    /// optional v2 stoken
    #[serde(default)]
    pub stoken_v2: String,
    #[serde(default)]
    pub ltoken: String,
    #[serde(default)]
    pub cookie_token: String,
    pub updated_at: String,
}

fn default_cookie_mid() -> String {
    "0cdapswfd1_mhy".to_string()
}

impl AccountSession {
    /// Cookie used by Porte passport QR: `stoken=...;mid=<passport mid>`.
    ///
    /// `user_info.mid` is **per-account** (e.g. `0cdapswfd1_mhy` vs `03pmu45q7y_mhy`),
    /// not a global constant. Fall back to cookie_mid only if mid is empty.
    pub fn cookie_pair(&self) -> String {
        let mid = if !self.mid.is_empty() {
            self.mid.as_str()
        } else if !self.cookie_mid.is_empty() {
            self.cookie_mid.as_str()
        } else {
            "0cdapswfd1_mhy"
        };
        format!("stoken={};mid={}", self.stoken, mid)
    }

    /// Mid to put in Cookie for QR (passport mid preferred).
    pub fn qr_mid(&self) -> &str {
        if !self.mid.is_empty() {
            &self.mid
        } else if !self.cookie_mid.is_empty() {
            &self.cookie_mid
        } else {
            "0cdapswfd1_mhy"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genshin_cn_maps_to_game_id_four() {
        assert_eq!(GameBiz::GenshinCn.game_id(), 4);
        assert_eq!(GameBiz::GenshinCn.as_str(), "hk4e_cn");
    }

    #[test]
    fn device_profile_loads_when_fields_are_missing() {
        // Simulates a profile written by an older build: only a couple of fields
        // present. Forward compatibility requires the rest to default.
        let json = serde_json::json!({
            "profile_version": 1,
            "account_id": "acct-1",
            "game_biz": "hk4e_cn",
            "device_id": "0123456789abcdef"
        });
        let profile: DeviceProfile = serde_json::from_value(json).expect("must load");
        assert_eq!(profile.device_id, "0123456789abcdef");
        assert_eq!(profile.model, "");
        assert!(profile.ext_fields.is_empty());
        // game_biz default is Genshin CN, the verified title
        assert_eq!(GameBiz::default(), GameBiz::GenshinCn);
    }

    #[test]
    fn device_registration_loads_when_fields_are_missing() {
        let json = serde_json::json!({ "device_id": "abc" });
        let reg: DeviceRegistration = serde_json::from_value(json).expect("must load");
        assert_eq!(reg.device_id, "abc");
        assert_eq!(reg.device_fp, "");
    }

    #[test]
    fn client_profile_default_is_miyoushe_2_113_1() {
        assert_eq!(ClientProfile::default(), ClientProfile::miyoushe_2_113_1());
    }

    #[test]
    fn game_biz_round_trip_and_ids() {
        for g in GameBiz::CN_ALL {
            assert_eq!(GameBiz::from_biz(g.as_str()), Some(g));
        }
        assert_eq!(GameBiz::StarRailCn.game_id(), 8);
        assert_eq!(GameBiz::ZenlessCn.game_id(), 12);
        assert_eq!(GameBiz::HonkaiImpact3Cn.game_id(), 1);
        assert_eq!(GameBiz::from_biz("unknown"), None);
    }

    #[test]
    fn account_id_round_trips_through_json() {
        let id = AccountId::new("acct-1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: AccountId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn account_id_rejects_path_hostile_values() {
        assert!(matches!(
            AccountId::new(""),
            Err(CoreError::InvalidAccountId)
        ));
        assert!(matches!(
            AccountId::new("   "),
            Err(CoreError::InvalidAccountId)
        ));
        assert!(matches!(
            AccountId::new("a/b"),
            Err(CoreError::InvalidAccountId)
        ));
        assert!(matches!(
            AccountId::new("a\\b"),
            Err(CoreError::InvalidAccountId)
        ));
        assert!(matches!(
            AccountId::new(".."),
            Err(CoreError::InvalidAccountId)
        ));
        assert!(matches!(
            AccountId::new("c:evil"),
            Err(CoreError::InvalidAccountId)
        ));
        // a value with surrounding whitespace is trimmed, not rejected
        assert_eq!(AccountId::new("  acct-1  ").unwrap().as_str(), "acct-1");
        // non-ASCII labels are fine
        assert!(AccountId::new("两号").is_ok());
    }

    #[test]
    fn account_session_cookie_pair() {
        let s = AccountSession {
            account_uid: "1".into(),
            stoken: "st".into(),
            mid: "03pmu45q7y_mhy".into(),
            cookie_mid: "0cdapswfd1_mhy".into(),
            stoken_v2: String::new(),
            ltoken: String::new(),
            cookie_token: String::new(),
            updated_at: "t".into(),
        };
        // passport mid wins
        assert_eq!(s.cookie_pair(), "stoken=st;mid=03pmu45q7y_mhy");
        assert_eq!(s.qr_mid(), "03pmu45q7y_mhy");
    }
}
