// BridgeError 携带挑战负载，体积越过 clippy 的 result_large_err 阈值；
// 对本桥接层而言堆装箱每个构造点纯属折腾，这里显式豁免。
#![allow(clippy::result_large_err)]

//! Flutter-facing bridge over the Rust core.
//!
//! Boundary rules (design §4.1):
//!
//! - Dart never builds signed requests and never holds a replayable secret.
//! - This crate is a thin adapter: it validates, masks, and forwards. All
//!   protocol logic stays in `mhy-qrscanner-mihoyo` / `mhy-qrscanner-qr` / `mhy-qrscanner-device`.
//! - Nothing here returns a full `stoken`, `device_fp`, or SMS code to the UI.
//!
//! The API is plain Rust with serde DTOs so it can be unit tested without a
//! Flutter toolchain; `flutter_rust_bridge` generates the Dart bindings from
//! `crate::api`.

// Injected by flutter_rust_bridge_codegen; must come after the module docs.
mod frb_generated;

mod api;
mod dto;
mod error;
mod events;

pub use api::auth::{
    auth_login_password, auth_logout, auth_request_sms, auth_show, auth_submit_sms, session_probe,
};
pub use api::core::{core_new, core_new_at, default_data_dir, Core};
pub use api::device::{
    device_create, device_delete, device_list, device_register, device_set_game_biz, device_show,
    TEMPLATE_NAMES,
};
pub use api::qr::{qr_cancel, qr_confirm_pending, qr_login_game, qr_scan, scan_stop, QrSource};
pub use api::race::{qr_race_start, qr_race_status, qr_race_stop};
pub use api::settings::{
    audit_recent, flag_get, flag_set, text_get, text_set, FLAG_CLOSE_TO_TRAY, FLAG_DEV_MODE,
    FLAG_KEEP_SESSION, TEXT_LAST_ACCOUNT,
};
pub use dto::{
    ApprovalOutcome, AuditEntryDto, BridgeInfo, DeviceSummary, LoginChallengeDto, RaceStatusDto,
    RegistrationSummary, SessionSummary, SmsRequestDto, StepReport,
};
pub use error::BridgeError;
