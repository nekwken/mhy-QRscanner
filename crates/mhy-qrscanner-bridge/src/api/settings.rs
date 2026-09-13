//! Approval policy and the append-only audit log.
//!
//! The policy is what the **UI** reads to decide whether to ask the user before
//! approving; the bridge never approves anything on its own — `qr_login_game`
//! confirms only when the caller passes `confirm = true`. An absent policy means
//! "ask every time", so the safe behaviour is the default.
//!
//! The audit log is written by the bridge, not the UI, so a record exists even
//! if the window closes mid-flight. It is plain NDJSON under the data directory;
//! entries carry labels and server codes, never tokens, tickets, or codes.

use crate::dto::{AuditEntryDto, FlagDto, TextDto};
use crate::{BridgeError, Core};
use std::io::Write;

const AUDIT_FILE: &str = "audit.log";
/// Hard cap per `audit_recent` call.
const AUDIT_MAX_LIMIT: u32 = 500;
/// Append-only, but not unbounded: past this size the oldest half is dropped.
/// Generous on purpose — an audit line is ~150 bytes, so 1 MiB is months of use.
const AUDIT_MAX_BYTES: u64 = 1024 * 1024;

/// 遗留存储键：自动确认策略已由「每房间模式 + 弹窗开关」取代，但删除账号
/// 时仍要清掉旧数据。
pub(crate) fn policy_key(account: &str) -> String {
    format!("settings/approval/{account}")
}

/// Well-known boolean settings keys (FlagDto store keys).
pub const FLAG_DEV_MODE: &str = "settings/dev_mode";
pub const FLAG_CLOSE_TO_TRAY: &str = "settings/close_to_tray";
pub const FLAG_KEEP_SESSION: &str = "settings/keep_session";
pub const TEXT_LAST_ACCOUNT: &str = "settings/last_account";

/// Read one persisted boolean flag. A missing record means `false` — the safe
/// default for every flag we ship.
pub fn flag_get(core: &Core, key: &str) -> Result<FlagDto, BridgeError> {
    validate_flag_key(key)?;
    Ok(core.store().load::<FlagDto>(key)?.unwrap_or(FlagDto {
        enabled: false,
        updated_at: String::new(),
    }))
}

pub fn flag_set(core: &Core, key: &str, enabled: bool) -> Result<FlagDto, BridgeError> {
    validate_flag_key(key)?;
    let flag = FlagDto {
        enabled,
        updated_at: super::now_secs(),
    };
    core.store().save(key, &flag)?;
    record(
        core,
        "-",
        "flag_set",
        true,
        &format!("{key} -> {enabled}"),
        None,
    );
    Ok(flag)
}

fn validate_flag_key(key: &str) -> Result<(), BridgeError> {
    if key.is_empty() || !key.starts_with("settings/") || key.contains("..") {
        return Err(BridgeError::Invalid(format!(
            "flag key must be a settings/ path: {key}"
        )));
    }
    Ok(())
}

/// Read one persisted string setting (empty when unset).
pub fn text_get(core: &Core, key: &str) -> Result<TextDto, BridgeError> {
    validate_flag_key(key)?;
    Ok(core.store().load::<TextDto>(key)?.unwrap_or_default())
}

pub fn text_set(core: &Core, key: &str, value: &str) -> Result<TextDto, BridgeError> {
    validate_flag_key(key)?;
    let text = TextDto {
        value: value.to_string(),
    };
    core.store().save(key, &text)?;
    Ok(text)
}

/// Newest audit entries first, at most `limit` of them.
pub fn audit_recent(core: &Core, limit: u32) -> Result<Vec<AuditEntryDto>, BridgeError> {
    let limit = limit.clamp(1, AUDIT_MAX_LIMIT) as usize;
    let path = core.data_dir().join(AUDIT_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| BridgeError::Storage(e.to_string()))?;
    let mut out = Vec::new();
    for line in text.lines().rev() {
        if out.len() >= limit {
            break;
        }
        // A torn last line (a crash mid-write) is skipped rather than fatal.
        if let Ok(entry) = serde_json::from_str::<AuditEntryDto>(line) {
            out.push(entry);
        }
    }
    Ok(out)
}

