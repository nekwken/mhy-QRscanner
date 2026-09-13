//! QR API: scan / approve a game QR, cancel a pending approval.
//!
//! Approval order is fixed by the server: panda scan → `scanQRLogin` →
//! `confirmQRLogin`. Skipping the middle call makes confirm fail.

use super::{session_key, validate_account};
use crate::dto::{ApprovalOutcome, ScanProgressDto, StepReport};
use crate::events::push_step;
use crate::{BridgeError, Core};
use mhy_qrscanner_capture::capture_virtual_screen;
use mhy_qrscanner_core::AccountSession;
use mhy_qrscanner_mihoyo::ds::Salt;
use mhy_qrscanner_qr::{
    decode_qr_luma, is_v1_qr_url, panda_scan_from_v1_url, parse_v2_qr_url, passport_qr_cookie,
    HttpQrApi, PassportQrConfirmRequest, PassportQrScanRequest, StabilityFilter,
};

/// Where the game QR comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrSource {
    /// An already-decoded V1 game QR URL.
    Url(String),
    /// Path to a screenshot containing the QR.
    Image(String),
    /// Capture the screen until a stable QR appears. `wait_secs = None` keeps
    /// scanning with no timeout (the default the UI sends); a value stops with
    /// an error after that many seconds.
    Screen { wait_secs: Option<u64> },
    /// Pull a Bilibili room's live stream via ffmpeg and decode frames until a
    /// stable QR appears (same wait semantics as `Screen`). The charter input
    /// path: lower latency than capturing a player window.
    Bilibili {
        room_id: String,
        wait_secs: Option<u64>,
    },
}

const APPROVE_STEPS: u32 = 3;

/// 手动停止单路捕获（屏幕/直播）的全局旗标；每次扫描开始时复位。
static SCAN_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// UI 的「停止捕获」按钮：让当前进行中的单路扫描在下个帧边界退出。
pub fn scan_stop() {
    SCAN_STOP.store(true, std::sync::atomic::Ordering::SeqCst);
}

fn progress() -> &'static std::sync::Mutex<ScanProgressDto> {
    static P: std::sync::OnceLock<std::sync::Mutex<ScanProgressDto>> = std::sync::OnceLock::new();
    P.get_or_init(|| std::sync::Mutex::new(ScanProgressDto::default()))
}

/// 单路捕获的实时进度。界面每秒轮询一次，把「在扫、扫到什么」显示出来，
/// 避免长时间无反馈让人以为工具没工作。
pub fn qr_scan_progress() -> ScanProgressDto {
    progress().lock().map(|p| p.clone()).unwrap_or_default()
}

fn progress_reset() {
    if let Ok(mut p) = progress().lock() {
        *p = ScanProgressDto::default();
    }
}

/// 同一份逐帧状态写两份：全局进度（单路扫描的界面）与某个源的
/// SharedStats（竞速卡片显示「在扫什么」）。
fn note_frame(
    stats: Option<&mhy_qrscanner_live::SharedStats>,
    decoded: bool,
    stable_hits: usize,
    hint: Option<String>,
) {
    progress_frame(decoded, stable_hits, hint.clone());
    if let Some(stats) = stats {
        if let Ok(mut st) = stats.lock() {
            if decoded {
                st.decoded += 1;
                st.hint = if stable_hits > 1 {
                    "已识别二维码，确认稳定性中".to_string()
                } else {
                    "已识别二维码，等待下一帧确认".to_string()
                };
            } else if let Some(h) = hint {
                st.hint = h;
            }
        }
    }
}

fn progress_frame(decoded: bool, stable_hits: usize, hint: Option<String>) {
    if let Ok(mut p) = progress().lock() {
        p.frames += 1;
        if decoded {
            p.decoded += 1;
        }
        p.waiting_stable = stable_hits > 0;
        if let Some(hint) = hint {
            p.last_error = hint;
        }
    }
}

