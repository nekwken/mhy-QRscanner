//! Device API: create / register / show.

use super::{device_summary, parse_game_biz, profile_key, registration_key, validate_account};
use crate::dto::{DeviceSummary, RegistrationSummary};
use crate::{BridgeError, Core};
use mhy_qrscanner_core::{AccountId, DeviceProfile, DeviceRegistration};
use mhy_qrscanner_device::generator::{generate_profile_with_template, DeviceTemplate};
use mhy_qrscanner_mihoyo::device_api::HttpDeviceApi;

/// Template names accepted by [`device_create`], in display order.
pub const TEMPLATE_NAMES: &[&str] = &[
    "xiaomi14",
    "samsung-s24",
    "oppo-find-x7",
    "vivo-x100",
    "samsung-a52",
];

fn parse_template(name: &str) -> Result<DeviceTemplate, BridgeError> {
    DeviceTemplate::parse(name)
        .ok_or_else(|| BridgeError::Invalid(format!("unknown device template `{name}`")))
}

/// Load the account's device profile, creating a default one on first use.
///
/// This is what keeps the login flow label-free for end users: a passport
/// login only needs a label, and the virtual device materialises behind it.
pub(crate) fn ensure_profile(core: &Core, account: &str) -> Result<DeviceProfile, BridgeError> {
    let key = profile_key(account);
    if let Some(profile) = core.store().load::<DeviceProfile>(&key)? {
        return Ok(profile);
    }
    let account_id = AccountId::new(account)?;
    let mut rng = rand::rngs::OsRng;
    let profile = generate_profile_with_template(
        &account_id,
        mhy_qrscanner_core::GameBiz::GenshinCn,
        DeviceTemplate::parse("xiaomi14").expect("xiaomi14 is a built-in template"),
        &mut rng,
    );
    core.store().save(&key, &profile)?;
    Ok(profile)
}

/// Create a virtual device profile for an account.
///
/// `force` replaces an existing profile and drops its stale registration.
pub fn device_create(
    core: &Core,
    account: &str,
    template: &str,
    force: bool,
) -> Result<DeviceSummary, BridgeError> {
    validate_account(account)?;
    let tmpl = parse_template(template)?;
    let key = profile_key(account);
    if core.store().load::<DeviceProfile>(&key)?.is_some() && !force {
        return Err(BridgeError::Invalid(format!(
            "device profile already exists for account `{account}`; pass force to replace it"
        )));
    }

    let account_id = AccountId::new(account)?;
    let mut rng = rand::rngs::OsRng;
    let profile =
        generate_profile_with_template(&account_id, mhy_qrscanner_core::GameBiz::GenshinCn, tmpl, &mut rng);
    core.store().save(&key, &profile)?;
    if force {
        // Drop the stale registration only after the new profile is written.
        core.store().delete(&registration_key(account))?;
    }
    Ok(device_summary(account, &profile, None))
}

/// Register the stored device with Miyoushe (`getExtList` + `getFp`).
pub async fn device_register(
    core: &Core,
    account: &str,
) -> Result<RegistrationSummary, BridgeError> {
    validate_account(account)?;
    let profile = core
        .store()
        .load::<DeviceProfile>(&profile_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(account.to_string()))?;
    let api = HttpDeviceApi::new(core.device_api_host());
    let registration =
        mhy_qrscanner_device::registration::register_device(&api, core.store(), &profile).await?;
    Ok(RegistrationSummary {
        account: account.to_string(),
        device_id_masked: crate::dto::mask(&registration.device_id),
        device_fp_masked: crate::dto::mask(&registration.device_fp),
        device_fp_len: registration.device_fp.chars().count(),
    })
}

/// Read the stored device profile (masked).
pub fn device_show(core: &Core, account: &str) -> Result<DeviceSummary, BridgeError> {
    validate_account(account)?;
    let profile = core
        .store()
        .load::<DeviceProfile>(&profile_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(account.to_string()))?;
    let registration = core
        .store()
        .load::<DeviceRegistration>(&registration_key(account))?;
    Ok(device_summary(account, &profile, registration.as_ref()))
}

/// Set the profile's `game_biz` (kept separate from `device_create` so the UI can
/// switch titles without rotating the device identity).
pub fn device_set_game_biz(
    core: &Core,
    account: &str,
    game_biz: &str,
) -> Result<DeviceSummary, BridgeError> {
    validate_account(account)?;
    let biz = parse_game_biz(game_biz)?;
    let key = profile_key(account);
    let mut profile = core
        .store()
        .load::<DeviceProfile>(&key)?
        .ok_or_else(|| BridgeError::AccountNotFound(account.to_string()))?;
    profile.game_biz = biz;
    core.store().save(&key, &profile)?;
    let registration = core
        .store()
        .load::<DeviceRegistration>(&registration_key(account))?;
    Ok(device_summary(account, &profile, registration.as_ref()))
}

