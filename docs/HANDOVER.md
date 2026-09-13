# Handover

Everything the next person needs to pick this up cold: what exists, how to build and run it,
what is actually verified, what is not, and which traps will otherwise cost hours.

This file is the operational companion to the root `README.md` (what the project is, usage
scope and quick start).

> Last updated: 2026-09-11, after the Flutter-shell follow-ups (packaging script, FFI end-to-end
> tests, Settings screen, generated-bindings config).

## 1. What this is

A **抢码工具** — a race approver for game login QRs shown on live streams. Streamers put a game
login QR on screen (viewer services: first scan wins); viewers race to approve with their own
Miyoushe, and the winner's account is the one that gets logged in and played. This tool makes the
operator win: a passport session resident on a virtual device, plus screen monitoring (and later a
direct Bilibili live-stream pull) that fires panda → scanQRLogin → confirmQRLogin the instant the
QR appears — faster than any manual phone.

Underneath it is also a faithful implementation of the Miyoushe mobile login protocol (password,
SMS, aigis challenge, session activation, token exchange), which is what makes the resident session
possible. Genshin CN (`hk4e_cn`) is the verified title; four more are wired but unverified.

The deliverable is `repo/`: a Rust workspace with a Flutter Windows UI on top.

### The race flow (core scenario, how to drive it)

1. 登录页：输入米哈游通行证账号（密码或短信，遇图形验证在应用内弹窗完成）。
   勾选「重启后保留登录态」则重启自动恢复该账号并探针检测登录态有效性。
2. 扫码页 → **源**：所有监控源各一张卡片——「本机屏幕」与各直播间并列，各自启用/模式
   （扫描并批准=自动批准、仅扫描=手动批准弹窗），卡片带来源标签、实时帧率与逐帧解码提示；
   「开始捕获」并发监控所有已启用来源，第一个稳定二维码按其模式行动。页面顶部的运行条
   汇总所有在跑的源，切页签也不会丢失谁在跑。
3. 扫码页 → **B站直播**：只登记直播间（房间号 + 标签 + 添加）；启用、模式与删除都在「源」
   页签的卡片上。直播拉流需要 ffmpeg 在 PATH（或 `MHYQR_FFMPEG`）。
   **截图文件 / 二维码链接**：一次性单路扫描（仅扫描 / 扫描并批准）。
   弹窗（批准窗口/成功通知）受设置开关控制。
4. 会话过期：登录页自动探针显示「登录态已失效」，重新登录即可；设备档案自动维护，
   手动管理在开发者选项。
5. 界面说明文字由设置里的「指引」开关控制（默认关闭）：关闭时只显示操作与状态，
   打开后出现各页说明段落。

### Safety boundary — these are not negotiable

The whole design assumes the operator is logging into **their own** account on **their own**
devices. Do not change these:

- Never persist a password. It is a function argument used for one request.
- Never automate an SMS code. The server sends it; the user types it.
- **Never automate or bypass CAPTCHA / geetest / aigis / risk controls.** A challenge is surfaced to
  the user as `BridgeError::Challenge` with manual instructions. `dto.rs` contains an assertion that
  the risk instruction text never promises an automatic bypass — keep it.
- Never write credentials, tokens, cookies, SMS codes, or replayable payloads into the
  repository. Field names, shapes and lengths only; raw captures and analysis notes belong
  outside the public repository.

## 2. Where things live

| Path | What it is |
|---|---|
| `repo/` | The publishable git repository. Workspace root: `Cargo.toml`, `crates/`, `docs/`, `app/`. |
| `_archive/` | Superseded Python and Android implementations, plus reference projects. Kept for traceability (the Python version is the behavioural reference for device fingerprints). |

## 3. Environment setup on a new machine

1. **Rust** — stable toolchain, `rustc`/`cargo` ≥ 1.88 (the workspace MSRV). MSVC linker required
   (Visual Studio Build Tools or VS Community with the C++ workload).
2. **Flutter** — 3.47.x stable with Windows desktop enabled. Here it is at `C:\flutter`
   (`C:\flutter\bin` is on the user PATH; a non-interactive shell may need it added explicitly).
3. **`flutter_rust_bridge_codegen` 2.13.0** — `cargo install flutter_rust_bridge_codegen --version 2.13.0`.
   It **must** match the `flutter_rust_bridge` crate version exactly; a mismatch corrupts the
   generated bindings.