/// Tickets approved within this window are refused on re-submission: the same
/// QR shown across rooms/frames (or re-displayed by the streamer) must not
/// fire a second approval.
const TICKET_TTL: std::time::Duration = std::time::Duration::from_secs(10 * 60);

fn seen_tickets() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>
{
    static SEEN: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    > = std::sync::OnceLock::new();
    SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// True when this ticket was not approved recently; prunes expired entries.
fn ticket_is_fresh(ticket: &str) -> bool {
    let mut seen = seen_tickets().lock().unwrap();
    seen.retain(|_, at| at.elapsed() < TICKET_TTL);
    !seen.contains_key(ticket)
}

fn ticket_mark(ticket: &str) {
    seen_tickets()
        .lock()
        .unwrap()
        .insert(ticket.to_string(), std::time::Instant::now());
}

fn load_session(core: &Core, account: &str) -> Result<AccountSession, BridgeError> {
    let session = core
        .store()
        .load::<AccountSession>(&session_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(format!("{account} (no session)")))?;
    if session.stoken.is_empty() {
        return Err(BridgeError::Invalid(
            "stored session has no token; log in again".into(),
        ));
    }
    Ok(session)
}

fn build_headers(core: &Core, account: &str) -> Result<mhy_qrscanner_mihoyo::RpcHeaders, BridgeError> {
    let profile = core
        .store()
        .load::<mhy_qrscanner_core::DeviceProfile>(&super::profile_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(account.to_string()))?;
    let registration = core
        .store()
        .load::<mhy_qrscanner_core::DeviceRegistration>(&super::registration_key(account))?;
    let device_fp = registration
        .as_ref()
        .map(|r| r.device_fp.clone())
        .unwrap_or_else(|| profile.initial_device_fp.clone());
    Ok(mhy_qrscanner_mihoyo::RpcHeaders {
        app_id: profile.client_profile.app_id.clone(),
        client_type: profile.client_profile.client_type.clone(),
        device_id: profile.device_id.clone(),
        device_fp,
        device_name: profile.device_name.clone(),
        device_model: profile.model.clone(),
        sys_version: profile.android_version.clone(),
        game_biz: profile.client_profile.game_biz.clone(),
        app_version: profile.client_profile.app_version.clone(),
        sdk_version: profile.client_profile.sdk_version.clone(),
        lifecycle_id: mhy_qrscanner_mihoyo::RpcHeaders::miyoushe_defaults("", "").lifecycle_id,
        account_version: profile.client_profile.sdk_version.clone(),
    })
}

/// The `game_biz` in the panda URL path is the **game title** biz (`hk4e_cn`),
/// which differs from the `x-rpc-game_biz` header (`bbs_cn`, the Miyoushe client
/// biz). Mixing them yields a 404.
fn title_biz(core: &Core, account: &str) -> Result<String, BridgeError> {
    let profile = core
        .store()
        .load::<mhy_qrscanner_core::DeviceProfile>(&super::profile_key(account))?
        .ok_or_else(|| BridgeError::AccountNotFound(account.to_string()))?;
    Ok(profile.game_biz.as_str().to_string())
}

/// Resolve a [`QrSource`] into a V1 game QR URL.
fn resolve_game_url(source: &QrSource) -> Result<String, BridgeError> {
    let url = match source {
        QrSource::Url(u) => u.clone(),
        QrSource::Image(path) => {
            let p = std::path::Path::new(path);
            if !p.exists() {
                return Err(BridgeError::Invalid(format!("image not found: {path}")));
            }
            mhy_qrscanner_qr::decode_qr_file(p)?
        }
        QrSource::Screen { wait_secs } => capture_stable_qr(*wait_secs)?,
        QrSource::Bilibili { room_id, wait_secs } => bilibili_stable_qr(room_id, *wait_secs)?,
    };
    if !is_v1_qr_url(&url) {
        return Err(BridgeError::Invalid(
            "not a game QR (expected qr_code_in_game.html)".into(),
        ));
    }
    Ok(url)
}

/// Capture until the same payload decodes twice in a row. `None` scans until
/// it succeeds; the UI's stop button is what ends an unlimited run's *next*
/// attempt (each call is one capture loop, so the deadline is the only in-loop
/// stop for the bounded form).
fn capture_stable_qr(wait_secs: Option<u64>) -> Result<String, BridgeError> {
    SCAN_STOP.store(false, std::sync::atomic::Ordering::SeqCst);
    progress_reset();
    let deadline = wait_secs.map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s));
    watch_screen(&SCAN_STOP, deadline, None)
}

