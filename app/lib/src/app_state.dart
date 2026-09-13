import 'package:flutter/foundation.dart';

import 'rust/api/core.dart';
import 'rust/dto.dart';

/// Shared state for the shell: the core handle, the selected account label and
/// the latest backend message.
///
/// Holds no secret: results from the core are already masked.
class AppState extends ChangeNotifier {
  AppState();

  Core? _core;
  BridgeInfo? _info;
  String _account = '';
  String? _status;
  bool _devMode = false;
  bool _closeToTray = false;
  bool _keepSession = true;
  bool _popupOnScan = true;
  bool _popupOnApprove = true;
  bool _guide = false;

  Core get core {
    final c = _core;
    if (c == null) {
      throw StateError('core not initialised');
    }
    return c;
  }

  bool get ready => _core != null;
  BridgeInfo? get info => _info;
  String get account => _account;
  String? get status => _status;
  bool get devMode => _devMode;
  bool get closeToTray => _closeToTray;
  bool get keepSession => _keepSession;
  bool get popupOnScan => _popupOnScan;
  bool get popupOnApprove => _popupOnApprove;

  /// 界面指引文本（默认关闭：界面上只留操作与状态）。
  bool get guide => _guide;

  Future<void> init() async {
    final core = await coreNew();
    _core = core;
    _info = await core.info();
    notifyListeners();
  }

  /// Empty clears the selection (e.g. after deleting the active profile).
  void selectAccount(String value) {
    _account = value.trim();
    notifyListeners();
  }

  void setDevMode(bool enabled) {
    _devMode = enabled;
    notifyListeners();
  }

  void setCloseToTray(bool enabled) {
    _closeToTray = enabled;
    notifyListeners();
  }

  void setKeepSession(bool enabled) {
    _keepSession = enabled;
    notifyListeners();
  }

  void setPopupFlags(bool onScan, bool onApprove) {
    _popupOnScan = onScan;
    _popupOnApprove = onApprove;
    notifyListeners();
  }

  void setGuide(bool enabled) {
    _guide = enabled;
    notifyListeners();
  }

  void setStatus(String? message) {
    _status = message;
    notifyListeners();
  }
}
