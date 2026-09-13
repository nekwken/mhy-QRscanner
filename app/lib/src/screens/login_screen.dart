import 'dart:async';

import 'package:flutter/material.dart';
import 'package:webview_windows/webview_windows.dart' as wv;

import '../app_state.dart';
import '../flags.dart';
import '../rust/api/auth.dart';
import '../rust/api/settings.dart' as settings_api;
import '../rust/dto.dart';
import '../rust/error.dart';

/// Password login with an SMS fallback, plus a modal for manual challenges.
///
/// The two modes take **different identifiers**: password login accepts an email
/// or a phone, SMS login needs a phone. They are separate fields on purpose —
/// sharing one field meant an email could be submitted as `mobile`, which the
/// server rejects without explaining why.
class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key, required this.state});

  final AppState state;

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _loginController = TextEditingController();
  final _passwordController = TextEditingController();
  final _areaCodeController = TextEditingController(text: '+86');
  final _phoneController = TextEditingController();
  final _codeController = TextEditingController();

  bool _busy = false;
  bool _smsMode = false;
  SessionSummary? _session;

  /// In-screen error, because the shell's status bar sits below the fold.
  String? _error;

  /// Seconds until another code may be requested. 0 = ready.
  int _resendIn = 0;
  Timer? _ticker;

  /// 重启后保留登录态（持久化开关，默认开）。
  bool? _keepSession;
  /// 会话有效性探针结果（启动恢复时自动检测）。
  SessionProbeDto? _probe;

  @override
  void initState() {
    super.initState();
    // 短信模式下账号栏输入手机号 → 实时同步到手机号栏（剥除非数字）。
    _loginController.addListener(_syncPhoneFromAccount);
    WidgetsBinding.instance.addPostFrameCallback((_) => _restoreSession());
  }

  /// 启动恢复：勾选「保留登录态」时恢复上次账号并探测登录态有效性。
  Future<void> _restoreSession() async {
    try {
      final core = widget.state.core;
      final keep = await settings_api.flagGet(core: core, key: kFlagKeepSession);
      // 从未设置过（updated_at 为空）时默认开启保留。
      final keepOn = keep.updatedAt.isEmpty ? true : keep.enabled;
      if (!mounted) return;
      setState(() => _keepSession = keepOn);
      widget.state.setKeepSession(keepOn);
      if (!keepOn) return;
      final last = await settings_api.textGet(
        core: core,
        key: kTextLastAccount,
      );
      if (last.value.isEmpty) return;
      widget.state.selectAccount(last.value);
      try {
        final session = await authShow(core: core, account: last.value);
        if (!mounted) return;
        setState(() => _session = session);
      } catch (_) {}
      final probe = await sessionProbe(core: core, account: last.value);
      if (!mounted) return;
      setState(() => _probe = probe);
      widget.state.setStatus(probe.valid
          ? '已恢复登录态：${probe.message}${probe.uidMasked.isEmpty ? '' : '（uid ${probe.uidMasked}）'}'
          : probe.message);
    } catch (_) {}
  }

  /// 登录刚成功即等价于探测通过：会话卡的徽标立刻变绿，不必等下次启动。
  void _markSessionValid() {
    if (!mounted) return;
    setState(() => _probe = const SessionProbeDto(
          valid: true,
          message: '登录态有效（本次登录已验证）',
          uidMasked: '',
        ));
  }

  Future<void> _setKeepSession(bool value) async {
    try {
      await settings_api.flagSet(
        core: widget.state.core,
        key: kFlagKeepSession,
        enabled: value,
      );
      if (!mounted) return;
      setState(() => _keepSession = value);
      widget.state.setKeepSession(value);
      widget.state.setStatus(value ? '重启后将保留登录态' : '退出时将清除登录态');
    } catch (e) {
      if (mounted) widget.state.setStatus('保存失败：$e');
    }
  }

  void _syncPhoneFromAccount() {
    if (!_smsMode || !mounted) return;
    if (!_looksLikePhone(_loginController.text)) return;
    final digits = _loginController.text.replaceAll(RegExp(r'[^0-9]'), '');
    if (digits.isNotEmpty && _phoneController.text != digits) {
      _phoneController.text = digits;
    }
  }

  @override
  void dispose() {
    _ticker?.cancel();
    _loginController.removeListener(_syncPhoneFromAccount);
    _loginController.dispose();
    _passwordController.dispose();
    _areaCodeController.dispose();
    _phoneController.dispose();
    _codeController.dispose();
    super.dispose();
  }

  /// 档案标签由通行证标识派生：同一账号永远对应同一档案，用户无需感知。
  String? _labelFromIdentifier(String raw) {
    final value = raw.trim();
    if (value.isEmpty) return null;
    if (value.contains('@')) {
      // email：仅剔除标签不允许的字符组合
      if (value.contains('..') || value.contains('/') || value.contains(r'\')) {
        return null;
      }
      return value;
    }
    final digits = value.replaceAll(RegExp(r'[^0-9]'), '');
    return digits.isEmpty ? null : digits;
  }

  String get _account => _labelFromIdentifier(_loginController.text) ?? widget.state.account;

  /// A phone number for SMS login: digits only, no country code, no email.
  String? _phoneProblem(String raw) {
    final value = raw.trim();
    if (value.isEmpty) return '请填写手机号';
    if (value.contains('@')) return '短信登录需要手机号，不能填邮箱';
    if (value.startsWith('+')) return '不要把国家/地区代码填在这里，请用上面的输入框';
    final digits = value.replaceAll(RegExp(r'[\s\-()]'), '');
    if (!RegExp(r'^\d{6,15}$').hasMatch(digits)) return '手机号格式不正确';
    return null;
  }

  bool _looksLikePhone(String raw) =>
      RegExp(r'^\+?\d[\d\s\-()]{5,}$').hasMatch(raw.trim());

  void _fail(String message) {
    if (!mounted) return;
    setState(() => _error = message);
    widget.state.setStatus(message);
  }

  Future<void> _guard(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// aigis 挑战：应用内弹出 WebView 加载本机求解页（极验官方组件），
  /// 用户完成（常为静默通过）后自动取回令牌并重试当前登录动作。
  Future<String?> _showAigisDialog(LoginChallengeDto challenge) async {
    final url = challenge.aigisUrl;
    final handle = challenge.aigisHandle;
    if (url == null || handle == null) return null;
    final controller = wv.WebviewController();
    try {
      await wv.WebviewController.initializeEnvironment();
      await controller.initialize();
      await controller.loadUrl(url);
    } catch (e) {
      if (!mounted) return null;
      _fail('应用内浏览器初始化失败：$e');
      return null;
    }
    if (!mounted) return null;
    final navigator = Navigator.of(context);
    StreamSubscription? urlSub;
    urlSub = controller.url.listen((u) {
      if (u.contains('/finished')) {
        urlSub?.cancel();
        if (navigator.canPop()) navigator.pop('finished');
      }
    });
    String? dialogResult;
    try {
      dialogResult = await showDialog<String>(
        context: context,
        barrierDismissible: false,
        builder: (dialogContext) => Dialog(
        child: SizedBox(
          width: 440,
          height: 600,
          child: Column(
            children: [
              Row(
                children: [
                  const SizedBox(width: 16),
                  Expanded(child: Text('图形验证', style: Theme.of(dialogContext).textTheme.titleMedium)),
                  IconButton(
                    onPressed: () => Navigator.of(dialogContext).pop('cancel'),
                    icon: const Icon(Icons.close),
                  ),
                ],
              ),
              const Divider(height: 1),
              Expanded(
                child: wv.Webview(controller),
              ),
            ],
          ),
        ),
      ),
      );
    } finally {
      urlSub.cancel();
    }
    return dialogResult;
  }

  /// 完整的 aigis 处置：弹窗 → 等结果 → 取令牌。用户取消返回 null。
  Future<String?> _solveAigisInApp(LoginChallengeDto challenge) async {
    widget.state.setStatus('请在弹出的窗口中完成图形验证…');
    final dialogResult = await _showAigisDialog(challenge);
    if (dialogResult != 'finished') {
      if (challenge.aigisHandle != null) {
        await aigisCancel(handle: challenge.aigisHandle!);
      }
      return null;
    }
    widget.state.setStatus('验证完成，正在取回令牌…');
    try {
      final dto = await aigisTake(handle: challenge.aigisHandle!, timeoutSecs: BigInt.from(15));
      return dto.token;
    } catch (e) {
      if (mounted) _fail('获取验证令牌失败：$e');
      return null;
    }
  }

  /// Surface a challenge in a modal and, when it is an SMS challenge, offer the
  /// one action that actually resolves it.
  Future<void> _showChallenge(LoginChallengeDto challenge) async {
    if (!mounted) return;
    final switchToSms = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('需要人工验证'),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('类型：${challenge.kind}'),
              if (challenge.retcode != 0) Text('retcode：${challenge.retcode}'),
              if (challenge.message.isNotEmpty) Text('服务端：${challenge.message}'),
              for (final hint in challenge.hints) Text('证据：$hint'),
              const SizedBox(height: 12),
              Text(challenge.instructions),
              const SizedBox(height: 12),
              const Text('本工具不会自动完成验证码或极验。'),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('知道了'),
          ),
          if (challenge.kind == 'sms_captcha')
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('改用短信登录'),
            ),
        ],
      ),
    );
    if (switchToSms != true) return;
    _enterSmsMode();
    await _requestCode();
  }

  /// Switch to SMS mode, carrying the phone over when the password field held one.
  void _enterSmsMode() {
    if (!mounted) return;
    setState(() {
      _smsMode = true;
      // 账号栏填的是手机号时，把数字搬运到手机号栏（剥离 +86、空格等）。
      if (_phoneController.text.trim().isEmpty && _looksLikePhone(_loginController.text)) {
        final digits = _loginController.text.replaceAll(RegExp(r'[^0-9]'), '');
        if (digits.isNotEmpty) _phoneController.text = digits;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('通行证登录', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        if (widget.state.guide)
          const Text('密码只用于本次请求，不会写入磁盘或日志。短信验证码由你手动输入。'),
        const SizedBox(height: 20),
        TextField(
          controller: _loginController,
          decoration: InputDecoration(
            labelText: '米哈游通行证账号（手机号或邮箱）',
            helperText: widget.state.guide
                ? '登录后自动准备对应的虚拟设备，无需手动管理'
                : null,
            border: const OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 16),
        SegmentedButton<bool>(
          segments: const [
            ButtonSegment(value: false, label: Text('密码登录'), icon: Icon(Icons.password)),
            ButtonSegment(value: true, label: Text('短信登录'), icon: Icon(Icons.sms_outlined)),
          ],
          selected: {_smsMode},
          onSelectionChanged: (v) {
            if (v.first) {
              // 携带账号栏里的手机号到短信模式（_enterSmsMode 内含搬运逻辑）
              _enterSmsMode();
            } else {
              setState(() => _smsMode = false);
            }
          },
        ),
        const SizedBox(height: 16),
        if (!_smsMode)
          TextField(
            controller: _passwordController,
            obscureText: true,
            decoration: const InputDecoration(
              labelText: '密码',
              border: OutlineInputBorder(),
            ),
          )
        else ...[
          TextField(
            controller: _areaCodeController,
            decoration: const InputDecoration(
              labelText: '国家/地区代码',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _phoneController,
            keyboardType: TextInputType.phone,
            decoration: const InputDecoration(
              labelText: '手机号',
              helperText: '只填号码本身，不要带国际区号',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _codeController,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    labelText: '短信验证码',
                    border: OutlineInputBorder(),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              OutlinedButton(
                onPressed: _busy || _resendIn > 0 ? null : _requestCode,
                child: Text(_resendIn > 0 ? '$_resendIn 秒后可重发' : '获取验证码'),
              ),
            ],
          ),
        ],
        const SizedBox(height: 20),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            FilledButton.icon(
              onPressed: _busy ? null : (_smsMode ? _submitSms : _loginPassword),
              icon: const Icon(Icons.login),
              label: Text(_smsMode ? '提交验证码登录' : '登录'),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : _showSession,
              icon: const Icon(Icons.info_outline),
              label: const Text('检查会话'),
            ),
            TextButton.icon(
              onPressed: _busy ? null : _logout,
              icon: const Icon(Icons.logout),
              label: const Text('清除会话'),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Material(
          color: Colors.transparent,
          child: CheckboxListTile(
            value: _keepSession ?? true,
            onChanged: (v) => _setKeepSession(v ?? true),
            title: const Text('重启后保留登录态'),
            subtitle: Text(
              _probe == null
                  ? '勾选后重启自动恢复本次登录的账号，并检测登录态是否有效'
                  : _probe!.message,
              style: TextStyle(
                fontSize: 12,
                color: (_probe?.valid ?? true)
                    ? Theme.of(context).colorScheme.onSurfaceVariant
                    : Theme.of(context).colorScheme.error,
              ),
            ),
            controlAffinity: ListTileControlAffinity.leading,
            dense: true,
            contentPadding: EdgeInsets.zero,
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: 16),
          _ErrorBanner(message: _error!, onDismiss: () => setState(() => _error = null)),
        ],
        if (_busy) ...[
          const SizedBox(height: 16),
          const LinearProgressIndicator(),
        ],
        const SizedBox(height: 24),
        if (_session != null)
          _SessionCard(
            session: _session!,
            devMode: widget.state.devMode,
            probe: _probe,
          ),
      ],
    );
  }

  Future<void> _loginPassword() async {
    final password = _passwordController.text;
    final login = _loginController.text.trim();
    if (login.isEmpty || password.isEmpty) {
      _fail('请填写账号与密码');
      return;
    }
    await _guard(() async {
      try {
        final summary = await authLoginPassword(
          core: widget.state.core,
          account: _account,
          login: login,
          password: password,
        );
        if (!mounted) return;
        setState(() => _session = summary);
        _markSessionValid();
        widget.state.selectAccount(_account);
        await settings_api.textSet(
          core: widget.state.core,
          key: kTextLastAccount,
          value: _account,
        );
        widget.state.setStatus('登录成功');
      } on BridgeError catch (e) {
        if (e is BridgeError_Challenge && e.field0.kind == 'aigis') {
          final token = await _solveAigisInApp(e.field0);
          if (token != null && mounted) {
            final summary = await authLoginPassword(
              core: widget.state.core,
              account: _account,
              login: login,
              password: password,
              aigisToken: token,
            );
            if (!mounted) return;
            setState(() => _session = summary);
            _markSessionValid();
            widget.state.selectAccount(_account);
            await settings_api.textSet(
              core: widget.state.core,
              key: kTextLastAccount,
              value: _account,
            );
            widget.state.setStatus('登录成功');
          }
        } else if (e is BridgeError_Challenge) {
          await _showChallenge(e.field0);
        } else {
          _fail('登录失败：$e');
        }
      } catch (e) {
        _fail('登录失败：$e');
      } finally {
        // never keep the password in memory longer than needed
        _passwordController.clear();
      }
    });
  }

  Future<void> _requestCode({String? aigisToken}) async {
    final phone = _phoneController.text.trim();
    final problem = _phoneProblem(phone);
    if (problem != null) {
      _fail(problem);
      return;
    }
    await _guard(() async {
      try {
        if (aigisToken == null) {
          widget.state.setStatus('正在请求验证码…（如弹出图形验证请在窗口中完成）');
        }
        final dto = await authRequestSms(
          core: widget.state.core,
          account: _account,
          login: phone,
          areaCode: _areaCodeController.text.trim(),
          aigisToken: aigisToken,
        );
        if (!mounted) return;
        // The server tells us how long to wait; 60s is what it sent on the
        // captured run, so it is the fallback when it says nothing.
        final seconds = dto.countdown.toInt();
        _startCountdown(seconds > 0 ? seconds : 60);
        widget.state.setStatus(
          dto.sentNew ? '验证码已发送，请查看手机短信' : '已复用刚发送的验证码，请查看手机短信',
        );
      } on BridgeError catch (e) {
        if (e is BridgeError_Challenge && e.field0.kind == 'aigis') {
          final token = await _solveAigisInApp(e.field0);
          if (token != null && mounted) {
            await _requestCode(aigisToken: token);
          }
        } else if (e is BridgeError_Api) {
          // A rejected request is a refusal, not a challenge: show it inline
          // and cool down, so a frustrated click cannot hammer the endpoint
          // and deepen a server-side rate limit.
          final needsAigis = e.retcode == -3101;
          _fail(
            needsAigis
                ? '服务端要求图形验证但未附带挑战（retcode=-3101），请重试一次'
                : '发送失败（retcode=${e.retcode}）：${e.message}',
          );
          _startCountdown(needsAigis ? 5 : 30);
        } else {
          _fail('发送失败：$e');
        }
      } catch (e) {
        _fail('发送失败：$e');
      }
    });
  }

  void _startCountdown(int seconds) {
    _ticker?.cancel();
    setState(() => _resendIn = seconds);
    _ticker = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (!mounted) {
        timer.cancel();
        return;
      }
      setState(() => _resendIn -= 1);
      if (_resendIn <= 0) timer.cancel();
    });
  }

  /// Clear the wait, so the user can retry immediately.
  void _stopCountdown() {
    _ticker?.cancel();
    _ticker = null;
    if (mounted) setState(() => _resendIn = 0);
  }

  Future<void> _submitSms() async {
    final phone = _phoneController.text.trim();
    final problem = _phoneProblem(phone);
    if (problem != null) {
      _fail(problem);
      return;
    }
    final code = _codeController.text.trim();
    if (code.isEmpty) {
      _fail('请输入收到的验证码');
      return;
    }
    await _guard(() async {
      try {
        final summary = await authSubmitSms(
          core: widget.state.core,
          account: _account,
          login: phone,
          areaCode: _areaCodeController.text.trim(),
          code: code,
        );
        if (!mounted) return;
        setState(() {
          _session = summary;
          _codeController.clear();
        });
        _markSessionValid();
        _stopCountdown();
        widget.state.selectAccount(_account);
        await settings_api.textSet(
          core: widget.state.core,
          key: kTextLastAccount,
          value: _account,
        );
        widget.state.setStatus('短信登录成功');
      } on BridgeError catch (e) {
        if (e is BridgeError_Challenge) {
          await _showChallenge(e.field0);
        } else {
          // A rejected code may be a typo or an expired code; let the user retry
          // straight away rather than leaving the resend button latched.
          _fail('短信登录失败：$e');
          _stopCountdown();
        }
      } catch (e) {
        _fail('短信登录失败：$e');
        _stopCountdown();
      }
    });
  }

  Future<void> _showSession() async {
    await _guard(() async {
      try {
        final summary = await authShow(core: widget.state.core, account: _account);
        if (!mounted) return;
        setState(() => _session = summary);
        // 顺手向服务端探一次，会话卡的有效性徽标保持实时。
        final probe = await sessionProbe(core: widget.state.core, account: _account);
        if (!mounted) return;
        setState(() => _probe = probe);
        widget.state.setStatus(probe.valid
            ? '登录态有效：${probe.message}'
            : '登录态已失效：${probe.message}');
      } catch (e) {
        _fail('读取会话失败：$e');
      }
    });
  }

  Future<void> _logout() async {
    await _guard(() async {
      try {
        await authLogout(core: widget.state.core, account: _account);
        if (!mounted) return;
        setState(() {
          _session = null;
          _probe = null; // 会话卡消失，有效性徽标一并作废
        });
        widget.state.setStatus('会话已清除（设备档案保留）');
      } catch (e) {
        _fail('清除失败：$e');
      }
    });
  }
}

class _ErrorBanner extends StatelessWidget {
  const _ErrorBanner({required this.message, required this.onDismiss});

  final String message;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
      decoration: BoxDecoration(
        color: scheme.errorContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.error_outline, size: 20, color: scheme.onErrorContainer),
          const SizedBox(width: 10),
          Expanded(
            child: SelectableText(
              message,
              style: TextStyle(color: scheme.onErrorContainer),
            ),
          ),
          IconButton(
            onPressed: onDismiss,
            icon: const Icon(Icons.close, size: 18),
            tooltip: '关闭',
          ),
        ],
      ),
    );
  }
}