/// 屏幕捕获狩猎循环。`stop` 可注入（竞速线程传竞速旗标，单路传全局旗标）。
/// `last_error`：竞速模式下把首个错误写回各源状态；单路模式传 None（用返回值）。
pub(crate) fn watch_screen(
    stop: &std::sync::atomic::AtomicBool,
    deadline: Option<std::time::Instant>,
    stats: Option<&mhy_qrscanner_live::SharedStats>,
) -> Result<String, BridgeError> {
    let mut filter = StabilityFilter::new(2);
    let mut consecutive_failures = 0u32;
    let mut screen_frames: u64 = 0;
    let watch_started = std::time::Instant::now();
    loop {
        // 只看注入的旗标：单路模式注入的就是全局 SCAN_STOP，竞速注入自己的旗标。
        // （此前这里还额外读全局标志，导致竞速被上一次「停止捕获」误杀。）
        if stop.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(BridgeError::Capture("已手动停止捕获".into()));
        }
        if let Some(deadline) = deadline {
            if std::time::Instant::now() >= deadline {
                return Err(BridgeError::Capture("捕获等待超时".into()));
            }
        }
        match capture_virtual_screen() {
            Ok(frame) => {
                screen_frames += 1;
                if let Some(stats) = stats {
                    let span = watch_started.elapsed().as_secs_f64();
                    let mut st = stats.lock().unwrap();
                    st.frames = screen_frames;
                    st.width = frame.width;
                    st.height = frame.height;
                    st.fps = if span > 0.0 {
                        screen_frames as f64 / span
                    } else {
                        0.0
                    };
                }
                match decode_qr_luma(frame.to_luma8()) {
                    Ok(payload) => {
                        let settled = filter.observe(&payload);
                        note_frame(stats, true, filter.hits(), None);
                        if settled {
                            return Ok(payload);
                        }
                    }
                    Err(_) => {
                        filter.miss();
                        note_frame(
                            stats,
                            false,
                            filter.hits(),
                            Some("本帧未解出二维码".into()),
                        );
                    }
                }
            }
            // A raced window handle fails the first capture after a window
            // appears (`BitBlt failed (error 6)`, live 2026-09-12); ride it
            // out instead of aborting the scan.
            Err(e) => {
                consecutive_failures += 1;
                note_frame(stats, false, filter.hits(), Some(format!("抓屏失败：{e}")));
                if consecutive_failures >= 8 {
                    return Err(BridgeError::Capture(e.to_string()));
                }
                filter.miss();
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
    }
}

pub(crate) fn watch_bilibili(
    room: &str,
    stop: &std::sync::atomic::AtomicBool,
    deadline: Option<std::time::Instant>,
    stats: Option<&mhy_qrscanner_live::SharedStats>,
) -> Result<String, BridgeError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| BridgeError::Capture(e.to_string()))?;
    let stream = mhy_qrscanner_live::resolve_play_url(room, "https://api.live.bilibili.com", &client)
        .map_err(|e| BridgeError::Capture(format!("bilibili: {e}")))?;
    let ffmpeg = std::env::var("MHYQR_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string());
    // 到流服务器的 ping：TCP 连接时延（一次，约等于建连 RTT）
    let ping_ms = mhy_qrscanner_live::ping_stream_server(&stream.url, std::time::Duration::from_secs(2)).ok();
    if let Some(stats) = stats {
        stats.lock().unwrap().ping_ms = ping_ms;
    }
    let mut pump = mhy_qrscanner_live::FramePump::start(
        &ffmpeg,
        &stream.url,
        1280,
        720,
        2,
        "https://live.bilibili.com/",
    )
    .map_err(|e| BridgeError::Capture(format!("bilibili: {e}")))?;

    let mut filter = StabilityFilter::new(2);
    loop {
        if let Some(deadline) = deadline {
            if std::time::Instant::now() >= deadline {
                return Err(BridgeError::Capture("捕获等待超时".into()));
            }
        }
        // 只看注入的旗标：单路模式注入的就是全局 SCAN_STOP，竞速注入自己的旗标。
        // （此前这里还额外读全局标志，导致竞速被上一次「停止捕获」误杀。）
        if stop.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(BridgeError::Capture("已手动停止捕获".into()));
        }
        match pump.read_frame() {
            Some(frame) => {
                if let Some(stats) = stats {
                    let mut st = stats.lock().unwrap();
                    st.frames = pump.frames();
                    st.width = pump.width();
                    st.height = pump.height();
                    st.fps = pump.fps();
                }
                match image::GrayImage::from_raw(frame.width, frame.height, frame.data)
                    .ok_or_else(|| BridgeError::Capture("frame size mismatch".into()))
                    .and_then(|img| {
                        decode_qr_luma(img).map_err(|e| BridgeError::Capture(e.to_string()))
                    }) {
                    Ok(payload) => {
                        let settled = filter.observe(&payload);
                        note_frame(stats, true, filter.hits(), None);
                        if settled {
                            return Ok(payload);
                        }
                    }
                    Err(_) => {
                        filter.miss();
                        note_frame(
                            stats,
                            false,
                            filter.hits(),
                            Some("本帧未解出二维码".into()),
                        );
                    }
                }
            }
            None => {
                return Err(BridgeError::Capture(
                    "live stream ended before a QR appeared".into(),
                ));
            }
        }
    }
}

