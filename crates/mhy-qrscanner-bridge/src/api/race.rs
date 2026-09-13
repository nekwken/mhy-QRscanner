//! Multi-source QR race (抢码核心): watch several sources concurrently —
//! Bilibili rooms and/or the local screen — and the first stable QR claims
//! the win. Per-source mode: "approve" lands the login automatically;
//! "scan" parks the ticket for a manual approve (qr_confirm_pending).

use crate::dto::{RaceStatusDto, WatcherSpecDto, WatcherStateDto};
use crate::{BridgeError, Core};
use flutter_rust_bridge::frb;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
#[frb(ignore)]
struct RaceInner {
    states: Vec<WatcherStateDto>,
    error: Option<String>,
}

#[frb(ignore)]
struct RaceHandle {
    stop: Arc<AtomicBool>,
    alive: Arc<AtomicU32>,
    total: u32,
    inner: Arc<Mutex<RaceInner>>,
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn races() -> &'static Mutex<HashMap<u32, RaceHandle>> {
    static R: std::sync::OnceLock<Mutex<HashMap<u32, RaceHandle>>> = std::sync::OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Start a race across the given sources. The first stable QR claims the win;
/// mode decides whether the win auto-approves or parks a pending ticket.
pub async fn qr_race_start(
    core: &Core,
    account: String,
    specs: Vec<WatcherSpecDto>,
    wait_secs: Option<u64>,
) -> Result<u32, BridgeError> {
    crate::api::validate_account(&account)?;
    if specs.is_empty() {
        return Err(BridgeError::Invalid("至少需要一个监控源".into()));
    }
    let handle = tokio::runtime::Handle::current();
    let core = core.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let alive = Arc::new(AtomicU32::new(0));
    let inner = Arc::new(Mutex::new(RaceInner {
        states: specs
            .iter()
            .map(|s| WatcherStateDto {
                key: watcher_key(s),
                kind: s.kind.clone(),
                room: s.room.clone(),
                label: s.label.clone(),
                mode: s.mode.clone(),
                alive: true,
                winner: false,
                scanned: false,
                approved: false,
                qr_url: None,
                pending_ticket: None,
                pending_token_types: Vec::new(),
                error: None,
                decoded_frames: 0,
                hint: String::new(),
                app_name: String::new(),
                account_disp_name: String::new(),
                risk_note: String::new(),
                frames: 0,
                resolution: String::new(),
                fps: 0.0,
                ping_ms: None,
            })
            .collect(),
        error: None,
    }));
    let deadline = wait_secs.map(|s| Instant::now() + Duration::from_secs(s));
    let deadline = Arc::new(deadline);

    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    for (i, spec) in specs.iter().enumerate() {
        let stop = stop.clone();
        let alive = alive.clone();
        let inner = inner.clone();
        let deadline = deadline.clone();
        let core = core.clone();
        let account = account.clone();
        let spec = spec.clone();
        let handle = handle.clone();
        alive.fetch_add(1, Ordering::SeqCst);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_watcher(
                    &core,
                    &account,
                    &spec,
                    stop.clone(),
                    deadline.clone(),
                    &handle,
                    inner.clone(),
                    i,
                )
            }));
            if let Err(_panic) = result {
                if let Ok(mut states) = inner.lock() {
                    states.states[i].error = Some("监控源异常退出".into());
                    if states.error.is_none() {
                        states.error = Some(format!("{} 异常退出", states.states[i].label));
                    }
                }
            }
            if let Ok(mut states) = inner.lock() {
                states.states[i].alive = false;
            }
            alive.fetch_sub(1, Ordering::SeqCst);
        });
    }

    races().lock().unwrap().insert(
        id,
        RaceHandle {
            stop,
            alive,
            total: specs.len() as u32,
            inner,
        },
    );
    Ok(id)
}

fn watcher_key(spec: &WatcherSpecDto) -> String {
    spec.label.clone()
}

