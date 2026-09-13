//! Serializable DTOs returned to Dart.
//!
//! Masking rule: identity values (device id, fingerprint, tokens) are never
//! returned in full. The UI gets a short preview plus the length so it can show
//! "identity present" without holding a replayable value.

use mhy_qrscanner_mihoyo::auth::LoginChallengeKind;
use serde::{Deserialize, Serialize};

/// Mask a secret: keep a short head and tail, never the middle.
pub fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "***".to_string();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}...{tail}")
}

/// Static description of the core, shown on a settings/about screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeInfo {
    pub core_version: String,
    pub data_dir: String,
    pub passport_host: String,
    pub device_api_host: String,
    /// Supported `game_biz` values.
    pub games: Vec<String>,
    /// Accepted `--template` names.
    pub device_templates: Vec<String>,
}

/// Masked view of a stored virtual device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub account: String,
    pub model: String,
    pub brand: String,
    pub android_version: String,
    pub sdk_version: String,
    pub game_biz: String,
    pub device_id_masked: String,
    pub device_id_len: usize,
    pub device_fp_masked: String,
    pub device_fp_len: usize,
    /// True when a server registration exists for this profile.
    pub registered: bool,
}

/// Result of a `getFp` registration round trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationSummary {
    pub account: String,
    pub device_id_masked: String,
    pub device_fp_masked: String,
    pub device_fp_len: usize,
}

/// Masked view of a stored session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub account: String,
    pub account_uid_len: usize,
    pub stoken_len: usize,
    /// `v2_` for current tokens; empty when unknown.
    pub stoken_prefix: String,
    pub mid_masked: String,
    /// Mid that goes into the QR Cookie (per account).
    pub qr_mid_masked: String,
    pub cookie_token_len: usize,
    pub ltoken_len: usize,
    pub updated_at: String,
    /// Progress steps of the operation that produced this summary.
    pub steps: Vec<StepReport>,
}

/// A manual challenge the UI must surface to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginChallengeDto {
    /// `sms_captcha` | `interactive_risk` | `real_name` | `unknown`
    pub kind: String,
    pub retcode: i64,
    pub message: String,
    pub hints: Vec<String>,
    /// kind == "aigis"：应用内求解页地址（本地服务器，等待人工完成）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aigis_url: Option<String>,
    /// kind == "aigis"：求解会话句柄，完成后传给 aigis_take。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aigis_handle: Option<u64>,
    /// Ready-to-display manual instructions. Never an automated bypass.
    pub instructions: String,
}

impl LoginChallengeDto {
    pub fn from_parts(
        kind: LoginChallengeKind,
        retcode: i64,
        message: String,
        hints: Vec<String>,
    ) -> Self {
        let kind_str = match kind {
            LoginChallengeKind::SmsCaptcha => "sms_captcha",
            LoginChallengeKind::Aigis => "aigis",
            LoginChallengeKind::InteractiveRisk => "interactive_risk",
            LoginChallengeKind::RealName => "real_name",
            LoginChallengeKind::Unknown => "unknown",
        };
        let instructions = match kind {
            // The UI offers a button that switches to SMS mode; the code itself is
            // always typed by the user.
            LoginChallengeKind::SmsCaptcha => {
                "这是新设备或登录验证：请点击「改用短信登录」，获取验证码后把你收到的验证码填入。\
                 验证码由你手动输入，本工具不会自动读取或填写。"
            }
            LoginChallengeKind::Aigis => {
                "需要图形验证码（极验）：重试时工具会自动打开本地求解页面，请在浏览器中完成。\
                 本工具不自动破解验证码，只把极验官方组件交给你手动完成。"
            }
            LoginChallengeKind::InteractiveRisk => {
                "服务端要求图形验证码（极验/aigis）或存在设备风控。本工具不会自动过验：\
                 请等待一段时间后重试，或改用官方米游社 App 操作。"
            }
            LoginChallengeKind::RealName => "请在官方 App 完成实名验证后重试。",
            LoginChallengeKind::Unknown => "请稍后重试，或改用短信登录。本工具不会绕过账号风控。",
        };
        Self {
            kind: kind_str.to_string(),
            retcode,
            message,
            hints,
            instructions: instructions.to_string(),
            aigis_url: None,
            aigis_handle: None,
        }
    }
}

/// 进行中的单路捕获进度：让界面能显示「在扫、扫到什么」，
/// 而不是一个静默的转圈。只含计数与状态，不含任何画面或令牌。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScanProgressDto {
    /// 已抓取的帧数。
    pub frames: u64,
    /// 解出过二维码的帧数（同一张码反复解出会累计）。
    pub decoded: u64,
    /// 已解出但还在等待稳定（需连续/近似连续的重复）。
    pub waiting_stable: bool,
    /// 最近一次失败（抓屏失败或解码失败）的原因。
    pub last_error: String,
}

/// A persisted string setting (e.g. last logged-in account label).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextDto {
    pub value: String,
}

/// Result of probing a stored session against the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProbeDto {
    /// true = the stored stoken still works.
    pub valid: bool,
    /// Human-readable outcome (no tokens).
    pub message: String,
    /// Masked account id from the probe response, when valid.
    pub uid_masked: String,
}

/// A persisted on/off flag (developer mode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagDto {
    pub enabled: bool,
    /// Unix seconds of the last change; empty when never set.
    #[serde(default)]
    pub updated_at: String,
}