/// 单路 B 站来源（保留给 API 完整性；GUI 的直播来源走竞速接口）。
fn bilibili_stable_qr(room_id: &str, wait_secs: Option<u64>) -> Result<String, BridgeError> {
    SCAN_STOP.store(false, std::sync::atomic::Ordering::SeqCst);
    progress_reset();
    let deadline = wait_secs.map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s));
    watch_bilibili(room_id, &SCAN_STOP, deadline, None)
}

/// Approve (or just scan) a game QR in one call.
///
/// `confirm = false` scans only: useful as a dry run and as the first half of a
/// manual-confirmation dialog.
pub async fn qr_login_game(
    core: &Core,
    account: &str,
    source: QrSource,
    confirm: bool,
) -> Result<ApprovalOutcome, BridgeError> {
    let result = login_game(core, account, source, confirm).await;
    match &result {
        Ok(outcome) => super::settings::record(
            core,
            account,
            "qr_login_game",
            true,
            &format!(
                "{} ({})",
                if confirm { "approved" } else { "scanned only" },
                outcome.app_name
            ),
            None,
        ),
        Err(e) => super::settings::record(
            core,
            account,
            "qr_login_game",
            false,
            &e.to_string(),
            e.api_retcode(),
        ),
    }
    result
}

async fn login_game(
    core: &Core,
    account: &str,
    source: QrSource,
    confirm: bool,
) -> Result<ApprovalOutcome, BridgeError> {
    validate_account(account)?;
    let source_label = match &source {
        QrSource::Url(_) => "二维码链接",
        QrSource::Image(_) => "截图文件",
        QrSource::Screen { .. } => "屏幕捕获",
        // 单路直播：与「监控来源」里的多源竞速并存，供只盯一个直播间时使用。
        QrSource::Bilibili { .. } => "B站直播间",
    };
    let game_url = resolve_game_url(&source)?;
    approve_game_url(core, account, game_url, confirm, source_label).await
}

