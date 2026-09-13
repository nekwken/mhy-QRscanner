# 米游扫码/抢码器（mhy-QRscanner）

**中文** | [English](README.en.md) | [日本語](README.ja.md)

[![Release](https://img.shields.io/github/v/release/nekwken/mhy-QRscanner?label=Release)](https://github.com/nekwken/mhy-QRscanner/releases)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

米哈游游戏扫码登陆/直播**抢码**工具 — Windows 桌面客户端

> ⚠️ 本工具仅限**本人账号、本人设备**使用
> 使用前请阅读[使用范围与免责声明](#使用范围与免责声明)。


本工具原理是把你的米哈游通行证会话常驻在一个虚拟设备上，通过 **B 站直播流**和/或**本机屏幕**
等方式捕获二维码并由这个虚拟设备上的会话批准，以期实现更快更方便的扫码登陆，可以用于直播抢码或者手机不在身边时的扫码登陆

![扫码页](docs/images/main.png)

## 功能
这个工具本质上是一个米游社扫码登陆功能的第三方客户端，它：
- 可以通过账号密码或者手机验证码登录米哈游通行证（⚠️登陆时可能需要手动完成人机验证以获取短信验证码）
- 登陆后可以以米游社移动客户端身份批准登录，理论上和米游社扫码器功能相同
- 可以从多个来源同时抢码，批准最先出现二维码的登录

## 下载与运行

从 [Releases](https://github.com/nekwken/mhy-QRscanner/releases) 下载 Windows x64 压缩包，
解压后运行 `mhy_QRscanner.exe`，免安装。发行包有两个版本：

| 版本 | 文件名 | 说明 |
|---|---|---|
| 精简版 | `mhy-QRscanner-v<版本>-windows-x64.zip` | 需要系统里已有 ffmpeg |
| **内置 ffmpeg** | `mhy-QRscanner-v<版本>-windows-x64-with-ffmpeg.zip` | 解压即用，直播拉流无需自备 ffmpeg |

运行要求：

- Windows 10/11 x64
- 使用 B 站直播源需要 [ffmpeg](https://ffmpeg.org/download.html)。解析顺序：
  `MHYQR_FFMPEG` 环境变量 → 程序同目录的 `ffmpeg.exe`（内置版发行包）→ `PATH` 上的
  `ffmpeg`（仅用屏幕监控 / 截图文件 / 二维码链接时不需要 ffmpeg）
- 内置版携带的 ffmpeg 为 LGPL 构建（BtbN/FFmpeg-Builds），附 `FFMPEG-LICENSE.txt`

## 快速上手

1. **登录**：账号页用密码或短信验证码登录米哈游通行证。
2. **配置源**：扫码页有四个页签——
   - **B站直播**：登记直播间（房间号 + 标签）；
   - **本机屏幕**：监控整个屏幕；
   - **截图文件 / 二维码链接**：一次性单路扫描。

   在「源」页签的卡片上启用/停用各路源，并设置模式（仅扫描 / 扫描并批准）。
3. **竞速**：点「开始捕获」。二维码被识别后，按各源的模式自动批准，或弹出确认框由你二次确认

命令行用法见 [CLI](#cli) 一节。

## 从源码构建

工具链：Rust 1.98.1，Flutter 3.47.3（`flutter_rust_bridge` 2.13.0 仅在重新生成绑定时需要）。

```powershell
# 先构建 Rust release 产物，再构建 Flutter Windows 应用，
# 并把 mhy_qrscanner_bridge.dll 放到 mhy_QRscanner.exe 旁（应用从那里加载）。
powershell -NoProfile -ExecutionPolicy Bypass -File app\tool\build_windows.ps1

# 需要内置 ffmpeg 的发行包时追加参数（ffmpeg.exe 与其 LICENSE 会一并放进 Release 目录）：
powershell -NoProfile -ExecutionPolicy Bypass -File app\tool\build_windows.ps1 -BundleFfmpeg F:\ffmpeg\bin\ffmpeg.exe
```

> 构建脚本通过 ASCII junction 工作，绕过 Flutter Windows 构建对非 ASCII 路径的解码问题；
> 手动命令与绑定重新生成见 `docs/HANDOVER.md` §4。

## CLI

```powershell
# 1. 虚拟设备
cargo run -p mhy-qrscanner-cli -- device create --account my-acct --template xiaomi14
cargo run -p mhy-qrscanner-cli -- device register --account my-acct
cargo run -p mhy-qrscanner-cli -- device show --account my-acct

# 2. 账号登录（密码 → 短信兜底 → 会话激活 → 票据交换）
$env:MHYQR_PASSWORD = "..."
cargo run -p mhy-qrscanner-cli -- auth login --account my-acct --login <手机号>
cargo run -p mhy-qrscanner-cli -- auth show --account my-acct

# 3. 批准一个游戏二维码（一次性：链接或截图）
cargo run -p mhy-qrscanner-cli -- qr login-game --account my-acct --image <screenshot.png>
```

实测链路：PC 上生成的虚拟设备通过 `getExtList`/`getFp` 注册；密码与短信登录拿到
`stoken`；`scanQRLogin` + `confirmQRLogin` 让真实的 PC 客户端完成登录。新设备风控质询
（`-3235`）会呈现给用户**手动**完成。


## 仓库结构

```text
crates/
  mhy-qrscanner-core/     # 领域模型与错误
  mhy-qrscanner-device/   # 虚拟设备生成（5 套模板）+ 注册
  mhy-qrscanner-mihoyo/   # DS2 签名、RSA 凭据、登录/验证/票据交换、x-rpc 头
  mhy-qrscanner-qr/       # panda + 通行证二维码、cookie 构建、本地二维码解码
  mhy-qrscanner-capture/  # Windows GDI 屏幕捕获
  mhy-qrscanner-store/    # DPAPI 加密存储
  mhy-qrscanner-cli/      # `mhyqr` 命令行
  mhy-qrscanner-bridge/   # Flutter FFI 边界（校验、脱敏、转发）
app/            # Flutter Windows 外壳
  lib/src/screens/  # 账号 / 登录 / 扫码 / 设置 / 关于
  lib/src/rust/     # 生成的绑定——不要手改
  tool/build_windows.ps1
docs/
  HANDOVER.md   # 环境、构建配方、验证分级、已知坑（从这里开始）
flutter_rust_bridge.yaml  # 代码生成配置
```

## 调试环境变量

正常使用不需要设置：

| 变量 | 用途 |
|---|---|
| `MHYQR_DEBUG` | stderr 详细诊断 |
| `MHYQR_PASSWORD` | `auth login` / `auth login-password` 的密码 |
| `MHYQR_SMS_CODE` | 直接提交已收到的短信码，不再触发下发 |
| `MHYQR_QR_CONFIRM=1` | 旧 `qr scan`：同时调用 `confirmQRLogin` |
| `MHYQR_APP_ID` / `MHYQR_CLIENT_TYPE` / `MHYQR_GAME_BIZ` | 覆盖 `x-rpc-*` 身份 |
| `MHYQR_DEVICE_ID` / `MHYQR_DEVICE_FP` / `MHYQR_DEVICE_NAME` / `MHYQR_DEVICE_MODEL` / `MHYQR_SYS_VERSION` | 覆盖设备身份 |
| `MHYQR_FFMPEG` | 指定 ffmpeg 可执行文件路径 |
| `MHYQR_BRIDGE_DLL` / `MHYQR_CLI_EXE` | 仅测试：告诉 `flutter test` release cdylib 与 CLI 的位置 |

## 使用范围与免责声明

- 仅用于**本人账号、本人设备**
- 你的账号安全由你自己负责，本工具只负责登录，不保证登录后的环境安全，不保证账号安全
- 本工具不保存你的密码，只将登录态存储于本地，但相关文件夹也会产生相关隐私文件，请妥善处理
- 这个项目处于非常早期阶段，可能存在海量bug（
- 使用本工具可能违反游戏或平台的服务条款，由此产生的账号风险与一切后果由使用者自行承担；本项目仅供学习与研究，请遵守所在地区的法律法规。

## 许可证

MIT — 见 [LICENSE](LICENSE)。