/// Append one audit line.
///
/// Deliberately total: an unwritable log must not block an approval the user
/// already decided on. Failures go to stderr.
pub(crate) fn record(
    core: &Core,
    account: &str,
    action: &str,
    ok: bool,
    detail: &str,
    retcode: Option<i64>,
) {
    let entry = AuditEntryDto {
        at: super::now_secs(),
        account: account.to_string(),
        action: action.to_string(),
        ok,
        detail: detail.to_string(),
        retcode,
    };
    let Ok(line) = serde_json::to_string(&entry) else {
        return;
    };
    let path = core.data_dir().join(AUDIT_FILE);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    rotate_if_oversized(&path);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{line}");
        }
        Err(e) => eprintln!("audit log not writable ({}): {e}", path.display()),
    }
}

/// Keep `audit.log` bounded: past [`AUDIT_MAX_BYTES`], rewrite the file with
/// only the newest half of the lines. Rotation failure is deliberately silent —
/// an audit rotation must never block the action being recorded.
fn rotate_if_oversized(path: &std::path::Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.len() <= AUDIT_MAX_BYTES {
        return;
    }
    let do_rotate = || -> std::io::Result<()> {
        let text = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let keep_from = lines.len() - lines.len() / 2;
        // The trailing newline matters: the next append must not glue onto the
        // last kept line and tear both entries.
        let mut kept = lines[keep_from..].join("\n");
        kept.push('\n');
        let tmp = path.with_extension("log.tmp");
        std::fs::write(&tmp, kept)?;
        std::fs::rename(&tmp, path)
    };
    if let Err(e) = do_rotate() {
        eprintln!("audit log rotation failed ({}): {e}", path.display());
    }
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
    fn audit_is_newest_first_and_bounded() {
        let (_dir, core) = temp_core();
        assert!(audit_recent(&core, 10).unwrap().is_empty());

        for i in 0..5 {
            record(
                &core,
                "acct-1",
                "qr_login_game",
                true,
                &format!("run {i}"),
                None,
            );
        }
        let recent = audit_recent(&core, 3).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].detail, "run 4");
        assert_eq!(recent[2].detail, "run 2");
        assert_eq!(recent[0].action, "qr_login_game");
        assert!(recent[0].ok);
    }

    #[test]
    fn audit_records_failures_with_the_server_code() {
        let (_dir, core) = temp_core();
        record(&core, "acct-1", "qr_login_game", false, "risk", Some(-3235));
        let entry = &audit_recent(&core, 1).unwrap()[0];
        assert!(!entry.ok);
        assert_eq!(entry.retcode, Some(-3235));
    }

    #[test]
    fn audit_limit_is_clamped() {
        let (_dir, core) = temp_core();
        record(&core, "acct-1", "qr_login_game", true, "one", None);
        // 0 is raised to 1 rather than returning nothing
        assert_eq!(audit_recent(&core, 0).unwrap().len(), 1);
    }

    #[test]
    fn audit_rotates_away_the_oldest_half_when_oversized() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        let path = core.data_dir().join(AUDIT_FILE);

        // Write 20000 normal-size lines (~2 MB) directly, so the next append
        // must rotate. This mirrors reality: every entry is ~150 bytes.
        let mut body = String::new();
        for i in 0..20_000 {
            let entry = AuditEntryDto {
                at: i.to_string(),
                account: "acct-1".into(),
                action: "qr_login_game".into(),
                ok: true,
                detail: format!("line {i}"),
                retcode: None,
            };
            body.push_str(&serde_json::to_string(&entry).unwrap());
            body.push('\n');
        }
        std::fs::write(&path, &body).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > AUDIT_MAX_BYTES);
        drop(body);

        // The next append must rotate first.
        record(&core, "acct-1", "qr_login_game", true, "after rotate", None);

        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size <= AUDIT_MAX_BYTES, "file still oversized: {size}");
        let recent = audit_recent(&core, AUDIT_MAX_LIMIT).unwrap();
        // The newest entry is the one written after rotation; the oldest half
        // (early lines) is gone.
        assert_eq!(recent[0].detail, "after rotate");
        assert!(!recent.iter().any(|e| e.detail == "line 0"));
        assert!(recent.iter().any(|e| e.detail == "line 19999"));
        // No temp file left behind.
        assert!(!path.with_extension("log.tmp").exists());
    }

    #[test]
    fn audit_rotation_is_skipped_while_under_the_cap() {
        let (_dir, core) = temp_core();
        for i in 0..5 {
            record(
                &core,
                "acct-1",
                "qr_login_game",
                true,
                &format!("run {i}"),
                None,
            );
        }
        assert_eq!(audit_recent(&core, AUDIT_MAX_LIMIT).unwrap().len(), 5);
    }
}