/// Every stored device profile, ordered by label (masked).
///
/// Enumeration walks the `accounts/` directory of the shared data dir, so
/// profiles created by the CLI are visible here too — both entry points open
/// the same store.
pub fn device_list(core: &Core) -> Result<Vec<DeviceSummary>, BridgeError> {
    let accounts_dir = core.data_dir().join("accounts");
    let mut labels: Vec<String> = match std::fs::read_dir(&accounts_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => Vec::new(),
    };
    labels.sort();
    let mut out = Vec::new();
    for label in labels {
        if validate_account(&label).is_err() {
            continue;
        }
        if let Some(profile) = core.store().load::<DeviceProfile>(&profile_key(&label))? {
            let registration = core
                .store()
                .load::<DeviceRegistration>(&registration_key(&label))?;
            out.push(device_summary(&label, &profile, registration.as_ref()));
        }
    }
    Ok(out)
}

/// Delete one account's device profile, registration, session, and approval
/// policy. The audit log entry stays — history is append-only.
pub fn device_delete(core: &Core, account: &str) -> Result<(), BridgeError> {
    validate_account(account)?;
    let existed = core
        .store()
        .load::<DeviceProfile>(&profile_key(account))?
        .is_some();
    if !existed {
        return Err(BridgeError::AccountNotFound(account.to_string()));
    }
    for key in [
        profile_key(account),
        registration_key(account),
        super::session_key(account),
        crate::api::settings::policy_key(account),
    ] {
        core.store().delete(&key)?;
    }
    super::settings::record(
        core,
        account,
        "device_delete",
        true,
        "profile removed",
        None,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_core() -> (tempfile::TempDir, Core) {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        (dir, core)
    }

    #[test]
    fn create_show_and_replace() {
        let (_dir, core) = temp_core();
        let created = device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        assert_eq!(created.model, "Xiaomi 14");
        assert!(!created.registered);
        assert_eq!(created.device_id_len, 16);

        // refuses to replace without force
        assert!(device_create(&core, "acct-1", "xiaomi14", false).is_err());

        let shown = device_show(&core, "acct-1").unwrap();
        assert_eq!(shown.device_id_masked, created.device_id_masked);

        let replaced = device_create(&core, "acct-1", "samsung-s24", true).unwrap();
        assert_eq!(replaced.model, "SM-S9280");
        assert_ne!(replaced.device_id_masked, created.device_id_masked);
    }

    #[test]
    fn create_rejects_unknown_template_and_bad_account() {
        let (_dir, core) = temp_core();
        assert!(device_create(&core, "acct-1", "nope", false).is_err());
        assert!(device_create(&core, "", "xiaomi14", false).is_err());
    }

    #[test]
    fn show_missing_account_reports_not_found() {
        let (_dir, core) = temp_core();
        let err = device_show(&core, "nobody").unwrap_err();
        assert!(matches!(err, BridgeError::AccountNotFound(_)));
    }

    #[test]
    fn set_game_biz_updates_profile() {
        let (_dir, core) = temp_core();
        device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        let updated = device_set_game_biz(&core, "acct-1", "hkrpg_cn").unwrap();
        assert_eq!(updated.game_biz, "hkrpg_cn");
        assert!(device_set_game_biz(&core, "acct-1", "nope").is_err());
    }

    #[test]
    fn all_templates_parse() {
        for name in TEMPLATE_NAMES {
            assert!(parse_template(name).is_ok(), "template {name} must parse");
        }
    }

    #[test]
    fn list_shows_every_profile_ordered_by_label() {
        let (_dir, core) = temp_core();
        assert!(device_list(&core).unwrap().is_empty());
        device_create(&core, "b-acct", "xiaomi14", false).unwrap();
        device_create(&core, "a-acct", "samsung-a52", false).unwrap();
        let listed = device_list(&core).unwrap();
        let labels: Vec<&str> = listed.iter().map(|d| d.account.as_str()).collect();
        assert_eq!(labels, ["a-acct", "b-acct"]);
        assert!(listed.iter().all(|d| !d.registered));
    }

    #[test]
    fn delete_removes_profile_and_allows_the_label_again() {
        let (_dir, core) = temp_core();
        device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        // deleting a missing account reports not found
        assert!(matches!(
            device_delete(&core, "nobody").unwrap_err(),
            BridgeError::AccountNotFound(_)
        ));
        device_delete(&core, "acct-1").unwrap();
        assert!(device_list(&core).unwrap().is_empty());
        assert!(matches!(
            device_show(&core, "acct-1").unwrap_err(),
            BridgeError::AccountNotFound(_)
        ));
        // the label is immediately reusable without --force
        device_create(&core, "acct-1", "samsung-a52", false).unwrap();
        // the deletion left an audit entry
        let audit = crate::api::settings::audit_recent(&core, 10).unwrap();
        assert!(audit.iter().any(|e| e.action == "device_delete"));
    }
}