/// Approve (or dry-run) an already-resolved V1 game QR URL. Shared by
/// `qr_login_game` and the multi-room race threads.
pub(crate) async fn approve_game_url(
    core: &Core,
    account: &str,
    game_url: String,
    confirm: bool,
    source: &str,
) -> Result<ApprovalOutcome, BridgeError> {
    if confirm {
        let ticket = mhy_qrscanner_qr::ticket_from_v1_url(&game_url).unwrap_or_default();
        if !ticket.is_empty() && !ticket_is_fresh(&ticket) {
            return Err(BridgeError::Invalid(
                "该二维码 10 分钟内已批准过（重复票据），跳过".into(),
            ));
        }
    }

    let headers = build_headers(core, account)?;
    let session = load_session(core, account)?;
    let game_biz = title_biz(core, account)?;
    let api = HttpQrApi::new(core.panda_host_for(&game_biz), core.passport_host());

    // 1/3 panda scan
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let preq = panda_scan_from_v1_url(&game_url, &headers.device_id, &headers.app_id, ts)?;
    let pdata = api.panda_scan(&headers, &game_biz, &preq).await?;
    let mut steps = Vec::new();
    push_step(
        &mut steps,
        1,
        APPROVE_STEPS,
        "panda_scan",
        "game QR scanned",
        true,
    );
    if pdata.passport_qr_url.is_empty() {
        return Err(BridgeError::Invalid(
            "scan returned no passport QR url".into(),
        ));
    }
    let (ticket, token_types) = parse_v2_qr_url(&pdata.passport_qr_url)
        .ok_or_else(|| BridgeError::Invalid("passport QR url is not parseable".into()))?;

    let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
    let sdata = scan_passport(
        core,
        account,
        &headers,
        &api,
        &cookie,
        &ticket,
        &token_types,
        &mut steps,
    )
    .await?;
    let steps = approve_or_skip(
        confirm,
        &api,
        &headers,
        &cookie,
        ticket.clone(),
        token_types.clone(),
        steps,
    )
    .await?;
    if confirm {
        ticket_mark(&ticket);
    }

    // 部分游戏（实测崩坏3）的 scanQRLogin 响应不带 app_name/账号显示名——
    // 用标题名与会话 uid 掩码兜底，别给用户看空字段。
    let app_display = if sdata.app_name.is_empty() {
        match game_biz.as_str() {
            "bh3_cn" => "崩坏3",
            "hkrpg_cn" => "崩坏:星穹铁道",
            "nap_cn" => "绝区零",
            "hk4e_cn" | "hk4e_global" => "原神",
            other => other,
        }
    } else {
        sdata.app_name.as_str()
    };
    let account_display = if sdata.account_disp_name.is_empty() {
        crate::dto::mask(&session.account_uid)
    } else {
        sdata.account_disp_name.clone()
    };
    Ok(ApprovalOutcome {
        steps,
        app_name: app_display.to_string(),
        account_disp_name: account_display.to_string(),
        risk_note: sdata.risk_note,
        scanned: true,
        approved: confirm,
        source: source.to_string(),
        pending_ticket: if confirm { None } else { Some(ticket) },
        pending_token_types: if confirm { Vec::new() } else { token_types },
    })
}