class _SessionCard extends StatelessWidget {
  const _SessionCard({
    required this.session,
    required this.devMode,
    this.probe,
  });

  final SessionSummary session;
  final bool devMode;

  /// 服务端探测结果：null = 尚未探测。
  final SessionProbeDto? probe;

  /// unix 秒 → 本地系统时间，括号标注 UTC 偏移。
  static String formatUpdatedAt(String unixSeconds) {
    final seconds = int.tryParse(unixSeconds);
    if (seconds == null) return unixSeconds;
    final t = DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
    final off = DateTime.now().timeZoneOffset;
    final sign = off.isNegative ? '-' : '+';
    final hours = off.inHours.abs();
    final minutes = off.inMinutes.remainder(60).abs();
    final utc = 'UTC$sign$hours${minutes > 0 ? ':${minutes.toString().padLeft(2, '0')}' : ''}';
    String two(int v) => v.toString().padLeft(2, '0');
    return '${t.year}-${two(t.month)}-${two(t.day)} '
        '${two(t.hour)}:${two(t.minute)}:${two(t.second)}（$utc）';
  }

  /// 登录态有效性徽标：一眼看出这次会话还能不能用。
  Widget _validityChip(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final Color color;
    final IconData icon;
    final String label;
    if (probe == null) {
      color = scheme.onSurfaceVariant;
      icon = Icons.help_outline;
      label = '未检测';
    } else if (probe!.valid) {
      color = Colors.green;
      icon = Icons.check_circle_outline;
      label = '登录态有效';
    } else {
      color = scheme.error;
      icon = Icons.error_outline;
      label = '登录态已失效';
    }
    return Tooltip(
      message: probe?.message ?? '尚未探测登录态，点「检查会话」可立即检测',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.10),
          borderRadius: BorderRadius.circular(20),
          border: Border.all(color: color.withValues(alpha: 0.45)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 15, color: color),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: color,
              ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    // 非开发者模式只关心两件事：哪个账号、什么时候登录的。
    final rows = devMode
        ? <(String, String)>[
            ('账号', session.account),
            ('stoken', '${session.stokenLen} 字符（前缀 ${session.stokenPrefix}）'),
            ('登录 mid', session.midMasked),
            ('扫码 mid', session.qrMidMasked),
            ('cookie_token', '${session.cookieTokenLen} 字符'),
            ('ltoken', '${session.ltokenLen} 字符'),
            ('更新时间', formatUpdatedAt(session.updatedAt)),
          ]
        : <(String, String)>[
            ('账号', session.account),
            ('更新时间', formatUpdatedAt(session.updatedAt)),
          ];
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text('会话', style: Theme.of(context).textTheme.titleMedium),
                const Spacer(),
                _validityChip(context),
              ],
            ),
            const Divider(),
            for (final (label, value) in rows)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  children: [
                    SizedBox(width: 110, child: Text(label)),
                    Expanded(
                      child: SelectableText(
                        value,
                        style: const TextStyle(fontFamily: 'monospace'),
                      ),
                    ),
                  ],
                ),
              ),
            if (devMode && session.steps.isNotEmpty) ...[
              const Divider(),
              Text('步骤', style: Theme.of(context).textTheme.titleSmall),
              for (final s in session.steps)
                Text(
                  '${s.index}/${s.total} ${s.name} — ${s.detail}${s.ok ? '' : '（非致命）'}',
                ),
            ],
          ],
        ),
      ),
    );
  }
}
