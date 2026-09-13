//! Bridge API surface, grouped by area.

pub mod auth;
pub mod core;
pub mod device;
pub mod qr;
pub mod race;
pub mod settings;

use crate::dto::mask;
use crate::BridgeError;
use mhy_qrscanner_core::{AccountSession, DeviceProfile, DeviceRegistration, GameBiz};

/// Unix seconds as a string, the timestamp format the store already uses.
pub(crate) fn now_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

pub(crate) fn profile_key(account: &str) -> String {
    format!("accounts/{account}/device-profile")
}

pub(crate) fn registration_key(account: &str) -> String {
    format!("accounts/{account}/device-registration")
}

pub(crate) fn session_key(account: &str) -> String {
    format!("accounts/{account}/session")
}

/// Reject empty or path-hostile account labels before they reach the store.
pub(crate) fn validate_account(account: &str) -> Result<(), BridgeError> {
    let trimmed = account.trim();
    if trimmed.is_empty() {
        return Err(BridgeError::Invalid("account label is empty".into()));
    }
    if trimmed.contains(['/', '\\']) || trimmed.contains("..") {
        return Err(BridgeError::Invalid(
            "account label must not contain path separators".into(),
        ));
    }
    Ok(())
}

pub(crate) fn parse_game_biz(value: &str) -> Result<GameBiz, BridgeError> {
    GameBiz::from_biz(value)
        .ok_or_else(|| BridgeError::Invalid(format!("unsupported game_biz `{value}`")))
}

pub(crate) fn mask_len(value: &str) -> (String, usize) {
    (mask(value), value.chars().count())
}

/// Masked device view shared by `device_create` / `device_show`.
pub(crate) fn device_summary(
    account: &str,
    profile: &DeviceProfile,
    registration: Option<&DeviceRegistration>,
) -> crate::DeviceSummary {
    let fp = registration
        .map(|r| r.device_fp.as_str())
        .unwrap_or(profile.initial_device_fp.as_str());
    let (id_masked, id_len) = mask_len(&profile.device_id);
    let (fp_masked, fp_len) = mask_len(fp);
    crate::DeviceSummary {
        account: account.to_string(),
        model: profile.model.clone(),
        brand: profile.brand.clone(),
        android_version: profile.android_version.clone(),
        sdk_version: profile.sdk_version.clone(),
        game_biz: profile.game_biz.as_str().to_string(),
        device_id_masked: id_masked,
        device_id_len: id_len,
        device_fp_masked: fp_masked,
        device_fp_len: fp_len,
        registered: registration.is_some(),
    }
}

/// Masked session view.
pub(crate) fn session_summary(
    account: &str,
    session: &AccountSession,
    steps: Vec<crate::dto::StepReport>,
) -> crate::SessionSummary {
    let (mid_masked, _) = mask_len(&session.mid);
    let (qr_mid_masked, _) = mask_len(session.qr_mid());
    crate::SessionSummary {
        account: account.to_string(),
        account_uid_len: session.account_uid.chars().count(),
        stoken_len: session.stoken.chars().count(),
        stoken_prefix: session.stoken.chars().take(3).collect(),
        mid_masked,
        qr_mid_masked,
        cookie_token_len: session.cookie_token.chars().count(),
        ltoken_len: session.ltoken.chars().count(),
        updated_at: session.updated_at.clone(),
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_account_rejects_bad_labels() {
        assert!(validate_account("").is_err());
        assert!(validate_account("   ").is_err());
        assert!(validate_account("a/b").is_err());
        assert!(validate_account("a\\b").is_err());
        assert!(validate_account("..").is_err());
        assert!(validate_account("acct-1").is_ok());
        assert!(validate_account("两号").is_ok());
    }

    #[test]
    fn parse_game_biz_accepts_known_and_rejects_unknown() {
        assert_eq!(parse_game_biz("hk4e_cn").unwrap(), GameBiz::GenshinCn);
        assert_eq!(parse_game_biz("hkrpg_cn").unwrap(), GameBiz::StarRailCn);
        assert!(parse_game_biz("nope").is_err());
    }

    #[test]
    fn keys_are_stable() {
        assert_eq!(profile_key("a"), "accounts/a/device-profile");
        assert_eq!(registration_key("a"), "accounts/a/device-registration");
        assert_eq!(session_key("a"), "accounts/a/session");
    }
}
