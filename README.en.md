# mhy-QRscanner (米游抢码器)

[中文](README.md) | **English** | [日本語](README.ja.md)

[![Release](https://img.shields.io/github/v/release/nekwken/mhy-QRscanner?label=Release)](https://github.com/nekwken/mhy-QRscanner/releases)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> **Machine translation notice** — this document is an AI translation of the Chinese
> [README.md](README.md). Where the two disagree, the Chinese version is authoritative.
<!-- readme-source-sha256: 86ce1f6d08c32fe392b90710072ad855868d854224027f51846c9b1c5aee1c32 -->

A **QR-race** tool for MiHoYo game live streams — Windows desktop client.

> ⚠️ Use it only with **your own account and your own devices**.
> Read [Scope & disclaimer](#scope--disclaimer) before using it.

The idea: keep your MiHoYo passport session resident on a virtual device, capture QR codes from a
**Bilibili live stream** and/or **your own screen**, and let the resident session approve them — so
you can win the race faster and more conveniently.

![Scan page](docs/images/main.png)

## Features

This tool is essentially a third-party client for Miyoushe's scan-to-log-in feature. It can:

- log into a MiHoYo passport with an account password or a phone verification code
  (⚠️ getting an SMS code may require completing a human-verification step by hand)
- approve logins as the Miyoushe mobile client — in principle the same job as the Miyoushe scanner
- race several sources at once and approve whichever QR code appears first

## Download & run

Grab the Windows x64 archive from [Releases](https://github.com/nekwken/mhy-QRscanner/releases),
unzip it and run `mhy_QRscanner.exe` — no installer. Two builds are published:

| Build | File name | Notes |
|---|---|---|
| Slim | `mhy-QRscanner-v<version>-windows-x64.zip` | expects ffmpeg to be installed already |
| **ffmpeg bundled** | `mhy-QRscanner-v<version>-windows-x64-with-ffmpeg.zip` | unzip and run; no ffmpeg needed for live capture |

Requirements:

- Windows 10/11 x64
- The Bilibili live source needs [ffmpeg](https://ffmpeg.org/download.html). Resolution order:
  the `MHYQR_FFMPEG` environment variable → `ffmpeg.exe` next to the program (the bundled build) →
  `ffmpeg` on `PATH`. (Screen monitoring, screenshot files and QR links need no ffmpeg.)
- The bundled ffmpeg is an LGPL build (BtbN/FFmpeg-Builds) and ships with `FFMPEG-LICENSE.txt`

## Quick start

1. **Sign in**: log into your MiHoYo passport on the account page with a password or an SMS code.
2. **Configure sources**: the scan page has four tabs —
   - **Bilibili**: register live rooms (room id + label);
   - **Local screen**: watch the whole screen;
   - **Screenshot file / QR link**: one-off single-source scans.

   Enable or disable each source on its card in the **Sources** tab and pick its mode
   (scan only / scan and approve).
3. **Race**: press “Start capture”. Once a QR is recognised it is approved automatically according to
   each source's mode, or a dialog asks you to confirm a second time.

Command-line usage is described in [CLI](#cli).

## Building from source

Toolchain: Rust 1.98.1, Flutter 3.47.3 (`flutter_rust_bridge` 2.13.0 is only needed when
regenerating the bindings).

```powershell
# Builds the Rust release artifacts, then the Flutter Windows app, then places
# mhy_qrscanner_bridge.dll next to mhy_QRscanner.exe (the app loads it from there).
powershell -NoProfile -ExecutionPolicy Bypass -File app\tool\build_windows.ps1

# Add this switch for a build with ffmpeg inside (ffmpeg.exe and its LICENSE land in Release):
powershell -NoProfile -ExecutionPolicy Bypass -File app\tool\build_windows.ps1 -BundleFfmpeg F:\ffmpeg\bin\ffmpeg.exe
```

> The build script works through an ASCII junction, side-stepping Flutter's decoding problem with
> non-ASCII project paths; manual commands and binding regeneration are in `docs/HANDOVER.md` §4.

## CLI

```powershell
# 1. virtual device
cargo run -p mhy-qrscanner-cli -- device create --account my-acct --template xiaomi14
cargo run -p mhy-qrscanner-cli -- device register --account my-acct
cargo run -p mhy-qrscanner-cli -- device show --account my-acct

# 2. account login (password → SMS fallback → session activation → token exchange)
$env:MHYQR_PASSWORD = "..."
cargo run -p mhy-qrscanner-cli -- auth login --account my-acct --login <phone>
cargo run -p mhy-qrscanner-cli -- auth show --account my-acct

# 3. approve one game QR (one-shot: link or screenshot)
cargo run -p mhy-qrscanner-cli -- qr login-game --account my-acct --image <screenshot.png>
```

Verified end to end: a PC-generated virtual device registers through `getExtList`/`getFp`; password
and SMS login obtain a `stoken`; `scanQRLogin` + `confirmQRLogin` make a real PC client log in.
A new-device risk challenge (`-3235`) is surfaced for the user to complete **manually**.

## Repository layout

```text
crates/
  mhy-qrscanner-core/     # domain models and errors
  mhy-qrscanner-device/   # virtual device generation (5 templates) + registration
  mhy-qrscanner-mihoyo/   # DS2 signing, RSA credentials, login/verify/exchange, x-rpc headers
  mhy-qrscanner-qr/       # panda + passport QR, cookie building, local QR decoding
  mhy-qrscanner-capture/  # Windows GDI screen capture
  mhy-qrscanner-store/    # DPAPI-encrypted storage
  mhy-qrscanner-cli/      # the `mhyqr` command line
  mhy-qrscanner-bridge/   # Flutter FFI boundary (validates, masks, forwards)
app/            # Flutter Windows shell
  lib/src/screens/  # accounts / login / scan / settings / about
  lib/src/rust/     # generated bindings — never edit by hand
  tool/build_windows.ps1
docs/
  HANDOVER.md   # environment, build recipes, verification grades, traps (start here)
flutter_rust_bridge.yaml  # codegen config
```

## Debug environment variables

Not needed for normal use:

| Variable | Purpose |
|---|---|
| `MHYQR_DEBUG` | verbose diagnostics on stderr |
| `MHYQR_PASSWORD` | password for `auth login` / `auth login-password` |
| `MHYQR_SMS_CODE` | submit an already-received SMS code without sending a new one |
| `MHYQR_QR_CONFIRM=1` | legacy `qr scan`: also call `confirmQRLogin` |
| `MHYQR_APP_ID` / `MHYQR_CLIENT_TYPE` / `MHYQR_GAME_BIZ` | override the `x-rpc-*` identity |
| `MHYQR_DEVICE_ID` / `MHYQR_DEVICE_FP` / `MHYQR_DEVICE_NAME` / `MHYQR_DEVICE_MODEL` / `MHYQR_SYS_VERSION` | override device identity |
| `MHYQR_FFMPEG` | path to the ffmpeg executable |
| `MHYQR_BRIDGE_DLL` / `MHYQR_CLI_EXE` | test-only: tell `flutter test` where the release cdylib and CLI are |

## Scope & disclaimer

- **Your own account and your own devices only**
- Your account security is your own responsibility: this tool only performs the login; it does not
  guarantee the safety of the environment after login, nor the safety of the account.
- The tool does not store your password, keeping only the login session locally — but that folder
  does contain private files, so handle it with care.
- This project is at a very early stage and may contain a great many bugs.
- Using this tool may violate a game's or platform's terms of service. Any account risk and all
  consequences are the user's own; the project is published for learning and research — follow the
  laws of your jurisdiction.

## License

MIT — see [LICENSE](LICENSE).
