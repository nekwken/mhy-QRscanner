# 后续版本想法（未排期）

> 用户提出的待办，仅记录，暂不实现。实现时把对应条目移入 `docs/HANDOVER.md` 的验证清单。
>
> README 的「后续开发计划」是公开摘要（Android / 更多直播平台 / 指定与多窗口捕获三条）；
> 本文件是明细版，条目更多（另含弹窗直达直播流、多账户能力）。
>
> ⚠️ 待确认：README「功能」一栏目前把 **多账户能力** 列为已具备，但实际只有后端按账号参数化，
> 界面仍是单账号（见下第 5 条）。要么把它移到「后续开发计划」，要么按第 5 条实现后再留在功能里。

## 1. 构建 Android 版本

把现在只面向 Windows 桌面（屏幕捕获 + Flutter Windows 外壳）的能力扩展到 Android。

要点（初步判断，实现前需重新评估）：

- **捕获来源**：桌面版依赖 Windows GDI 截屏与 ffmpeg 拉流；Android 上对应的是
  MediaProjection 截屏 + 直接 HTTP/FLV 拉流，`mhy-qrscanner-capture` / `mhy-qrscanner-live`
  需要新的平台实现或抽象层。
- **桥接层**：`mhy-qrscanner-bridge` 是 Flutter FFI（cdylib）；Android 需要改成 JNI/`flutter_rust_bridge`
  的 Android 产物（`cargo-ndk` + `.so`），构建脚本要新增一套。
- **凭据存储**：桌面用 Windows DPAPI；Android 对应 Keystore / EncryptedSharedPreferences，
  `mhy-qrscanner-store` 需要平台分支。
- **签名与风控**：设备身份（`device_id` / `device_fp`）目前按桌面虚拟设备生成，移动端是否需要
  换一套模板需实测。
- 参考：`docs/HANDOVER.md` §3（环境准备）与 §4（构建配方）。

## 2. 批准/通知弹窗上加“一键直达直播流”

抢码成功后的弹窗里，给出**获胜来源的直播流链接**，点击直接打开对应直播间。

要点：

- 数据链路已具备大半：竞速状态 `WatcherStateDto` 已含 `kind`（`bilibili` / `screen`）与 `room`
  （直播间号），Dart 侧 `RoomEntry` 也保留 `roomId`；只需把它带进 `ApprovalOutcome`
  （或弹窗入参）即可。
- 链接形式：`https://live.bilibili.com/<room_id>`；用 `Process.run('rundll32', ['url.dll,FileProtocolHandler', url])`
  打开（与「关于」页的项目主页链接同一套做法）。
- 屏幕源没有对应直播间：该情况下不显示链接，或退化为“打开抓到的二维码图片/截图”。
- 多源竞速时只对**获胜源**给链接；若获胜源是屏幕而用户想直达直播，可在同一弹窗里列出
  其余启用中的直播间作为次要入口。
- 弹窗受设置开关控制（`popup_on_scan` / `popup_on_approve`），链接随弹窗一起出现即可。

## 3. 支持更多直播平台

目前直播源只实现了 B 站（`mhy-qrscanner-live::resolve_play_url` 走 `getRoomPlayInfo` 公共接口，
拿到 host + base_url + extra 拼出流地址，再交给 ffmpeg 抽帧）。抖音 / 虎牙 / 斗鱼 / 快手 /
YouTube Live 等平台同理存在「主播在直播里展示游戏登录码」的场景。

要点：

- **最省事的一步：通用「流地址」源**。ffmpeg 能吃 HLS/FLV/RTMP，所以先加一个
  `kind: "stream_url"` 的源——用户自己粘贴播放地址（浏览器 F12 或现成工具拿到），
  其余解码、判稳、竞速逻辑完全复用。这一步不依赖任何平台接口，见效最快。
- **再按平台补解析器**：`WatcherSpecDto.kind` 从 `bilibili | screen` 扩展为平台标识，
  `mhy-qrscanner-live` 内按平台分发 `resolve_play_url`。各平台难度差别很大：
  B 站（已实现，公共接口 + WBI 可选）< 斗鱼/虎牙（接口较固定）< 抖音/快手（签名与风控
  参数随版本变化，需持续维护）。
- **UI**：源卡片目前只有「直播」一个标签与一个房间号输入框；需要加平台选择（下拉），
  `RoomEntry` 增加 `platform` 字段并持久化（旧的 `settings/rooms` JSON 需兼容缺省值）。
- **与第 2 条联动**：「一键直达直播流」的房间 URL 模板按平台不同
  （B 站 `live.bilibili.com/<room>`、虎牙 `huya.com/<room>` 等），做平台抽象时一并定义。
- **维护成本与风险**：平台解析器会随对方改版失效，建议每个平台一个可选特性开关，
  失效时回退到通用「流地址」源；同时注意各平台服务条款与抓流限制。