/// Solved aigis header value (session-bound, single-use).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AigisTokenDto {
    pub token: String,
}

/// One watched source (Bilibili room or the local screen) in a race.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherStateDto {
    /// 稳定键：label 或房间号。
    pub key: String,
    /// "bilibili" | "screen"。
    pub kind: String,
    pub room: Option<String>,
    pub label: String,
    /// "approve"（自动批准）| "scan"（仅扫描，等待手动批准）。
    pub mode: String,
    pub alive: bool,
    /// 该源的稳定二维码抢到了胜者名额。
    pub winner: bool,
    pub scanned: bool,
    pub approved: bool,
    pub qr_url: Option<String>,
    /// mode == "scan" 时暂存的票据，等待手动批准（qr_confirm_pending）。
    pub pending_ticket: Option<String>,
    pub pending_token_types: Vec<String>,
    pub error: Option<String>,
    /// 胜者扫描到的游戏显示名（供结果卡与弹窗展示；未扫描时为空）。
    #[serde(default)]
    pub app_name: String,
    /// 扫码账号的显示名（掩码）。
    #[serde(default)]
    pub account_disp_name: String,
    /// 服务端风控备注（通常为空）。
    #[serde(default)]
    pub risk_note: String,
    /// 已解码帧数（直播源 = ffmpeg 输出帧；屏幕源 = 截屏次数）。
    pub frames: u64,
    /// 解出过二维码的帧数。
    #[serde(default)]
    pub decoded_frames: u64,
    /// 最近一帧的解码说明，如「本帧未解出二维码」。
    #[serde(default)]
    pub hint: String,
    /// 最近一帧的分辨率（如 1280x720）。
    pub resolution: String,
    /// 实测帧率（帧/s，滚动窗口均值）。
    pub fps: f64,
    /// 与流服务器的 TCP 连接时延（≈ping，毫秒）；屏幕源为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping_ms: Option<u64>,
}

/// Live status of a multi-source race.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceStatusDto {
    pub running: bool,
    pub watchers_total: u32,
    pub watchers_alive: u32,
    pub approved: bool,
    /// First watcher-level failure worth reporting.
    pub error: Option<String>,
    /// Per-watcher states, in the order the specs were given.
    pub watchers: Vec<WatcherStateDto>,
}

/// One race source spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatcherSpecDto {
    /// "bilibili" | "screen"。
    pub kind: String,
    pub room: Option<String>,
    pub label: String,
    /// "approve" | "scan"。
    pub mode: String,
}

/// One step of a multi-step operation, for progress display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReport {
    /// 1-based step number.
    pub index: u32,
    pub total: u32,
    pub name: String,
    pub detail: String,
    pub ok: bool,
}

/// What the server said when an SMS code was requested.
///
/// The UI needs `countdown` to offer a resend; without it the request button
/// latches after the first send and a failed login has no way back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmsRequestDto {
    /// False when the server reused a code it had already sent.
    #[serde(default)]
    pub sent_new: bool,
    /// Seconds before another request is allowed; 0 when the server did not say.
    #[serde(default)]
    pub countdown: u64,
    /// `action_type` the server expects on the submit call; usually
    /// `login_by_mobile_captcha`.
    #[serde(default)]
    pub action_type: String,
}

/// One line of the audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntryDto {
    /// Unix seconds.
    pub at: String,
    pub account: String,
    /// `qr_login_game` | `qr_scan` | `qr_cancel`
    pub action: String,
    pub ok: bool,
    /// Non-secret summary. Never a token, ticket, or code.
    pub detail: String,
    /// Server retcode when the action failed on the server side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retcode: Option<i64>,
}

/// Outcome of a QR approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalOutcome {
    pub steps: Vec<StepReport>,
    /// Game reported by the scan response (e.g. 原神). Empty when unknown.
    pub app_name: String,
    /// Masked account name from the scan response.
    pub account_disp_name: String,
    /// Risk note from the scan response, when the server sent one.
    pub risk_note: String,
    pub scanned: bool,
    pub approved: bool,
    /// 来源描述：屏幕捕获/截图文件/二维码链接/直播·<标签或房间号>。
    #[serde(default)]
    pub source: String,
    /// 仅扫描时暂存：手动批准所需的 passport 票据（一次性，短期有效）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_ticket: Option<String>,
    /// 仅扫描时暂存：批准所需的 token_types。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_token_types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_short_values() {
        assert_eq!(mask("abc"), "***");
        assert_eq!(mask(""), "***");
    }

    #[test]
    fn mask_long_values_keeps_head_and_tail() {
        let m = mask("0123456789abcdef");
        assert_eq!(m, "0123...cdef");
        assert!(!m.contains("456789ab"));
    }

    #[test]
    fn challenge_dto_maps_kind_and_instructions() {
        let dto = LoginChallengeDto::from_parts(
            LoginChallengeKind::SmsCaptcha,
            0,
            "need code".into(),
            vec!["captcha".into()],
        );
        assert_eq!(dto.kind, "sms_captcha");
        assert!(dto.instructions.contains("短信"));
        let risk = LoginChallengeDto::from_parts(
            LoginChallengeKind::InteractiveRisk,
            -3235,
            "risk".into(),
            vec![],
        );
        assert_eq!(risk.kind, "interactive_risk");
        assert_eq!(risk.retcode, -3235);
        // the instruction must never promise an automatic bypass
        assert!(risk.instructions.contains("不会自动过验"));
    }
}
