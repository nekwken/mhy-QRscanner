# 米游抢码器（mhy-QRscanner）

**米游直播抢码工具** — Windows desktop client, Rust protocol core with a Flutter Windows shell,
built for **抢码**:
when a streamer shows a game login QR on stream (viewer services: first scan wins), the tool
races to approve it with the operator's **own** resident passport session — Bilibili live-stream
capture and/or local screen monitoring feed the decode pipeline, and the approval
(panda → scanQRLogin → confirmQRLogin) lands far faster than any manual phone.

One MiHoYo passport account maps to one virtual device profile; after one login the session sits
resident and every future approval is a single programmatic action.

## 使用范围与免责声明

- 仅用于**本人账号、本人设备**：工具只使用你本人登录的米哈游通行证会话，只批准它自己从画面中
  读到的二维码，不接触、不代管任何他人凭证。
- **不绕过任何风控**：图形验证（极验/aigis）只会弹出官方组件由你手动完成；工具不做、也不包含
  任何验证码破解或风控规避逻辑。
- 密码仅在当次请求中使用（环境变量读入，不落盘、不写日志）；短信验证码始终由你手动输入。
- 使用本工具可能违反游戏或平台的服务条款，由此产生的账号风险与一切后果由使用者自行承担；
  本项目仅供学习与研究，请遵守所在地区的法律法规。


## Status

Core is feature-complete for the Genshin CN scan/approval path, and the Flutter shell on top of it is
packaged, tested and running. **New here? Read `docs/HANDOVER.md` first** — environment setup,
verified build commands, what is live-verified vs only unit tested, and the known traps.

- Rust workspace root: repository root (`Cargo.toml`, `crates/`); Flutter app in `app/`.
- Toolchain: Rust 1.98.1, Flutter 3.47.3, `flutter_rust_bridge` 2.13.0.
- Tests: `cargo test --workspace` **120 pass**; `flutter test` **11 pass** (13 FFI cases need the
  release cdylib; see `docs/HANDOVER.md` §4).

## What works today (live verified, own accounts only)

```powershell
# 1. virtual device
cargo run -p mhy-qrscanner-cli -- device create --account my-acct --template xiaomi14
cargo run -p mhy-qrscanner-cli -- device register --account my-acct
cargo run -p mhy-qrscanner-cli -- device show --account my-acct

# 2. account login (password → SMS fallback → session activation → token exchange)
$env:MHYQR_PASSWORD = "..."
cargo run -p mhy-qrscanner-cli -- auth login --account my-acct --login <phone>
cargo run -p mhy-qrscanner-cli -- auth show --account my-acct

# 3. approve a game QR (one-shot: URL or screenshot)
cargo run -p mhy-qrscanner-cli -- qr login-game --account my-acct --image <screenshot.png>
```

Verified end-to-end: PC-generated virtual device registers via `getExtList`/`getFp`; password and
SMS login obtain a `stoken`; `scanQRLogin` + `confirmQRLogin` make the real Genshin PC client log in.
A new-device risk challenge (`-3235`) is surfaced for **manual** completion — never bypassed.

## Desktop app

```powershell
# Builds the Rust release binaries, then the Flutter Windows app, then places
# mhy_qrscanner_bridge.dll next to mhy_QRscanner.exe (the app loads it from there).
powershell -NoProfile -ExecutionPolicy Bypass -File app\tool\build_windows.ps1
```

`app\tool\build_windows.ps1` exists because Flutter's Windows build mis-decodes this workspace's
non-ASCII path; it builds through an ASCII junction and sets `ProgramFiles(x86)`/`ProgramW6432` if the
machine lacks them. See `docs/HANDOVER.md` §4 for the manual commands and for regenerating the
`flutter_rust_bridge` bindings.

## Target Stack

- Flutter for the Windows desktop UI
- Rust for protocol, device identity, storage, orchestration, and state machines
- `flutter_rust_bridge` for the UI/core boundary via the `mhy-qrscanner-bridge` crate

## Workspace layout

```text
crates/
  mhy-qrscanner-core/     # domain models, errors
  mhy-qrscanner-device/   # virtual device generation (5 templates) + registration
  mhy-qrscanner-mihoyo/   # DS2 signing, RSA credentials, login/verify/exchange, x-rpc headers
  mhy-qrscanner-qr/       # panda + passport QR, cookie building, local QR decode
  mhy-qrscanner-capture/  # Windows GDI screen capture
  mhy-qrscanner-store/    # DPAPI-encrypted local storage
  mhy-qrscanner-cli/      # `mhyqr` binary
  mhy-qrscanner-bridge/   # Flutter FFI surface (validates, masks, forwards)
app/            # Flutter Windows shell
  lib/src/screens/  # accounts / login / qr / settings / about
  lib/src/rust/     # generated bindings — never edit by hand
  tool/build_windows.ps1
docs/
  HANDOVER.md   # start here: setup, build recipes, verification grades, traps
flutter_rust_bridge.yaml  # codegen config; run the generator from here
```

## Scope

Core race path (all implemented, live verified on Genshin CN / Star Rail CN):
- Approve a game QR as a Miyoushe mobile client — panda scan → scanQRLogin → confirmQRLogin.
- Multi-source monitoring in one race: Bilibili live-stream pull (ffmpeg) and/or local screen
  capture, per-source mode (auto-approve vs scan-then-confirm), first stable QR wins.
- Global ticket dedup: the same QR is approved at most once per 10 minutes.
- Human-in-the-loop aigis: the official Geetest widget is served in an in-app page for the user to
  complete; the tool only picks up the produced header and retries.

Supporting path:
- Password login and SMS verification-code login (aigis graphic captcha solved by the user).
- One stable virtual device profile per passport account, auto-provisioned on first login;
  raw profile management behind developer options.
- Windows DPAPI encrypted device/session storage.
- Audit log of every approval attempt (`audit.log`, plain NDJSON with 1 MiB rotation).
- Adapters for other official titles (groundwork; Star Rail approved live via the shared path).

Not pursued: generating a QR as the waiting client (the tool is always the scanner, not the
streamer's game client).

## Safety boundary

The tool approves QRs with the operator's **own** passport session only — it never touches other
people's credentials, and the account-ownership outcome is identical to a manual scan. It does
**not** automate CAPTCHA or geetest bypass (the aigis challenge is completed by the user in a
browser page), does not bypass account authorization, and does not evade platform risk controls.
Passwords and SMS codes are never persisted — the password is read from an environment variable and
used for a single request; SMS codes are read from stdin. Interactive challenges are always
completed by the user.

## Debug environment variables

Not required on the happy path:

| Variable | Purpose |
|---|---|
| `MHYQR_DEBUG` | verbose diagnostics on stderr |
| `MHYQR_PASSWORD` | password for `auth login` / `auth login-password` |
| `MHYQR_SMS_CODE` | submit an already-received SMS code without re-sending |
| `MHYQR_QR_CONFIRM=1` | legacy `qr scan`: also call `confirmQRLogin` |
| `MHYQR_APP_ID`, `MHYQR_CLIENT_TYPE`, `MHYQR_GAME_BIZ` | override `x-rpc-*` identity |
| `MHYQR_DEVICE_ID`, `MHYQR_DEVICE_FP`, `MHYQR_DEVICE_NAME`, `MHYQR_DEVICE_MODEL`, `MHYQR_SYS_VERSION` | override device identity |
| `MHYQR_BRIDGE_DLL`, `MHYQR_CLI_EXE` | test-only: tell `flutter test` where the released cdylib and CLI are |

## License

MIT — see [LICENSE](LICENSE).