## 4. 屏幕捕获支持「指定窗口」与「多窗口同时捕获」

现在屏幕源是整块虚拟屏幕（`GetDC(NULL)` + `BitBlt`，见 `mhy-qrscanner-capture/src/windows.rs`），
一个竞速里只能有一个屏幕源。期望：只捕获某个窗口，并且能同时捕获多个窗口、各自作为一个源参与竞速。

要点：

- **捕获 API 的选择**（按优先级）：
  1. **Windows Graphics Capture（WGC）** — Win10 1803+ 的现代接口（`windows` crate 的
     `Graphics::Capture`），按窗口捕获、被遮挡仍可抓、对 GPU 合成内容（播放器/浏览器）兼容最好；
     需要一个 WinRT 互操作层（`CreateDirect3D11DeviceFromDXGIDevice` + `Direct3D11CaptureFramePool`）。
     新版 Windows 上可关闭捕获提示黄框。
  2. **`PrintWindow(hwnd, …, PW_RENDERFULLCONTENT)`** — 实现最轻，覆盖大多数普通窗口；
     对部分 DirectX/独占全屏/UWP 表面会返回黑帧。
  3. **`GetWindowDC` + `BitBlt`** — 只能拿到可见部分，被遮挡或最小化即失效，仅作兜底。
  建议：WGC 为主，保留现有整屏 `BitBlt` 作为兜底与降级路径。
- **接口设计**：`QrSource::Screen { target: ScreenTarget, wait_secs }`，其中
  `ScreenTarget = FullScreen | Window(WindowRef) | Region{...}`；`WatcherSpecDto.kind` 仍是
  `screen`，新增窗口标识字段，竞速层不用改（本来就是 N 源并发、先到先得，每个源独立统计）。
- **窗口引用怎么存**：HWND 不能跨进程重启复用。持久化存「可执行文件名 + 标题关键字」，
  启动时用 `EnumWindows` 重新解析；解析不到就提示用户重选。选择器只列可见、有标题、
  且不是本工具自身的顶层窗口。
- **边界与坑**：
  - 最小化窗口无法捕获（WGC/PrintWindow 都拿不到有效帧）→ 需要提示用户保持窗口可见；
  - 独占全屏的游戏窗口不可捕获 → 需切「无边框窗口」模式；
  - 多显示器 + DPI 缩放下的坐标/尺寸换算；
  - 别捕获本工具自己的窗口（自反馈）→ 选择器里排除并把当前窗口置灰；
  - WGC 需要处理窗口关闭/重建（重新挂钩或把该源标记为错误）。
- **验证方式**：用一张放着二维码的图片窗口做目标，断言解码成功；同时开两个窗口捕获，
  断言两个源都在出帧；把目标窗口用别的窗口盖住，断言仍能连续出帧（这是选 WGC 的主要理由）。

## 5. 多账户能力

现状：后端其实已经按账号参数化——`Core` 的 store 里会话（`session/<标签>`）与虚拟设备档案
（`profile/<标签>`）都是**按账号标签分区**的，桥接层的 `qr_login_game` / `qr_race_start` 也都带
`account` 参数；受限的是界面：`AppState.account` 只有一个当前账号，竞速与批准全部用它。

期望：多个账号可并存待命、可一键切换，并能让**每个源绑定不同的账号**。

要点：

- **界面**：
  - 账号切换（导航栏或标题栏放一个账号下拉，显示掩码账号 + 登录态徽标，切换即换 `AppState.account`）；
  - 源卡片上增加「账号」下拉（默认跟随当前账号，可选「固定到某账号」）；
  - 状态栏显示当前账号；托盘悬浮提示同步。
- **持久化**：`settings/rooms` 的每条房间记录增加 `account` 字段（缺省 = 跟随当前账号，兼容旧 JSON）；
  `settings/last_account` 变成「上次使用的账号」，另外记住每个账号的登录态（已有 `keep_session` 机制）。
- **竞速语义（需要定清）**：
  - 一个源 = 一个账号：胜出的源用**它绑定的账号**批准；
  - 同一张二维码只能被一个账号批准（现有 `TicketGate` 10 分钟票据去重继续生效，
    且只有先到的源会走到 confirm，其余源在胜者锁定后即收工）；
  - 多个源绑定同一个账号时行为与现状一致。
- **登录成本**：每个账号首次都要登录并各自建一份虚拟设备档案；图形验证（极验）逐个手动完成，
  这是设计边界，不做批量自动登录。
- **安全边界不变**：仍然是**仅本人账号**——多账号指的是用户自己名下的多个米哈游通行证，
  不是代管他人凭证；README 的免责声明需要同步措辞。
- **验证方式**：两个账号各登录一次 → 两个源分别绑定不同账号 → 竞速一次，断言胜者源用其绑定账号
  完成批准（审计日志里 `qr_race` 的 account 字段可核对）。
