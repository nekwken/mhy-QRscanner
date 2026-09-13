use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use mhy_qrscanner_capture::{capture_virtual_screen, Frame};
use mhy_qrscanner_core::{AccountId, AccountSession, GameBiz};
use mhy_qrscanner_device::generator::{generate_profile_with_template, DeviceTemplate};
use mhy_qrscanner_mihoyo::auth::{
    challenge_from_api_error, challenge_from_entity, HttpAuthApi, LoginAccount, LoginChallenge,
    LoginChallengeKind,
};
use mhy_qrscanner_mihoyo::device_api::{HttpDeviceApi, MihoyoError};
use mhy_qrscanner_mihoyo::ds::Salt;
use mhy_qrscanner_mihoyo::RpcHeaders;
use mhy_qrscanner_qr::{
    decode_qr_file, decode_qr_luma, is_v1_qr_url, panda_host_for, panda_host_is_derived,
    panda_scan_from_v1_url, parse_v2_qr_url, passport_qr_cookie, HttpQrApi,
    PassportQrConfirmRequest, PassportQrScanRequest, StabilityFilter, DEFAULT_COOKIE_MID,
};
use mhy_qrscanner_store::file_store::EncryptedStore;
use rand::rngs::OsRng;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mhyqr")]
struct Cli {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    Qr {
        #[command(subcommand)]
        command: QrCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
}

#[derive(Subcommand)]
enum DeviceCommand {
    Create {
        #[arg(long)]
        account: String,
        /// Replace an existing profile (discards any stored registration).
        #[arg(long)]
        force: bool,
        /// Device template: xiaomi14 | samsung-s24 | oppo-find-x7 | vivo-x100 | samsung-a52
        #[arg(long, default_value = "xiaomi14")]
        template: String,
    },
    Register {
        #[arg(long)]
        account: String,
    },
    Show {
        #[arg(long)]
        account: String,
    },
}

#[derive(Subcommand)]
enum QrCommand {
    /// Scan a game/passport QR as the Miyoushe mobile client.
    Scan {
        #[arg(long)]
        url: String,
        #[arg(long)]
        account: String,
        /// panda host for V1 game QR
        #[arg(long, default_value = "https://hk4e-sdk.mihoyo.com")]
        panda_host: String,
        /// passport host for V2
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// One-shot game login: game QR (URL, image, or screen capture) → panda → scanQRLogin → confirmQRLogin.
    LoginGame {
        #[arg(long)]
        account: String,
        /// V1 game QR URL (qr_code_in_game.html)
        #[arg(long)]
        url: Option<String>,
        /// Screenshot/image file containing the game QR
        #[arg(long)]
        image: Option<PathBuf>,
        /// Capture the screen (all monitors) and decode the QR from it
        #[arg(long)]
        capture_screen: bool,
        /// Watch a Bilibili live room's stream and decode the QR from it
        /// (requires ffmpeg on PATH or MHYQR_FFMPEG; 0 wait-secs = unlimited)
        #[arg(long)]
        bilibili_room: Option<String>,
        /// With --capture-screen: keep capturing until a stable QR appears (seconds)
        #[arg(long, default_value_t = 30)]
        wait_secs: u64,
        /// Save the captured frame here (debug / audit)
        #[arg(long)]
        save_capture: Option<PathBuf>,
        /// panda host; empty = derive from profile game_biz (`<prefix>-sdk.mihoyo.com`)
        #[arg(long, default_value = "")]
        panda_host: String,
        /// passport host
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
        /// scan only; do not approve (default approves)
        #[arg(long)]
        no_confirm: bool,
    },
    /// Cancel a pending QR approval (account/ma-cn-passport/app/cancelQRLogin).
    Cancel {
        #[arg(long)]
        account: String,
        /// V2 passport QR URL (mobile.html#/login/qr?tk=...) or bare ticket
        #[arg(long)]
        url: String,
        /// token_types to send; defaults to ["1"] which matches a game QR
        #[arg(long, value_delimiter = ',', default_value = "1")]
        token_types: Vec<String>,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
}
#[derive(Subcommand)]
enum AuthCommand {
    /// Full interactive login: password (env MHYQR_PASSWORD) → SMS fallback → login_verify → exchange.
    Login {
        /// local profile label
        #[arg(long)]
        account: String,
        /// phone or email for password login; also SMS destination if numeric
        #[arg(long)]
        login: String,
        /// SMS country code (used when falling back to SMS)
        #[arg(long, default_value = "+86")]
        area_code: String,
        /// skip password even if MHYQR_PASSWORD is set
        #[arg(long)]
        sms_only: bool,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// Password login. Reads password from env MHYQR_PASSWORD (never argv).
    /// If server demands captcha/risk, prints manual instructions and exits non-zero.
    LoginPassword {
        /// local profile label
        #[arg(long)]
        account: String,
        /// miyoushe account (email/phone)
        #[arg(long)]
        login: String,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// SMS login: send captcha, then type the code on stdin (manual path).
    LoginSms {
        /// local profile label
        #[arg(long)]
        account: String,
        /// phone number
        #[arg(long)]
        login: String,
        /// country code, default +86
        #[arg(long, default_value = "+86")]
        area_code: String,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// Show stored session metadata (tokens masked).
    Show {
        #[arg(long)]
        account: String,
    },
    /// Exchange stoken for cookie account info / refresh.
    CookieRefresh {
        #[arg(long)]
        account: String,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// Exchange a game_token for passport stoken.
    GameToken {
        #[arg(long)]
        account: String,
        /// game token from game log / capture (never persist passwords)
        #[arg(long)]
        game_token: String,
        #[arg(long, default_value_t = 1)]
        account_type: i64,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// Exchange stored stoken for cookie_token (Porte exchange API).
    ExchangeCookie {
        #[arg(long)]
        account: String,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
    /// Probe session: GET getCookieAccountInfoBySToken (diagnostic).
    SessionProbe {
        #[arg(long)]
        account: String,
        #[arg(long, default_value = "https://passport-api.mihoyo.com")]
        login_host: String,
    },
}

fn data_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    let dirs = directories::ProjectDirs::from("dev", "mhy", "QRscanner")
        .context("cannot resolve project data directory")?;
    Ok(dirs.data_local_dir().to_path_buf())
}

fn profile_key(account: &str) -> String {
    format!("accounts/{account}/device-profile")
}

fn registration_key(account: &str) -> String {
    format!("accounts/{account}/device-registration")
}

fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "***".to_string();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}...{tail}")
}

/// -3101 = the server demands the Geetest graphic captcha. Run the
/// human-in-the-loop solver (official widget on a localhost page opened in the
/// user's browser) and produce the retry `x-rpc-aigis` header value. The human
/// solves; nothing is automated.
async fn solve_aigis_or_bail(challenge: &str) -> Result<String> {
    let parsed = mhy_qrscanner_mihoyo::aigis::AigisChallenge::parse(challenge)
        .context("unparseable aigis challenge header")?;
    println!(
        "需要图形验证码（极验）：gt={} risk_type={}",
        parsed.gt, parsed.risk_type
    );
    tokio::task::spawn_blocking(move || {
        mhy_qrscanner_mihoyo::aigis::solve_interactive(&parsed, std::time::Duration::from_secs(300))
    })
    .await
    .context("aigis solver join")?
    .map_err(|e| anyhow::anyhow!("aigis solve failed: {e}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = EncryptedStore::new(data_dir(cli.data_dir)?);

    match cli.command {
        Command::Device { command } => match command {
            DeviceCommand::Create {
                account,
                force,
                template,
            } => {
                let key = profile_key(&account);
                if store.load::<mhy_qrscanner_core::DeviceProfile>(&key)?.is_some() && !force {
                    bail!(
                        "device profile already exists for account={account}; pass --force to replace it (this discards any existing registration)"
                    );
                }
                let account_id = AccountId::new(account.clone())
                    .with_context(|| format!("invalid account id `{account}`"))?;
                let mut rng = OsRng;
                let tmpl = DeviceTemplate::parse(&template)
                    .with_context(|| format!("unknown template {template}"))?;
                let profile =
                    generate_profile_with_template(&account_id, GameBiz::GenshinCn, tmpl, &mut rng);
                store.save(&key, &profile)?;
                if force {
                    // Drop the stale registration only after the new profile is safely written.
                    store.delete(&registration_key(&account))?;
                }
                println!("created device profile for account={account} template={template}");
                println!("device_id={}", mask(&profile.device_id));
                println!("device_fp={}", mask(&profile.initial_device_fp));
                println!("model={}", profile.model);
                println!("android={}", profile.android_version);
            }
            DeviceCommand::Register { account } => {
                let profile = store
                    .load::<mhy_qrscanner_core::DeviceProfile>(&profile_key(&account))?
                    .context("device profile not found; run device create first")?;
                let api = HttpDeviceApi::new("https://public-data-api.mihoyo.com");
                let registration =
                    mhy_qrscanner_device::registration::register_device(&api, &store, &profile).await?;
                println!("registered device for account={account}");
                println!("device_id={}", mask(&registration.device_id));
                println!("device_fp={}", mask(&registration.device_fp));
            }
            DeviceCommand::Show { account } => {
                let profile = store
                    .load::<mhy_qrscanner_core::DeviceProfile>(&profile_key(&account))?
                    .context("device profile not found; run device create first")?;
                let registration =
                    store.load::<mhy_qrscanner_core::DeviceRegistration>(&registration_key(&account))?;

                println!("account={account}");
                println!("game_biz={}", profile.game_biz.as_str());
                println!("device_id={}", mask(&profile.device_id));
                println!(
                    "device_fp={}",
                    registration
                        .as_ref()
                        .map(|r| mask(&r.device_fp))
                        .unwrap_or_else(|| mask(&profile.initial_device_fp))
                );
                println!("model={}", profile.model);
                println!("android={}", profile.android_version);
                println!("sdk={}", profile.sdk_version);
            }
        },
        Command::Qr { command } => match command {
            QrCommand::Scan {
                url,
                account,
                panda_host,
                login_host,
            } => {
                let headers = load_rpc_headers(&store, &account)?;
                let device_id = headers.device_id.clone();
                if std::env::var("MHYQR_DEBUG").is_ok() {
                    eprintln!(
                        "qr headers app_id={} client_type={} device_id_len={}",
                        headers.app_id,
                        headers.client_type,
                        headers.device_id.len()
                    );
                }
                let api = HttpQrApi::new(panda_host, login_host);

                if is_v1_qr_url(&url) {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let req = panda_scan_from_v1_url(&url, &device_id, &headers.app_id, ts)?;
                    let data = api.panda_scan(&headers, "hk4e_cn", &req).await?;
                    println!("panda scan ok");
                    println!("app_name={}", data.app_name);
                    if !data.passport_qr_url.is_empty() {
                        println!("passport_qr_url={}", data.passport_qr_url);
                    }
                    // Game QR: panda scan only returns the passport URL; login completes
                    // via V2 scanQRLogin (+ optional confirm) on that URL.
                    if std::env::var("MHYQR_QR_CONFIRM").ok().as_deref() == Some("1") {
                        if data.passport_qr_url.is_empty() {
                            bail!("panda scan returned no passport_qr_url; cannot confirm");
                        }
                        let (ticket, token_types) = parse_v2_qr_url(&data.passport_qr_url)
                            .context("panda passport_qr_url is not parseable")?;
                        let session = store
                            .load::<AccountSession>(&session_key(&account))?
                            .context("confirm needs stored session")?;
                        let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
                        // Must scanQRLogin first (marks ticket scanned), then confirm.
                        let sreq = PassportQrScanRequest {
                            ticket: ticket.clone(),
                            token_types: token_types.clone(),
                        };
                        let sdata = api
                            .passport_scan(&headers, &Salt::Prod, &cookie, &sreq)
                            .await?;
                        println!("passport scan ok");
                        println!("app_name={}", sdata.app_name);
                        println!("account_disp_name={}", sdata.account_disp_name);
                        let creq = PassportQrConfirmRequest::approve(ticket.clone(), token_types);
                        let cdata = api
                            .passport_confirm(&headers, &Salt::Prod, &cookie, &creq)
                            .await?;
                        println!("passport confirm sent");
                        println!("confirm_data={cdata}");
                    }
                } else {
                    let (ticket, token_types) = parse_v2_qr_url(&url)
                        .context("URL is neither V1 game QR nor V2 passport QR")?;
                    let session = store
                        .load::<AccountSession>(&session_key(&account))?
                        .context("V2 QR needs stored session; run auth login-password first")?;
                    if session.stoken.is_empty() {
                        bail!("session has empty stoken");
                    }
                    let req = PassportQrScanRequest {
                        ticket: ticket.clone(),
                        token_types: token_types.clone(),
                    };
                    // Cookie mid is per-account passport user_info.mid (HAR 2026-09-11).
                    let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
                    if std::env::var("MHYQR_DEBUG").is_ok() {
                        eprintln!(
                            "passport mid={} stoken_len={}",
                            session.qr_mid(),
                            session.stoken.len()
                        );
                    }
                    let data = api
                        .passport_scan(&headers, &Salt::Prod, &cookie, &req)
                        .await?;
                    println!("passport scan ok");
                    println!("app_name={}", data.app_name);
                    println!("app_id={}", data.app_id);
                    println!("account_disp_name={}", data.account_disp_name);
                    println!("risk_note={}", data.risk_note);
                    println!("--confirm-url-ticket={}", ticket);
                    // Auto-confirm only when MHYQR_QR_CONFIRM=1 (explicit opt-in).
                    if std::env::var("MHYQR_QR_CONFIRM").ok().as_deref() == Some("1") {
                        let creq = PassportQrConfirmRequest::approve(ticket, token_types);
                        let cdata = api
                            .passport_confirm(&headers, &Salt::Prod, &cookie, &creq)
                            .await?;
                        println!("passport confirm sent");
                        println!("confirm_data={cdata}");
                    } else {
                        println!("(set MHYQR_QR_CONFIRM=1 to also call confirmQRLogin)");
                    }
                }
            }
            QrCommand::LoginGame {
                account,
                url,
                image,
                capture_screen,
                bilibili_room,
                wait_secs,
                save_capture,
                panda_host,
                login_host,
                no_confirm,
            } => {
                // Resolve the game QR URL from --url, --image, --capture-screen,
                // or --bilibili-room (local decode).
                let game_url = match (&url, image.as_deref(), capture_screen, bilibili_room.as_deref())
                {
                    (Some(u), _, _, _) => u.clone(),
                    (None, Some(path), false, _) => {
                        println!("decoding QR from image {path:?} ...");
                        let decoded = decode_qr_file(path)?;
                        println!("decoded QR len={}", decoded.len());
                        decoded
                    }
                    (None, None, true, _) => {
                        println!(
                            "capturing screen (waiting up to {wait_secs}s for a stable QR) ..."
                        );
                        let decoded = capture_until_stable(wait_secs, save_capture.as_deref())?;
                        println!("decoded QR len={}", decoded.len());
                        decoded
                    }
                    (None, None, false, Some(room)) => {
                        let decoded = bilibili_until_stable(room, wait_secs)?;
                        println!("decoded QR len={}", decoded.len());
                        decoded
                    }
                    (None, None, false, None) => bail!(
                        "pass --url <game qr>, --image <screenshot>, --capture-screen, or --bilibili-room"
                    ),
                    (None, Some(_), true, _) => {
                        bail!("--image and --capture-screen are mutually exclusive")
                    }
                };
                if !is_v1_qr_url(&game_url) {
                    bail!("not a V1 game QR URL (expects qr_code_in_game.html)");
                }

                let headers = load_rpc_headers(&store, &account)?;
                let device_id = headers.device_id.clone();
                // The panda path/host use the **game title** biz (`hk4e_cn` from the
                // device profile), not the x-rpc client biz (`bbs_cn`). Mixing them
                // yields a 404.
                let profile = store
                    .load::<mhy_qrscanner_core::DeviceProfile>(&profile_key(&account))?
                    .context("device profile not found; run device create first")?;
                let game_biz = profile.game_biz.as_str().to_string();
                // Empty --panda-host means "derive from game_biz".
                let panda_host = if panda_host.trim().is_empty() {
                    panda_host_for(&game_biz)
                } else {
                    panda_host
                };
                if std::env::var("MHYQR_DEBUG").is_ok() {
                    eprintln!(
                        "login-game game_biz={game_biz} panda_host={panda_host} derived={}",
                        panda_host_is_derived(&game_biz)
                    );
                }
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login first")?;
                if session.stoken.is_empty() {
                    bail!("session has empty stoken; run auth login first");
                }
                let api = HttpQrApi::new(panda_host, login_host);

                // 1/3 panda scan
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let preq = panda_scan_from_v1_url(&game_url, &device_id, &headers.app_id, ts)?;
                let pdata = api.panda_scan(&headers, &game_biz, &preq).await?;
                println!("1/3 panda scan ok");
                if pdata.passport_qr_url.is_empty() {
                    bail!("panda scan returned no passport_qr_url");
                }
                let (ticket, token_types) = parse_v2_qr_url(&pdata.passport_qr_url)
                    .context("panda passport_qr_url is not parseable")?;

                // 2/3 scanQRLogin (marks ticket scanned for this account)
                let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
                let sreq = PassportQrScanRequest {
                    ticket: ticket.clone(),
                    token_types: token_types.clone(),
                };
                let sdata = api
                    .passport_scan(&headers, &Salt::Prod, &cookie, &sreq)
                    .await?;
                println!("2/3 scanQRLogin ok");
                println!("app_name={}", sdata.app_name);
                println!("account={}", sdata.account_disp_name);
                if !sdata.risk_note.is_empty() {
                    println!("risk_note={}", sdata.risk_note);
                }

                // 3/3 confirmQRLogin (approve) unless --no-confirm
                if no_confirm {
                    println!("3/3 skipped confirm (--no-confirm)");
                } else {
                    let creq = PassportQrConfirmRequest::approve(ticket, token_types);
                    let _ = api
                        .passport_confirm(&headers, &Salt::Prod, &cookie, &creq)
                        .await?;
                    println!("3/3 confirmQRLogin ok");
                    println!("game login approved; check the game client");
                }
            }
            QrCommand::Cancel {
                account,
                url,
                token_types,
                login_host,
            } => {
                // Accept either a full V2 QR URL or a bare ticket.
                let (ticket, token_types) = match parse_v2_qr_url(&url) {
                    Some((tk, types)) => {
                        let types = if types.is_empty() { token_types } else { types };
                        (tk, types)
                    }
                    None => (url, token_types),
                };
                let headers = load_rpc_headers(&store, &account)?;
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login first")?;
                if session.stoken.is_empty() {
                    bail!("session has empty stoken; run auth login first");
                }
                let cookie = passport_qr_cookie(&session.stoken, session.qr_mid());
                let api = HttpQrApi::new("https://unused.invalid", login_host);
                api.passport_cancel(&headers, &Salt::Prod, &cookie, &ticket, &token_types)
                    .await?;
                println!("cancelQRLogin ok (ticket_len={})", ticket.len());
            }
        },
        Command::Auth { command } => match command {
            AuthCommand::Login {
                account,
                login,
                area_code,
                sms_only,
                login_host,
            } => {
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                let mut session: Option<AccountSession> = None;

                // 1) Password login if requested and available
                let password = if sms_only {
                    None
                } else {
                    std::env::var("MHYQR_PASSWORD").ok()
                };
                if let Some(password) = password {
                    println!("step 1/4 password login ...");
                    let attempt = api
                        .login_by_password(&headers, &Salt::Prod, &login, &password, "")
                        .await;
                    // -3101 → graphic captcha: solve in the browser, retry once.
                    let attempt = match attempt {
                        Err(MihoyoError::AigisChallenge { challenge }) => {
                            let token = solve_aigis_or_bail(&challenge).await?;
                            api.login_by_password(&headers, &Salt::Prod, &login, &password, &token)
                                .await
                        }
                        other => other,
                    };
                    match attempt {
                        Ok(data) => {
                            if let Some(a) = data.to_login_account().filter(|a| !a.token.is_empty())
                            {
                                println!("password login ok (stoken_len={})", a.token.len());
                                session = Some(new_session(a));
                            } else if let Some(ch) = challenge_from_entity(&data) {
                                print_challenge(&ch);
                                println!("falling back to SMS path ...");
                            } else {
                                println!(
                                    "password login returned no stoken; falling back to SMS ..."
                                );
                            }
                        }
                        Err(e) => {
                            if let Some(ch) = challenge_from_api_error(&e) {
                                print_challenge(&ch);
                                if matches!(
                                    ch.kind,
                                    LoginChallengeKind::InteractiveRisk
                                        | LoginChallengeKind::Unknown
                                ) {
                                    // aigis precheck may return required fields
                                    if let Ok(pre) =
                                        api.aigis_pre_check(&headers, &Salt::Prod).await
                                    {
                                        println!("aigis preCheck data={pre}");
                                    }
                                    println!("SMS fallback may still work; trying ...");
                                }
                            } else {
                                return Err(e.into());
                            }
                        }
                    }
                } else {
                    println!("step 1/4 skip password (no MHYQR_PASSWORD or --sms-only)");
                }

                // 2) SMS path: actively request captcha, user types code
                if session.is_none() {
                    println!("step 2/4 request SMS captcha for {area_code} {login} ...");
                    let attempt = api
                        .send_login_captcha(&headers, &Salt::Prod, &area_code, &login, "")
                        .await;
                    // -3101 → graphic captcha: solve in the browser, retry once.
                    let attempt = match attempt {
                        Err(MihoyoError::AigisChallenge { challenge }) => {
                            let token = solve_aigis_or_bail(&challenge).await?;
                            api.send_login_captcha(
                                &headers,
                                &Salt::Prod,
                                &area_code,
                                &login,
                                &token,
                            )
                            .await
                        }
                        other => other,
                    };
                    match attempt {
                        Ok(v) => println!("captcha requested ok; server_data={v}"),
                        Err(e) => {
                            if let Some(ch) = challenge_from_api_error(&e) {
                                print_challenge(&ch);
                            }
                            bail!("failed to request SMS captcha: {e}");
                        }
                    }
                    println!("Enter the SMS code from your phone, then press Enter:");
                    let mut code = String::new();
                    std::io::stdin()
                        .read_line(&mut code)
                        .context("read SMS code from stdin")?;
                    let code = code.trim().to_string();
                    if code.is_empty() {
                        bail!("empty SMS code");
                    }
                    println!("step 3/4 submit SMS login ...");
                    let data = api
                        .login_by_mobile_captcha(&headers, &Salt::Prod, &area_code, &login, &code)
                        .await
                        .inspect_err(|e| {
                            if let Some(ch) = challenge_from_api_error(e) {
                                print_challenge(&ch);
                            }
                        })?;
                    let a = data
                        .to_login_account()
                        .filter(|a| !a.token.is_empty())
                        .context("SMS login returned no stoken")?;
                    println!("SMS login ok (stoken_len={})", a.token.len());
                    session = Some(new_session(a));
                }

                let mut session = session.context("no session obtained")?;

                // 3) login_verify (Porte session activation)
                println!("step 3/4 login_verify ...");
                match api
                    .login_verify(&headers, &Salt::Prod, &session.mid, &session.stoken)
                    .await
                {
                    Ok(v) => {
                        println!("login_verify ok");
                        if let Some(a) = v.to_login_account() {
                            if !a.token.is_empty() && a.token != session.stoken {
                                session.stoken = a.token;
                                if !a.mid.is_empty() {
                                    session.mid = a.mid;
                                }
                                println!("session refreshed from login_verify");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("login_verify failed (non-fatal): {e}");
                        if let Some(ch) = challenge_from_api_error(&e) {
                            print_challenge(&ch);
                        }
                    }
                }

                // 4) exchange cookie_token (4) + ltoken (2) as App does
                println!("step 4/4 exchange cookie_token + ltoken ...");
                match api
                    .exchange_token(&headers, &Salt::Prod, &session.stoken, &session.mid, 4)
                    .await
                {
                    Ok(v) => {
                        if let Some(ct) = v
                            .get("token")
                            .and_then(|t| t.get("token"))
                            .and_then(|x| x.as_str())
                        {
                            session.cookie_token = ct.to_string();
                            println!("cookie_token_len={}", session.cookie_token.len());
                        }
                        if let Some(mid) = v
                            .get("user_info")
                            .and_then(|u| u.get("mid"))
                            .and_then(|x| x.as_str())
                        {
                            if !mid.is_empty() {
                                session.mid = mid.to_string();
                            }
                        }
                    }
                    Err(e) => eprintln!("exchange cookie_token failed (non-fatal): {e}"),
                }
                match api
                    .exchange_token(&headers, &Salt::Prod, &session.stoken, &session.mid, 2)
                    .await
                {
                    Ok(v) => {
                        if let Some(lt) = v
                            .get("token")
                            .and_then(|t| t.get("token"))
                            .and_then(|x| x.as_str())
                        {
                            session.ltoken = lt.to_string();
                            println!("ltoken_len={}", session.ltoken.len());
                        }
                    }
                    Err(e) => eprintln!("exchange ltoken failed (non-fatal): {e}"),
                }

                session.updated_at = chrono_like_now();
                store.save(&session_key(&account), &session)?;
                println!("session stored account={account}");
                println!("uid_len={}", session.account_uid.len());
                println!("stoken_len={}", session.stoken.len());
                println!("mid={} (use this in QR Cookie)", session.qr_mid());
                println!("cookie_token_len={}", session.cookie_token.len());
                println!("ltoken_len={}", session.ltoken.len());
            }
            AuthCommand::LoginPassword {
                account,
                login,
                login_host,
            } => {
                let password = std::env::var("MHYQR_PASSWORD")
                    .context("set MHYQR_PASSWORD env var (do not pass password on argv)")?;
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                let attempt = api
                    .login_by_password(&headers, &Salt::Prod, &login, &password, "")
                    .await;
                // -3101 → graphic captcha: solve in the browser, retry once.
                let attempt = match attempt {
                    Err(MihoyoError::AigisChallenge { challenge }) => {
                        let token = solve_aigis_or_bail(&challenge).await?;
                        api.login_by_password(&headers, &Salt::Prod, &login, &password, &token)
                            .await
                    }
                    other => other,
                };
                let data = match attempt {
                    Ok(d) => d,
                    Err(e) => {
                        if let Some(ch) = challenge_from_api_error(&e) {
                            print_challenge(&ch);
                            bail!(
                                "login rejected by server retcode={} ({})",
                                ch.retcode,
                                ch.message
                            );
                        }
                        Err(e)?
                    }
                };
                if let Some(ch) = challenge_from_entity(&data) {
                    print_challenge(&ch);
                    bail!("login requires manual challenge ({:?})", ch.kind);
                }
                let acct = match data.to_login_account() {
                    Some(a) if !a.token.is_empty() => a,
                    _ => {
                        let keys: Vec<String> = data
                            .rest
                            .as_object()
                            .map(|o| o.keys().cloned().collect())
                            .unwrap_or_default();
                        let has_token = data
                            .token
                            .as_ref()
                            .map(|t| !t.token.is_empty())
                            .unwrap_or(false);
                        bail!(
                            "login response has no usable stoken (has_token={has_token}, data_keys={keys:?})"
                        );
                    }
                };
                let session = AccountSession {
                    account_uid: acct.uid,
                    stoken: acct.token,
                    mid: acct.mid,
                    cookie_mid: DEFAULT_COOKIE_MID.to_string(),
                    stoken_v2: String::new(),
                    ltoken: String::new(),
                    cookie_token: String::new(),
                    updated_at: chrono_like_now(),
                };
                store.save(&session_key(&account), &session)?;
                println!("login ok; session stored for account={account}");
                println!("uid_len={}", session.account_uid.len());
                println!("stoken_len={}", session.stoken.len());
                println!("mid_len={}", session.mid.len());
                println!("cookie_mid={}", session.cookie_mid);
                // Porte always runs loginVerify after STOKEN password login.
                match api
                    .login_verify(&headers, &Salt::Prod, &session.mid, &session.stoken)
                    .await
                {
                    Ok(v) => {
                        println!("login_verify ok");
                        if let Some(a) = v.to_login_account() {
                            if !a.token.is_empty() && a.token != session.stoken {
                                let mut session = session;
                                session.stoken = a.token;
                                if !a.mid.is_empty() {
                                    session.mid = a.mid;
                                }
                                session.updated_at = chrono_like_now();
                                store.save(&session_key(&account), &session)?;
                                println!("session refreshed from login_verify");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("login_verify failed (non-fatal for now): {e}");
                        if let Some(ch) = challenge_from_api_error(&e) {
                            print_challenge(&ch);
                        }
                    }
                }
            }
            AuthCommand::LoginSms {
                account,
                login,
                area_code,
                login_host,
            } => {
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host.clone());
                // If MHYQR_SMS_CODE is set, skip send (reuse a just-received code).
                let code_from_env = std::env::var("MHYQR_SMS_CODE").ok();
                let code = if let Some(c) = code_from_env {
                    let c = c.trim().to_string();
                    if c.is_empty() {
                        bail!("MHYQR_SMS_CODE is empty");
                    }
                    println!("using SMS code from MHYQR_SMS_CODE (len={})", c.len());
                    c
                } else {
                    println!("sending SMS captcha to {area_code} {login} ...");
                    let attempt = api
                        .send_login_captcha(&headers, &Salt::Prod, &area_code, &login, "")
                        .await;
                    // -3101 → graphic captcha: solve in the browser, retry once.
                    let attempt = match attempt {
                        Err(MihoyoError::AigisChallenge { challenge }) => {
                            let token = solve_aigis_or_bail(&challenge).await?;
                            api.send_login_captcha(
                                &headers,
                                &Salt::Prod,
                                &area_code,
                                &login,
                                &token,
                            )
                            .await
                        }
                        other => other,
                    };
                    match attempt {
                        Ok(v) => println!("captcha request accepted; data={v}"),
                        Err(e) => {
                            if let Some(ch) = challenge_from_api_error(&e) {
                                print_challenge(&ch);
                            }
                            Err(e)?
                        }
                    }
                    println!("Type the SMS code and press Enter (manual step):");
                    let mut code = String::new();
                    std::io::stdin()
                        .read_line(&mut code)
                        .context("failed to read SMS code from stdin")?;
                    let code = code.trim().to_string();
                    if code.is_empty() {
                        bail!("empty SMS code");
                    }
                    code
                };
                println!("submitting SMS login ...");
                let data = api
                    .login_by_mobile_captcha(&headers, &Salt::Prod, &area_code, &login, &code)
                    .await
                    .inspect_err(|e| {
                        if let Some(ch) = challenge_from_api_error(e) {
                            print_challenge(&ch);
                        }
                    })?;
                if let Some(ch) = challenge_from_entity(&data) {
                    print_challenge(&ch);
                    bail!("SMS login requires further manual challenge");
                }
                let acct = data
                    .to_login_account()
                    .filter(|a| !a.token.is_empty())
                    .context("SMS login returned no stoken")?;
                let mut session = new_session(acct);
                // login_verify + exchange
                match api
                    .login_verify(&headers, &Salt::Prod, &session.mid, &session.stoken)
                    .await
                {
                    Ok(v) => {
                        println!("login_verify ok");
                        if let Some(a) = v.to_login_account() {
                            if !a.token.is_empty() && a.token != session.stoken {
                                session.stoken = a.token;
                                if !a.mid.is_empty() {
                                    session.mid = a.mid;
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("login_verify failed (non-fatal): {e}"),
                }
                match api
                    .exchange_token(&headers, &Salt::Prod, &session.stoken, &session.mid, 4)
                    .await
                {
                    Ok(v) => {
                        if let Some(ct) = v
                            .get("token")
                            .and_then(|t| t.get("token"))
                            .and_then(|x| x.as_str())
                        {
                            session.cookie_token = ct.to_string();
                        }
                    }
                    Err(e) => eprintln!("exchange cookie_token failed: {e}"),
                }
                match api
                    .exchange_token(&headers, &Salt::Prod, &session.stoken, &session.mid, 2)
                    .await
                {
                    Ok(v) => {
                        if let Some(lt) = v
                            .get("token")
                            .and_then(|t| t.get("token"))
                            .and_then(|x| x.as_str())
                        {
                            session.ltoken = lt.to_string();
                        }
                    }
                    Err(e) => eprintln!("exchange ltoken failed: {e}"),
                }
                session.updated_at = chrono_like_now();
                store.save(&session_key(&account), &session)?;
                println!("SMS login ok; session stored for account={account}");
                println!("stoken_len={}", session.stoken.len());
                println!("mid={}", session.qr_mid());
            }
            AuthCommand::Show { account } => {
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login-password first")?;
                println!("account={account}");
                println!("uid_len={}", session.account_uid.len());
                println!("stoken_len={}", session.stoken.len());
                println!(
                    "stoken_prefix={}",
                    session.stoken.chars().take(3).collect::<String>()
                );
                println!("mid_len={}", session.mid.len());
                println!("cookie_mid={}", session.cookie_mid);
                println!("updated_at={}", session.updated_at);
            }
            AuthCommand::CookieRefresh {
                account,
                login_host,
            } => {
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login-password first")?;
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                // App POST getTokenBySToken with Cookie: stuid=...;stoken=...
                let data = api
                    .get_token_by_stoken(
                        &headers,
                        &Salt::Prod,
                        &session.account_uid,
                        &session.stoken,
                    )
                    .await?;
                let acct = data.to_login_account().unwrap_or_default();
                let mut session = session;
                if !acct.token.is_empty() {
                    session.stoken = acct.token;
                }
                if !acct.mid.is_empty() {
                    session.mid = acct.mid;
                }
                if !acct.uid.is_empty() {
                    session.account_uid = acct.uid;
                }
                if let Some(lt) = data
                    .rest
                    .get("token")
                    .and_then(|t| t.get("token"))
                    .and_then(|v| v.as_str())
                {
                    // some responses nest new token under token.token
                    if session.stoken.is_empty() {
                        session.stoken = lt.to_string();
                    }
                }
                session.updated_at = chrono_like_now();
                store.save(&session_key(&account), &session)?;
                println!("stoken refresh ok");
                println!("uid_len={}", session.account_uid.len());
                println!("stoken_len={}", session.stoken.len());
                println!("mid_len={}", session.mid.len());
            }
            AuthCommand::GameToken {
                account,
                game_token,
                account_type,
                login_host,
            } => {
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                let data = api
                    .get_token_by_game_token(&headers, &Salt::Prod, &game_token, account_type)
                    .await?;
                let acct = data
                    .to_login_account()
                    .filter(|a| !a.token.is_empty())
                    .context("game_token exchange returned no stoken")?;
                let session = AccountSession {
                    account_uid: acct.uid,
                    stoken: acct.token,
                    mid: acct.mid,
                    cookie_mid: DEFAULT_COOKIE_MID.to_string(),
                    stoken_v2: String::new(),
                    ltoken: String::new(),
                    cookie_token: String::new(),
                    updated_at: chrono_like_now(),
                };
                store.save(&session_key(&account), &session)?;
                println!("game_token exchange ok; session stored");
                println!("uid_len={}", session.account_uid.len());
                println!("stoken_len={}", session.stoken.len());
            }
            AuthCommand::ExchangeCookie {
                account,
                login_host,
            } => {
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login-password first")?;
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                let data = api
                    .exchange_stoken_for_cookie_token(
                        &headers,
                        &Salt::Prod,
                        &session.stoken,
                        &session.mid,
                    )
                    .await?;
                // TokenExchangeEntity typically has token / user_info or cookie_token fields
                if let Some(ct) = data
                    .get("token")
                    .and_then(|t| t.get("token"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        data.get("cookie_token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                {
                    let mut session = session;
                    session.cookie_token = ct;
                    if let Some(mid) = data
                        .get("user_info")
                        .and_then(|u| u.get("mid"))
                        .and_then(|v| v.as_str())
                    {
                        session.mid = mid.to_string();
                    }
                    session.updated_at = chrono_like_now();
                    store.save(&session_key(&account), &session)?;
                    println!("exchange ok");
                    println!("cookie_token_len={}", session.cookie_token.len());
                    println!("mid_len={}", session.mid.len());
                } else {
                    let keys: Vec<String> = data
                        .as_object()
                        .map(|o| o.keys().cloned().collect())
                        .unwrap_or_default();
                    bail!("exchange response missing token; data_keys={keys:?} data={data}");
                }
            }
            AuthCommand::SessionProbe {
                account,
                login_host,
            } => {
                let session = store
                    .load::<AccountSession>(&session_key(&account))?
                    .context("no session; run auth login-password first")?;
                let headers = load_rpc_headers(&store, &account)?;
                let api = HttpAuthApi::new(login_host);
                let result = api
                    .get_cookie_account_info_by_stoken(
                        &headers,
                        &Salt::Prod,
                        &session.stoken,
                        &session.mid,
                    )
                    .await;
                match result {
                    Ok(data) => {
                        println!("session probe ok");
                        if let Some(a) = data.to_login_account() {
                            println!("uid_len={}", a.uid.len());
                            println!("stoken_len={}", a.token.len());
                            println!("mid_len={}", a.mid.len());
                        } else {
                            println!(
                                "data_keys={:?}",
                                data.rest
                                    .as_object()
                                    .map(|o| o.keys().cloned().collect::<Vec<_>>())
                            );
                        }
                    }
                    Err(e) => {
                        if let Some(ch) = challenge_from_api_error(&e) {
                            print_challenge(&ch);
                        }
                        Err(e)?
                    }
                }
            }
        },
    }

    Ok(())
}

fn load_rpc_headers(store: &EncryptedStore, account: &str) -> Result<RpcHeaders> {
    let profile = store
        .load::<mhy_qrscanner_core::DeviceProfile>(&profile_key(account))?
        .context("device profile not found; run device create first")?;
    let registration = store.load::<mhy_qrscanner_core::DeviceRegistration>(&registration_key(account))?;
    let device_fp = registration
        .as_ref()
        .map(|r| r.device_fp.clone())
        .unwrap_or_else(|| profile.initial_device_fp.clone());
    // Overrides for non-mys clients (e.g. cloud genshin: app_id=c76ync6mutq8, client_type=22)
    let client_type = std::env::var("MHYQR_CLIENT_TYPE")
        .unwrap_or_else(|_| profile.client_profile.client_type.clone());
    let app_id =
        std::env::var("MHYQR_APP_ID").unwrap_or_else(|_| profile.client_profile.app_id.clone());
    let game_biz =
        std::env::var("MHYQR_GAME_BIZ").unwrap_or_else(|_| profile.client_profile.game_biz.clone());
    let device_id = std::env::var("MHYQR_DEVICE_ID").unwrap_or_else(|_| profile.device_id.clone());
    let device_fp = std::env::var("MHYQR_DEVICE_FP").unwrap_or(device_fp);
    let device_name =
        std::env::var("MHYQR_DEVICE_NAME").unwrap_or_else(|_| profile.device_name.clone());
    let device_model = std::env::var("MHYQR_DEVICE_MODEL").unwrap_or_else(|_| profile.model.clone());
    let sys_version =
        std::env::var("MHYQR_SYS_VERSION").unwrap_or_else(|_| profile.android_version.clone());
    Ok(RpcHeaders {
        app_id,
        client_type,
        device_id,
        device_fp,
        device_name,
        device_model,
        sys_version,
        game_biz,
        app_version: profile.client_profile.app_version.clone(),
        sdk_version: profile.client_profile.sdk_version.clone(),
        lifecycle_id: uuid_like(),
        account_version: profile.client_profile.sdk_version.clone(),
    })
}

fn session_key(account: &str) -> String {
    format!("accounts/{account}/session")
}

fn save_frame(frame: &Frame, path: &std::path::Path) -> Result<()> {
    frame
        .save_png(path)
        .with_context(|| format!("failed to save capture to {}", path.display()))
}

/// Capture the screen until the same QR payload is decoded twice in a row.
///
/// Requires two identical observations so a half-drawn or stale frame cannot be
/// mistaken for the game QR. Gives up after `timeout_secs`.
/// Watch a Bilibili room's live stream (via ffmpeg) until a stable QR decodes.
/// `wait_secs == 0` means unlimited.
fn bilibili_until_stable(room_id: &str, wait_secs: u64) -> Result<String> {
    use std::time::Duration;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let stream = mhy_qrscanner_live::resolve_play_url(room_id, "https://api.live.bilibili.com", &client)
        .map_err(anyhow::Error::from)?;
    println!(
        "room {} stream ready ({}), pumping frames via ffmpeg ...",
        stream.room_id, stream.format
    );
    let ffmpeg = std::env::var("MHYQR_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string());
    let mut pump = mhy_qrscanner_live::FramePump::start(
        &ffmpeg,
        &stream.url,
        1280,
        720,
        2,
        "https://live.bilibili.com/",
    )
    .map_err(anyhow::Error::from)?;
    let mut filter = StabilityFilter::new(2);
    let deadline = (wait_secs > 0)
        .then(|| std::time::Instant::now() + std::time::Duration::from_secs(wait_secs));
    loop {
        if let Some(deadline) = deadline {
            if std::time::Instant::now() >= deadline {
                bail!("no stable QR appeared within {wait_secs}s");
            }
        }
        match pump.read_frame() {
            Some(frame) => {
                if let Some(img) = image::GrayImage::from_raw(frame.width, frame.height, frame.data)
                {
                    if let Ok(payload) = decode_qr_luma(img) {
                        if filter.observe(&payload) {
                            return Ok(payload);
                        }
                    } else {
                        filter.miss();
                    }
                }
            }
            None => bail!("live stream ended before a QR appeared"),
        }
    }
}

fn capture_until_stable(
    timeout_secs: u64,
    save_capture: Option<&std::path::Path>,
) -> Result<String> {
    let mut filter = StabilityFilter::new(2);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut attempts = 0u32;
    let mut last_error: Option<String> = None;

    loop {
        attempts += 1;
        let started = std::time::Instant::now();
        let frame = capture_virtual_screen().context("screen capture failed")?;
        if attempts == 1 {
            println!(
                "frame {}x{} from {} ({} bytes)",
                frame.width,
                frame.height,
                frame.source_id,
                frame.byte_len()
            );
            if let Some(out) = save_capture {
                save_frame(&frame, out)?;
                println!("saved capture to {}", out.display());
            }
        }

        let luma = frame.to_luma8();
        let decoded = decode_qr_luma(luma);
        if std::env::var("MHYQR_DEBUG").is_ok() {
            eprintln!(
                "attempt {attempts}: capture+decode in {} ms",
                started.elapsed().as_millis()
            );
        }
        match decoded {
            Ok(payload) => {
                if filter.observe(&payload) {
                    return Ok(payload);
                }
                println!("seen a QR ({}/2 identical frames) ...", filter.hits());
            }
            Err(e) => {
                filter.miss();
                last_error = Some(e.to_string());
            }
        }

        if std::time::Instant::now() >= deadline {
            bail!(
                "no stable QR after {attempts} capture(s) in {timeout_secs}s{}; open the game login QR and retry",
                last_error
                    .map(|e| format!(" (last decode error: {e})"))
                    .unwrap_or_default()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
    }
}

fn new_session(acct: LoginAccount) -> AccountSession {
    AccountSession {
        account_uid: acct.uid,
        stoken: acct.token,
        mid: acct.mid,
        cookie_mid: DEFAULT_COOKIE_MID.to_string(),
        stoken_v2: String::new(),
        ltoken: String::new(),
        cookie_token: String::new(),
        updated_at: chrono_like_now(),
    }
}

fn print_challenge(ch: &LoginChallenge) {
    eprintln!();
    eprintln!("=== MANUAL CHALLENGE REQUIRED (no auto-bypass) ===");
    eprintln!("kind={:?} retcode={}", ch.kind, ch.retcode);
    eprintln!("message={}", ch.message);
    if !ch.hints.is_empty() {
        eprintln!("hints={:?}", ch.hints);
    }
    eprintln!("{}", ch.manual_instructions());
    eprintln!("=================================================");
}

fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn uuid_like() -> String {
    let mut rng = OsRng;
    use rand::Rng;
    let hex: String = (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16u8)))
        .collect();
    format!(
        "{}-{}-4{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}