/// 2/3: scanQRLogin marks the ticket scanned for this account.
#[allow(clippy::too_many_arguments)]
async fn scan_passport(
    _core: &Core,
    _account: &str,
    headers: &mhy_qrscanner_mihoyo::RpcHeaders,
    api: &HttpQrApi,
    cookie: &str,
    ticket: &str,
    token_types: &[String],
    steps: &mut Vec<StepReport>,
) -> Result<mhy_qrscanner_qr::QrScanData, BridgeError> {
    let sreq = PassportQrScanRequest {
        ticket: ticket.to_string(),
        token_types: token_types.to_vec(),
    };
    let sdata = api
        .passport_scan(headers, &Salt::Prod, cookie, &sreq)
        .await?;
    let detail = if sdata.app_name.is_empty() {
        "scanned".to_string()
    } else {
        format!("scanned: {}", sdata.app_name)
    };
    push_step(steps, 2, APPROVE_STEPS, "scan_qr_login", detail, true);
    Ok(sdata)
}

/// 3/3: confirmQRLogin performs the approval (or is skipped in a dry run).
async fn approve_or_skip(
    confirm: bool,
    api: &HttpQrApi,
    headers: &mhy_qrscanner_mihoyo::RpcHeaders,
    cookie: &str,
    ticket: String,
    token_types: Vec<String>,
    mut steps: Vec<StepReport>,
) -> Result<Vec<StepReport>, BridgeError> {
    if confirm {
        let creq = PassportQrConfirmRequest::approve(ticket, token_types);
        api.passport_confirm(headers, &Salt::Prod, cookie, &creq)
            .await?;
        push_step(
            &mut steps,
            3,
            APPROVE_STEPS,
            "confirm_qr_login",
            "approval sent to the game client",
            true,
        );
    } else {
        push_step(
            &mut steps,
            3,
            APPROVE_STEPS,
            "confirm_qr_login",
            "skipped (dry run)",
            true,
        );
    }
    Ok(steps)
}

/// Scan an already-decoded passport QR URL (the V2 form).
pub async fn qr_scan(
    core: &Core,
    account: &str,
    passport_qr_url: &str,
    confirm: bool,
) -> Result<ApprovalOutcome, BridgeError> {
    let result = scan_only(core, account, passport_qr_url, confirm).await;
    match &result {
        Ok(outcome) => super::settings::record(
            core,
            account,
            "qr_scan",
            true,
            &format!(
                "{} ({})",
                if confirm { "approved" } else { "scanned only" },
                outcome.app_name
            ),
            None,
        ),
        Err(e) => super::settings::record(
            core,
            account,
            "qr_scan",
            false,
            &e.to_string(),
            e.api_retcode(),
        ),
    }
    result
}

async fn scan_only(
    core: &Core,
    account: &str,
    passport_qr_url: &str,
    confirm: bool,
) -> Result<ApprovalOutcome, BridgeError> {
    validate_account(account)?;
    let (ticket, token_types) = parse_v2_qr_url(passport_qr_url)
        .ok_or_else(|| BridgeError::Invalid("not a passport QR url".into()))?;
    let headers = build_headers(core, account)?;
    let session = load_session(core, account)?;
    let api = HttpQrApi::new(
        core.panda_host_for(&title_biz(core, account)?),
        core.passport_host(),
    );
    let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());

    let mut steps = Vec::new();
    let sdata = scan_passport(
        core,
        account,
        &headers,
        &api,
        &cookie,
        &ticket,
        &token_types,
        &mut steps,
    )
    .await?;
    let steps = approve_or_skip(
        confirm,
        &api,
        &headers,
        &cookie,
        ticket.clone(),
        token_types.clone(),
        steps,
    )
    .await?;

    Ok(ApprovalOutcome {
        steps,
        app_name: sdata.app_name,
        account_disp_name: sdata.account_disp_name,
        risk_note: sdata.risk_note,
        scanned: true,
        approved: confirm,
        source: "二维码链接".into(),
        pending_ticket: if confirm { None } else { Some(ticket) },
        pending_token_types: if confirm { Vec::new() } else { token_types },
    })
}