/// One watcher's capture→decode→claim→dispatch loop. 统计经 SharedStats 由
/// 独立同步线程搬进竞速状态（watch 循环被 ffmpeg/截屏阻塞，不能自己搬）。
#[allow(clippy::too_many_arguments)]
fn run_watcher(
    core: &Core,
    account: &str,
    spec: &WatcherSpecDto,
    stop: Arc<AtomicBool>,
    deadline: Arc<Option<Instant>>,
    approve_handle: &tokio::runtime::Handle,
    inner: Arc<Mutex<RaceInner>>,
    index: usize,
) {
    let stats: mhy_qrscanner_live::SharedStats = Arc::new(Mutex::new(Default::default()));

    // 统计同步：把 SharedStats 的快照周期性搬进竞速状态供 GUI 轮询。
    let sync_stop = stop.clone();
    let sync_stats = stats.clone();
    let sync_inner = inner.clone();
    let sync_index = index;
    let sync_thread = std::thread::spawn(move || loop {
        if sync_stop.load(Ordering::SeqCst) {
            break;
        }
        {
            let st = sync_stats.lock().unwrap();
            if let Ok(mut states) = sync_inner.lock() {
                let w = &mut states.states[sync_index];
                w.frames = st.frames;
                w.fps = st.fps;
                w.ping_ms = st.ping_ms;
                w.decoded_frames = st.decoded;
                w.hint = st.hint.clone();
                if st.width > 0 {
                    w.resolution = format!("{}x{}", st.width, st.height);
                }
                if st.frames > 0 {
                    w.alive = true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(600));
    });

    let attempted = match spec.kind.as_str() {
        "screen" => crate::api::qr::watch_screen(&stop, *deadline, Some(&stats)),
        "bilibili" => crate::api::qr::watch_bilibili(
            spec.room.as_deref().unwrap_or_default(),
            &stop,
            *deadline,
            Some(&stats),
        ),
        other => {
            if let Ok(mut states) = inner.lock() {
                states.states[index].error = Some(format!("未知监控源类型 {other}"));
            }
            return;
        }
    };
    let game_url = match attempted {
        Ok(url) => url,
        Err(e) => {
            if let Ok(mut states) = inner.lock() {
                states.states[index].error = Some(e.to_string());
            }
            return;
        }
    };

    // Stable QR: claim the win exactly once.
    let claimed = {
        let mut state = inner.lock().unwrap();
        if state.states.iter().any(|w| w.winner) {
            false
        } else {
            state.states[index].winner = true;
            state.states[index].qr_url = Some(game_url.clone());
            true
        }
    };
    if !claimed {
        return; // 别的源已经赢了
    }
    stop.store(true, Ordering::SeqCst); // 胜者已定，全员收工
    let _ = sync_thread.join();

    let outcome = approve_handle.block_on(async {
        crate::api::qr::approve_game_url(
            core,
            account,
            game_url.clone(),
            spec.mode == "approve",
            &spec.label,
        )
        .await
    });
    match outcome {
        Ok(out) => {
            if let Ok(mut states) = inner.lock() {
                let w = &mut states.states[index];
                w.scanned = true;
                w.approved = out.approved;
                w.pending_ticket = out.pending_ticket.clone();
                w.pending_token_types = out.pending_token_types.clone();
                // GUI 的结果卡/弹窗直接读这些字段，避免只看到空白的游戏与账号。
                w.app_name = out.app_name.clone();
                w.account_disp_name = out.account_disp_name.clone();
                w.risk_note = out.risk_note.clone();
            }
            crate::api::settings::record(
                core,
                account,
                "qr_race",
                true,
                &format!(
                    "{} via {} ({})",
                    if out.approved { "approved" } else { "scanned" },
                    spec.label,
                    out.app_name
                ),
                None,
            );
        }
        Err(e) => {
            if let Ok(mut states) = inner.lock() {
                states.states[index].error = Some(e.to_string());
            }
            crate::api::settings::record(
                core,
                account,
                "qr_race",
                false,
                &e.to_string(),
                e.api_retcode(),
            );
        }
    }
}

/// Snapshot one race's status. Unknown ids read as "not running".
pub fn qr_race_status(id: u32) -> RaceStatusDto {
    let map = races().lock().unwrap();
    let Some(handle) = map.get(&id) else {
        return RaceStatusDto {
            running: false,
            watchers_total: 0,
            watchers_alive: 0,
            approved: false,
            error: None,
            watchers: Vec::new(),
        };
    };
    let state = handle.inner.lock().unwrap();
    let approved = state.states.iter().any(|w| w.approved);
    let pending = state.states.iter().any(|w| w.pending_ticket.is_some());
    // `stop` is deliberately not consulted: the winner sets it the moment it
    // claims, and the GUI would then stop polling before the approval outcome
    // (approved / parked ticket / error) ever lands — that is what made the
    // race end silently, with no popup and an empty result card.
    let running = !approved && !pending && handle.alive.load(Ordering::SeqCst) > 0;
    RaceStatusDto {
        running,
        watchers_total: handle.total,
        watchers_alive: handle.alive.load(Ordering::SeqCst),
        approved,
        error: state.error.clone(),
        watchers: state.states.clone(),
    }
}

/// Signal every watcher to stand down; threads detach and finish on their
/// next frame boundary.
pub fn qr_race_stop(id: u32) -> Result<(), BridgeError> {
    let map = races().lock().unwrap();
    if let Some(handle) = map.get(&id) {
        handle.stop.store(true, Ordering::SeqCst);
    }
    Ok(())
}

/// 退出前的收尾：停掉所有竞速源与单路扫描。各源线程退出时会 drop 掉
/// FramePump，ffmpeg 子进程随之被 kill——若直接 exit(0)，Rust 析构不会
/// 运行，ffmpeg 会变成孤儿进程继续偷偷拉流。有界等待最多 ~2 秒。
pub fn app_shutdown() {
    crate::api::qr::scan_stop();
    let stops: Vec<std::sync::Arc<AtomicBool>> = {
        let map = races().lock().unwrap();
        map.values().map(|h| h.stop.clone()).collect()
    };
    for stop in &stops {
        stop.store(true, Ordering::SeqCst);
    }
    for _ in 0..40 {
        let alive = {
            let map = races().lock().unwrap();
            map.values().map(|h| h.alive.load(Ordering::SeqCst)).sum::<u32>()
        };
        if alive == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
