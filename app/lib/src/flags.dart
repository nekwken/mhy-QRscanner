/// Persisted boolean/string settings keys — must match the bridge's settings/ paths.
const kFlagDevMode = 'settings/dev_mode';
const kFlagCloseToTray = 'settings/close_to_tray';
const kFlagKeepSession = 'settings/keep_session';
const kFlagPopupOnScan = 'settings/popup_on_scan';
const kFlagPopupOnApprove = 'settings/popup_on_approve';

/// 界面指引：开启后显示说明性文本，关闭（默认）则只留操作与状态。
const kFlagGuide = 'settings/guide';
const kTextLastAccount = 'settings/last_account';
const kTextRooms = 'settings/rooms';

/// 把本机屏幕作为竞速来源：`{"enabled":bool,"mode":"approve"|"scan"}`。
const kTextScreenWatcher = 'settings/screen_watcher';