/// Complete a scan-only run: approve the ticket that 扫描 already fetched.
pub async fn qr_confirm_pending(
    core: &Core,
    account: &str,
    ticket: &str,
    token_types: Vec<String>,
) -> Result<ApprovalOutcome, BridgeError> {
    validate_account(account)?;
    if ticket.trim().is_empty() {
        return Err(BridgeError::Invalid("ticket is empty".into()));
    }
    if !ticket_is_fresh(ticket) {
        return Err(BridgeError::Invalid(
            "该二维码 10 分钟内已批准过（重复票据），跳过".into(),
        ));
    }
    let headers = build_headers(core, account)?;
    let session = load_session(core, account)?;
    let api = HttpQrApi::new(
        core.panda_host_for(&title_biz(core, account)?),
        core.passport_host(),
    );
    let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());

    let steps = Vec::new();
    let result = approve_or_skip(
        true,
        &api,
        &headers,
        &cookie,
        ticket.to_string(),
        token_types.clone(),
        steps,
    )
    .await;
    match &result {
        Ok(_) => {
            ticket_mark(ticket);
            super::settings::record(core, account, "qr_confirm_pending", true, "approved", None);
        }
        Err(e) => super::settings::record(
            core,
            account,
            "qr_confirm_pending",
            false,
            &e.to_string(),
            e.api_retcode(),
        ),
    }
    let steps = result?;
    Ok(ApprovalOutcome {
        steps,
        app_name: String::new(),
        account_disp_name: String::new(),
        risk_note: String::new(),
        scanned: true,
        approved: true,
        source: "手动批准".into(),
        pending_ticket: None,
        pending_token_types: Vec::new(),
    })
}

/// Cancel a pending approval. Accepts a passport QR URL or a bare ticket.
pub async fn qr_cancel(
    core: &Core,
    account: &str,
    url_or_ticket: &str,
    token_types: Vec<String>,
) -> Result<(), BridgeError> {
    let result = cancel(core, account, url_or_ticket, token_types).await;
    match &result {
        Ok(()) => super::settings::record(
            core,
            account,
            "qr_cancel",
            true,
            "pending approval cancelled",
            None,
        ),
        Err(e) => super::settings::record(
            core,
            account,
            "qr_cancel",
            false,
            &e.to_string(),
            e.api_retcode(),
        ),
    }
    result
}