4. **Two environment variables that are missing on this machine** and break Flutter's build:
   `ProgramFiles(x86)` and `ProgramW6432`. Without them `vswhere.exe` discovery fails and
   `flutter doctor`/`flutter build` error out. `app/tool/build_windows.ps1` sets them for its own
   process if absent; set them at user scope for interactive work.
5. **ASCII build path.** Flutter's Windows build mis-decodes a non-ASCII project path
   (`E:\抢码工具\...` reaches MSBuild as GBK mojibake) and dies reading `app.dill`. Build through a
   junction; `build_windows.ps1` creates `C:\mhy-qrscanner` automatically. Sources stay in the repo.
6. **Git ownership.** The repo is owned by a different Windows account than the one you run as, so
   plain `git -C E:\抢码工具\repo ...` fails with *dubious ownership*. Either pass
   `-c safe.directory='E:/抢码工具/repo'` per command or add that one path to the user's git config.
   Do not add a blanket exception for the whole drive.

## 4. Build, run, test

All of these are verified working as written.

### Package the desktop app

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File repo\app\tool\build_windows.ps1
```

This builds `mhy-qrscanner-bridge` + `mhy-qrscanner-cli` in release, runs `flutter pub get` and
`flutter build windows --release` **through the ASCII junction**, then copies `mhy_qrscanner_bridge.dll` next
to `mhy_QRscanner.exe`. Flags: `-SkipRust`, `-SkipFlutter`, `-Flutter <path to flutter.bat>`,
`-AsciiPath <junction>`. The app loads the DLL from beside the executable, so a bare
`flutter build windows` produces a binary that opens on an error screen — use the script.

Result: `repo/app/build/windows/x64/runner/Release/mhy_QRscanner.exe` (~90 KB) plus
`mhy_qrscanner_bridge.dll` (~5.9 MB).

### Rust

```powershell
cd repo
cargo test --workspace                                    # 98 tests, 23 targets
cargo clippy --workspace --all-targets -- -D warnings      # must stay clean
cargo fmt --all -- --check
cargo build --release -p mhy-qrscanner-mihoyo -p mhy-qrscanner-qr -p mhy-qrscanner-device -p mhy-qrscanner-store -p mhy-qrscanner-core -p mhy-qrscanner-cli
```

### Flutter / Dart

```powershell
cd repo\app
flutter test        # 19 tests; FFI groups skip themselves when the dll is absent
```

The FFI tests need the release cdylib and (for the storage-interop group) the release CLI. Point them
at explicit paths when the working directory cannot reach `target/release` — e.g. when running
through the junction:

```powershell
$env:MHYQR_BRIDGE_DLL = 'E:\抢码工具\repo\target\release\mhy_qrscanner_bridge.dll'
$env:MHYQR_CLI_EXE    = 'E:\抢码工具\repo\target\release\mhyqr.exe'
flutter test
```

Without those variables the search order is `$MHYQR_BRIDGE_DLL` → beside the executable → the
workspace `target/release` (walking up from the CWD). `MHYQR_CLI_EXE` falls back to the directory of
the resolved DLL.

### Regenerate the Flutter ↔ Rust bindings

After changing anything public under `crates/mhy-qrscanner-bridge/src/api`:

```powershell
cd repo
flutter_rust_bridge_codegen generate      # reads ./flutter_rust_bridge.yaml
```

`flutter` must be on PATH for this (the generator shells out to `flutter --version`). The generated
files are committed and must never be hand-edited: `crates/mhy-qrscanner-bridge/src/frb_generated.rs` plus
everything under `app/lib/src/rust/` (`frb_generated.dart`, `frb_generated.io.dart`,
`frb_generated.web.dart`, `api/`, `dto.dart`, `error.dart`).

**After regenerating, rebuild the DLL before running tests.** The bridge embeds a content hash; a
stale DLL fails with *"Content hash on Dart side … is different from Rust side"*. That error means
"rebuild", not "your code is broken".

### Run the CLI

```powershell
repo\target\release\mhyqr.exe --help
repo\target\release\mhyqr.exe --data-dir <dir> device create --account acct-1 --template xiaomi14
repo\target\release\mhyqr.exe --data-dir <dir> qr login-game --account acct-1 --capture-screen --wait-secs 30
```

`--data-dir` is global; omit it to use `%LOCALAPPDATA%\mhy\QRscanner` (what the GUI uses).
`--data-dir` is how the storage-interop tests prove the CLI and the shell share one store.

## 5. Architecture map

| Crate | Responsibility | Start here |
|---|---|---|
| `mhy-qrscanner-core` | Domain types: `AccountId`, `GameBiz`, `ClientProfile`, `DeviceProfile`, `DeviceRegistration`, `AccountSession` | `src/model.rs` |
| `mhy-qrscanner-device` | Virtual device generation (5 templates) and the `getFp` registration flow | `src/generator.rs`, `src/registration.rs` |
| `mhy-qrscanner-mihoyo` | DS2 signing, RSA credentials, password/SMS login, `login_verify`, token exchange, challenge classification, shared `x-rpc-*` headers | `src/ds.rs`, `src/auth.rs`, `src/headers.rs` |
| `mhy-qrscanner-qr` | panda + passport QR calls, cookie building, local QR decoding, frame stability filter | `src/lib.rs`, `src/decode.rs`, `src/stability.rs` |
| `mhy-qrscanner-capture` | Win32 GDI screen capture → `Frame` | `src/windows.rs`, `src/frame.rs` |
| `mhy-qrscanner-store` | DPAPI-encrypted keyed blob store | `src/file_store.rs`, `src/dpapi.rs` |
| `mhy-qrscanner-cli` | The `mhyqr` binary | `src/main.rs` |
| `mhy-qrscanner-bridge` | Flutter FFI surface: validates, masks, forwards. No protocol logic. | `src/api/` |

Flutter app (`app/`): `lib/main.dart` is the `NavigationRail` shell (账户 / 登录 / 扫码 / 设置 / 关于);
`lib/src/app_state.dart` holds the core handle and the selected account label; screens live in
`lib/src/screens/`; `lib/src/rust/` is generated. `lib/src/rust_loader.dart` resolves the cdylib and
`initRust()` is idempotent.

Storage layout under the data directory: `accounts/<label>/device-profile.bin`,
`accounts/<label>/device-registration.bin`, `accounts/<label>/session.bin`,
`settings/approval/<label>.bin`, and `audit.log` (NDJSON, plain text by design).

### Protocol facts that took the longest to establish

Recorded here because getting any of them wrong produces a confusing failure, not an obvious one.

| Item | Value | Wrong value looks like |
|---|---|---|
| `x-rpc-app_id` | `bll8iq97cem8` (Miyoushe Android 2.113.1) | using the cloud app id `c76ync6mutq8` → rejected |
| `x-rpc-client_type` | `2` | — |
| Cookie | `stoken=v2_…;mid=<per-account user_info.mid>` | `-100` |
| panda host + path | `hk4e-sdk.mihoyo.com` + `{game_biz}/combo/panda/qrcode/scan` | **404** if the biz is wrong |
| panda body | `passport_app_id` (string), `ticket`, `app_id` (**number** 4), `device` (16 hex), `ts` | — |
| approval order | panda scan → `scanQRLogin` → `confirmQRLogin` (`confirm:true`) | confirm fails if the middle call is skipped |
| SMS login body | RSA `area_code`/`mobile`, plain `captcha`, `action_type:"login_by_mobile_captcha"` | — |
| session activation | `account/ma-cn-session/app/verify` with `{mid, token:{token,token_type:1}, refresh:true}` | — |
| token exchange | `dst_token_type` 4 (cookie_token) then 2 (ltoken) | — |
| DS2 | `md5("salt={SALT}&t={t}&r={r}&b={b}&q=")`, DS = `t,r,sign` | — |
| SMS: request | `verifier/createLoginCaptcha` with RSA `area_code`+`mobile` only, plus an **empty** `x-rpc-aigis` header | — |
| SMS: submit | `loginByMobileCaptcha` adds plaintext `captcha` and `action_type` as a **string**; no `x-rpc-aigis` | — |
| `-3235` | new-device check, solved by requesting an SMS code and entering it | reported as interactive risk → the user is sent to a web risk check that cannot help |
| `-3503` | inconsistent device identity; **not** solvable by SMS | — |

**The trap worth memorising:** two different `game_biz` values coexist. The `x-rpc-game_biz` *header*
is the client biz (`bbs_cn`); the panda **URL path** prefix is the *title* biz (`hk4e_cn`). Mixing
them yields a 404. A mock-server test in `mhy-qrscanner-bridge` caught this; the CLI had the same bug.

**The second trap:** the cookie `mid` is **per account** (`user_info.mid`, e.g. `03pmu45q7y_mhy`), not
the global constant. A hard-coded mid produced `-100` on the second account. Encoded as
`AccountSession::qr_mid()` with a unit test.

**The third trap (fixed 2026-09-11):** `-3235` is the *new-device* check, and the remedy the server
wants is an SMS code — the live run proved it. Classifying it as interactive risk told the user to go
and finish a web risk check, which dead-ends a flow that works. `-3503` is the opposite case
(inconsistent device identity) and genuinely is not SMS-solvable, so the two retcodes must not share a
branch. Distinct from both: when the server rejects with `风险` in the message, that is text, not
semantics — classify on the retcode first.

## 6. What is verified, and how

Verification grades used below: **live** = a real request against the real server with the result
observed by the user; **unit** = automated test with no network; **structural** = it builds and runs
but its behaviour has not been exercised end to end.

| Area | Grade | Evidence |
|---|---|---|
| DS2 signature | live | Byte-for-byte match against a captured request |
| Device templates, DPAPI store | unit + live | `cargo test`, `mhyqr device create/show` against a real store |
| `getFp` device registration | live | Live round trip on a generated virtual device |
| Password login, SMS login | live | Both accounts; SMS code typed manually |
| `login_verify` (Porte session activation) | live | After password login |
| Token exchange (cookie_token + ltoken) | live | Same run |
| Game QR scan + approve | live | PC Genshin client logged in; pure virtual device, retcode 0 all three steps |
| Risk control (`-3235`) | live | Fresh virtual device, new-device verification on; server sent SMS, code entered manually, flow completed |
| One-shot `mhyqr qr login-game` | live | `--capture-screen` and `--image` paths, no env overrides |
| Rust test suite | unit | `cargo test --workspace` — 103 passed / 23 targets (2026-09-11) |
| Clippy / fmt | structural | `cargo clippy --workspace --all-targets -- -D warnings` clean |
| FFI boundary (Dart → Rust → DPAPI) | unit | `app/test/ffi_test.dart` — 10 bridge cases + a shell-navigation `testWidgets` |
| CLI ↔ shell share one store | unit | Two interop tests: CLI writes → Dart reads, and the reverse. Proves DPAPI blobs written by one process open in the other under the same Windows user |
| SMS request shapes | unit | Mock-server tests assert the bytes: `createLoginCaptcha` sends only the encrypted pair plus an empty `x-rpc-aigis`; the submit call uses a **string** `action_type` and no aigis header |
| SMS input handling | unit | Widget tests reject an email, an embedded country code, and an empty phone before any request is made |
| Password login hitting a code challenge | **structural** | The `-3235` → SMS classification, the audit entry and the dialog's 改用短信登录 action are unit tested; the dialog has not been clicked through against a live challenge |
| Packaged desktop app | structural | `build_windows.ps1` produces a runner that launches and renders the shell; launches and renders the shell end to end |
| Settings flags + audit log | unit | Rust round-trip/ordering/rotation tests, plus a `testWidgets` that navigates to the Settings tab and asserts the screen renders. The audit list (最近操作) is **developer-mode-only** and not even read in normal mode; the per-account approval policy this row used to cover was removed 2026-09-12 — approval is per source now |
| Auth attempt auditing | unit | A failed password login is asserted to leave one audit entry carrying the action, the challenge kind and the retcode — and **not** the server's wording or the masked phone number |
| Manual-approval dialog (仅扫描) | live approve path / structural scan branch | Approval is per source: 抢码 approves on win, 仅扫描 parks the ticket and the dialog approves it via `qr_confirm_pending`. The auto-approve path was exercised live on a real Genshin QR; the parked-ticket branch is covered by Rust tests and its UI mapping (`confirm: _approveMode`) needs one live 仅扫描 run |
| Non-Genshin titles | **unverified** | `GameBiz` and panda-host derivation are inferred; only `hk4e_cn` is live confirmed |
| QR role A (tool displays a QR for a phone to scan) | **not implemented** | — |

Live steps last ran on 2026-09-12 (full GUI pass with a real Genshin QR, SMS login through the
aigis solve, first Bilibili live-room pull). Rows marked *live* are dated; anything structural or
unverified must be re-confirmed before relying on it.

## 7. Outstanding work, in priority order

1. **Live QR-in-stream race.** The Bilibili pipeline is verified end-to-end (real room resolved,
   ffmpeg pumping frames, per-room 分辨率/fps/延迟 on the cards), but no QR has yet appeared in a
   real stream for the tool to approve. A second browser window showing a real game QR works as a
   stand-in for the screen-capture path.
2. **Decode latency tuning.** Measure QR-on-screen → confirm-sent, then tune frame pacing/decoding.
   The race is won or lost here.
3. **Plan 6: per-title verification.** Star Rail / Zenless / Honkai 3rd need a live QR each to
   confirm the panda host (`<prefix>-sdk.mihoyo.com`) and `x-rpc_app_id`. 崩坏3 approved once
   (display fallback added); `--panda-host` overrides the derived host in the meantime.
4. **`getTokenBySToken` returns `-5300`** for both test accounts; the endpoint expects a different
   token generation. Documented, does not block QR login.
5. **Virtual profiles vs. a real phone's `pre_device.xml`.** A real device carries a UUID/suffix/
   mock-`device_id` triple distinct from the 16-hex `porte_sdk_common.device_id`. Synthesize it only
   if risk control starts rejecting virtual profiles.
6. **No git remote.** `main` exists only locally. Adding one is a user decision, not a code change.

Done since this list was written (2026-09-12): the live GUI pass, SMS login re-verified through the
aigis solve, the per-source approval dialog, `audit.log` rotation (1 MiB, keeps the newest half),
`AccountId` validation + `zeroize` plaintext scrubbing, and the `game_id` rename. Plan 4 role A is
**out of scope** per the charter — the tool is always the scanner.

## 8. Traps

Each of these has already cost time once.

- **Stale DLL after codegen** → "Content hash on Dart side … is different from Rust side". Rebuild;
  do not go hunting for a real bug (see §4).
- **Non-ASCII path breaks the Flutter Windows build**, not the Dart part. Always build through the
  junction. The same class of defect bit the FRB generator: supplying an explicit `rust_output`
  fails with `prefix not found` because the generator canonicalises `rust_root` (adding the Windows
  `\\?\` verbatim prefix) but not an explicit output path. That is why `flutter_rust_bridge.yaml`
  deliberately omits `rust_output`.
- **`flutter analyze` crashes its analysis server here.** `flutter test` and `flutter build` are the
  reliable gates; do not treat a crashed analyze as a code problem.
- **Debug builds are ~20× slower** for capture + QR decode (≈7 s vs ≈300 ms per attempt). Always use
  a release cdylib for anything involving the QR loop.
- **Capture + decode needs two identical consecutive payloads** before accepting a QR
  (`StabilityFilter::new(2)`). Accepting the first read produces flaky approvals.
- **QR decoding needs scale retries**: frames ≥ 2560 px are halved first, frames < 800 px are tried
  at 2× and 3×. A 1280 px detection cap made it roughly 10× faster — do not remove it casually.
- **Risk `-3503` means inconsistent device identity** — usually a real `device_id`/`fp` mixed with a
  virtual model. Keep the whole identity set internally consistent.
- **PowerShell `>` writes UTF-16LE.** Any diff or report consumed by other tooling must be written as
  UTF-8 explicitly.
- **`git` needs `-c safe.directory='E:/抢码工具/repo'`** (see §3.6).
- **The GUI used to have no SMS fallback while the CLI did.** The CLI's `auth login` retries SMS
  whenever the password step does not produce a session, *regardless of the challenge kind*. That
  hides classification bugs: the CLI succeeds where the GUI dead-ends. When a login path behaves
  differently in the two front ends, suspect the classification rather than the transport.
- **A challenge's retcode carries the semantics; its `message` does not.** Matching on message text
  (`风险`, `验证码`) is a fallback for unknown codes only. `-3235` and `-3503` both contain `风险` and
  need opposite remedies.
- **SMS login needs a phone number, not an email.** The two login modes take different identifiers;
  sharing one input is how an email ends up in the `mobile` field. The RSA encryption hides the
  mistake from every layer until the server rejects it.

## 9. Accounts and data handling

Test accounts are referred to by their **local labels only**, which is what the store keys use:
local labels only (see the DPAPI store). The
device profiles behind them live in the DPAPI store, never in git.

Two account passwords were pasted into a chat transcript during development. They are not recorded
here and must not be: **rotate both** if that transcript is retained anywhere.

Account 1's new-device verification was toggled off and back on while testing the `-3235` path;
check the account's security settings if login behaviour looks unexpected.

## 10. Where the evidence is

Live-verification evidence (captures, analysis notes) is kept outside the public repository.
Never copy raw captures into the repository. Summarise field names and shapes instead.