async fn cancel(
    core: &Core,
    account: &str,
    url_or_ticket: &str,
    token_types: Vec<String>,
) -> Result<(), BridgeError> {
    validate_account(account)?;
    let (ticket, token_types) = match parse_v2_qr_url(url_or_ticket) {
        Some((tk, types)) => {
            let types = if types.is_empty() { token_types } else { types };
            (tk, types)
        }
        None => (url_or_ticket.to_string(), token_types),
    };
    let headers = build_headers(core, account)?;
    let session = load_session(core, account)?;
    let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
    let api = HttpQrApi::new(
        core.panda_host_for(&title_biz(core, account)?),
        core.passport_host(),
    );
    api.passport_cancel(&headers, &Salt::Prod, &cookie, &ticket, &token_types)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn setup(core: &Core) {
        crate::api::device::device_create(core, "acct-1", "xiaomi14", false).unwrap();
        let session = AccountSession {
            account_uid: "42".into(),
            stoken: "v2_abcdefghijklmnop".into(),
            mid: "03pmu45q7y_mhy".into(),
            cookie_mid: "0cdapswfd1_mhy".into(),
            stoken_v2: String::new(),
            ltoken: String::new(),
            cookie_token: String::new(),
            updated_at: "1".into(),
        };
        core.store()
            .save(&super::session_key("acct-1"), &session)
            .unwrap();
    }

    fn game_url() -> String {
        "https://user.mihoyo.com/qr_code_in_game.html?app_id=4&ticket=tk1&biz_key=hk4e_cn"
            .to_string()
    }

    #[tokio::test]
    async fn login_game_runs_panda_scan_confirm_in_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hk4e_cn/combo/panda/qrcode/scan"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK",
                "data": { "passport_qr_url": "https://user.mihoyo.com/login-platform/mobile.html?expire=1&tk=tk-9&token_types=1#/login/qr" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/scanQRLogin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK",
                "data": { "app_name": "原神", "account_disp_name": "195******50" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/confirmQRLogin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK", "data": {}
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path())
            .with_passport_host(server.uri())
            .with_panda_host(server.uri());
        setup(&core);

        let outcome = qr_login_game(&core, "acct-1", QrSource::Url(game_url()), true)
            .await
            .unwrap();
        assert!(outcome.scanned && outcome.approved);
        assert_eq!(outcome.app_name, "原神");
        assert_eq!(outcome.steps.len(), 3);
        assert_eq!(outcome.steps[0].name, "panda_scan");
        assert_eq!(outcome.steps[2].name, "confirm_qr_login");

        let audit = super::super::settings::audit_recent(&core, 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].action, "qr_login_game");
        assert!(audit[0].ok);
        assert!(audit[0].detail.contains("approved"));
        assert!(audit[0].detail.contains("原神"));
        assert_eq!(audit[0].retcode, None);
    }

    #[tokio::test]
    async fn dry_run_skips_confirm_but_still_scans() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hk4e_cn/combo/panda/qrcode/scan"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK",
                "data": { "passport_qr_url": "https://user.mihoyo.com/login-platform/mobile.html?tk=tk-9&token_types=1#/login/qr" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/account/ma-cn-passport/app/scanQRLogin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retcode": 0, "message": "OK", "data": { "app_name": "原神" }
            })))
            .mount(&server)
            .await;
        // No confirm mock: the call must not happen.

        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path())
            .with_passport_host(server.uri())
            .with_panda_host(server.uri());
        setup(&core);
        let outcome = qr_login_game(&core, "acct-1", QrSource::Url(game_url()), false)
            .await
            .unwrap();
        assert!(outcome.scanned);
        assert!(!outcome.approved);
        assert!(outcome.steps[2].detail.contains("dry run"));
    }

    /// 捕获过程要把进度写进 qr_scan_progress（界面靠它显示「在扫、扫到什么」）。
    #[test]
    fn a_capture_run_reports_progress() {
        let _ = capture_stable_qr(Some(1));
        let p = qr_scan_progress();
        assert!(p.frames > 0, "应至少统计到一帧：{p:?}");
    }

    /// 诊断：上一轮「停止捕获」置位的全局标志，会不会让下一轮捕获秒退。
    #[test]
    fn a_previous_stop_does_not_poison_the_next_capture() {
        scan_stop();
        match capture_stable_qr(Some(1)) {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    !msg.contains("已手动停止捕获"),
                    "global stop flag leaked into the next run: {msg}"
                );
            }
        }
    }

    #[tokio::test]
    async fn login_game_rejects_non_game_url() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        setup(&core);
        let err = qr_login_game(
            &core,
            "acct-1",
            QrSource::Url("https://user.mihoyo.com/index.html".into()),
            true,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, BridgeError::Invalid(_)));

        // a rejected approval still leaves a trace, with the reason
        let audit = super::super::settings::audit_recent(&core, 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert!(!audit[0].ok);
        assert!(audit[0].detail.contains("qr_code_in_game"));
    }

    #[tokio::test]
    async fn login_game_requires_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        crate::api::device::device_create(&core, "acct-1", "xiaomi14", false).unwrap();
        let err = qr_login_game(&core, "acct-1", QrSource::Url(game_url()), true)
            .await
            .unwrap_err();
        assert!(matches!(err, BridgeError::AccountNotFound(_)));
    }

    #[tokio::test]
    async fn missing_image_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(dir.path());
        setup(&core);
        let err = qr_login_game(
            &core,
            "acct-1",
            QrSource::Image("definitely-not-here.png".into()),
            true,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("image not found"));
    }
}
