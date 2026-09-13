import 'package:flutter/material.dart';

import '../app_state.dart';
import '../flags.dart';
import '../rust/api/settings.dart';
import '../rust/dto.dart';

/// Appearance, behaviour switches, the developer switch, and (in developer
/// mode) the tail of the audit log.
///
/// 批准模式不在这里设置：扫码页按来源选择「抢码」（自动批准）或「仅扫描」
/// （先弹窗确认）。本页只放全局开关；「最近操作」属于排查用的细节，仅在
/// 开发者模式下显示。
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({
    super.key,
    required this.state,
    this.dark = true,
    this.onToggleTheme,
  });

  final AppState state;
  final bool dark;
  final void Function(bool dark)? onToggleTheme;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  List<AuditEntryDto> _audit = const [];

  /// 策略当前对应的标签；与登录账号不一致时自动重读。
  FlagDto? _devMode;
  FlagDto? _closeToTray;
  FlagDto? _popupOnScan;
  FlagDto? _popupOnApprove;
  FlagDto? _guide;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  String get _account => widget.state.account;

  Future<void> _load() async {
    if (_busy) return;
    final account = _account;
    setState(() => _busy = true);
    try {
      final core = widget.state.core;
      final devMode = await flagGet(core: core, key: kFlagDevMode);
      final trayFlag = await flagGet(core: core, key: kFlagCloseToTray);
      final popupScan = await flagGet(core: core, key: kFlagPopupOnScan);
      final popupApprove = await flagGet(core: core, key: kFlagPopupOnApprove);
      final guide = await flagGet(core: core, key: kFlagGuide);
      // 审计记录只在开发者模式下显示，普通模式不读、不留痕。
      final audit = devMode.enabled
          ? await auditRecent(core: core, limit: 50)
          : const <AuditEntryDto>[];
      if (!mounted) return;
      setState(() {
        _audit = audit;
        _devMode = devMode;
        _closeToTray = trayFlag;
        _popupOnScan = popupScan;
        _popupOnApprove = popupApprove;
        _guide = guide;
      });
      widget.state.setDevMode(devMode.enabled);
      widget.state.setCloseToTray(trayFlag.enabled);
      widget.state.setPopupFlags(popupScan.enabled, popupApprove.enabled);
      widget.state.setGuide(guide.enabled);
      widget.state.setStatus(
        account.isEmpty
            ? '未登录'
            : devMode.enabled
                ? '已读取最近 ${audit.length} 条记录'
                : '设置已加载',
      );
    } catch (e) {
      if (mounted) widget.state.setStatus('读取失败：$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _setDevMode(bool value) async {
    setState(() => _busy = true);
    try {
      final flag = await flagSet(
        core: widget.state.core,
        key: kFlagDevMode,
        enabled: value,
      );
      if (!mounted) return;
      setState(() => _devMode = flag);
      widget.state.setDevMode(flag.enabled);
      widget.state.setStatus(value ? '已开启开发者选项' : '已关闭开发者选项');
    } catch (e) {
      if (mounted) widget.state.setStatus('保存失败：$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
    // 打开开发者选项时补读一次审计记录（普通模式不读）。
    if (value && mounted) await _load();
  }

  Future<void> _setCloseToTray(bool value) async {
    setState(() => _busy = true);
    try {
      final flag = await flagSet(
        core: widget.state.core,
        key: kFlagCloseToTray,
        enabled: value,
      );
      if (!mounted) return;
      setState(() => _closeToTray = flag);
      widget.state.setCloseToTray(flag.enabled);
      widget.state.setStatus(value ? '关闭时将最小化到托盘' : '关闭时直接退出');
    } catch (e) {
      if (mounted) widget.state.setStatus('保存失败：$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _setPopupOnScan(bool value) async {
    await _flagWrite(kFlagPopupOnScan, value);
    if (mounted) {
      setState(() => _popupOnScan = FlagDto(enabled: value, updatedAt: '1'));
      widget.state.setStatus(value ? '仅扫描成功后将弹出批准窗口' : '仅扫描后不弹批准窗口');
    }
  }

  Future<void> _setPopupOnApprove(bool value) async {
    await _flagWrite(kFlagPopupOnApprove, value);
    if (mounted) {
      setState(() => _popupOnApprove = FlagDto(enabled: value, updatedAt: '1'));
      widget.state.setStatus(value ? '批准成功后将弹出通知' : '批准成功后不弹通知');
    }
  }

  Future<void> _setGuide(bool value) async {
    await _flagWrite(kFlagGuide, value);
    if (mounted) {
      setState(() => _guide = FlagDto(enabled: value, updatedAt: '1'));
      // 各页面读的是 AppState，必须同步，否则开关拨了界面不变。
      widget.state.setGuide(value);
      widget.state.setStatus(value ? '已开启界面指引' : '已关闭界面指引');
    }
  }

  Future<void> _flagWrite(String key, bool value) async {
    setState(() => _busy = true);
    try {
      await flagSet(core: widget.state.core, key: key, enabled: value);
      if (key == kFlagPopupOnScan) {
        widget.state.setPopupFlags(value, widget.state.popupOnApprove);
      } else if (key == kFlagPopupOnApprove) {
        widget.state.setPopupFlags(widget.state.popupOnScan, value);
      }
    } catch (e) {
      if (mounted) widget.state.setStatus('保存失败：$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final info = widget.state.info;

    return AnimatedBuilder(
      animation: widget.state,
      builder: (context, _) {
        return ListView(
          padding: const EdgeInsets.all(24),
          children: [
            Text('设置', style: Theme.of(context).textTheme.headlineSmall),
            const SizedBox(height: 8),
            if (widget.state.guide) ...[
              const Text(
                '扫码页按来源选择批准方式：「扫描并批准」自动批准，「仅扫描」先弹窗让你确认。'
                '短信验证码与图形验证始终由你手动完成。',
              ),
              const SizedBox(height: 20),
            ] else
              const SizedBox(height: 8),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.dark_mode_outlined),
                title: const Text('深色外观'),
                subtitle: const Text('云母背景会跟随明暗切换'),
                value: widget.dark,
                onChanged: widget.onToggleTheme == null
                    ? null
                    : (v) => widget.onToggleTheme!(v),
              ),
            ),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.width_full_outlined),
                title: const Text('关闭时最小化到托盘'),
                subtitle: Text(
                  (_closeToTray?.enabled ?? widget.state.closeToTray)
                      ? '已开启：点关闭隐藏到系统托盘，托盘左键恢复、右键退出'
                      : '关闭。点关闭直接退出应用。',
                ),
                value: _closeToTray?.enabled ?? widget.state.closeToTray,
                onChanged: _busy ? null : _setCloseToTray,
              ),
            ),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.assignment_outlined),
                title: const Text('仅扫描成功后弹出批准窗口'),
                subtitle: const Text('扫描到二维码先弹窗让你确认，再批准登录'),
                value: _popupOnScan?.enabled ?? true,
                onChanged: _busy ? null : _setPopupOnScan,
              ),
            ),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.notifications_active_outlined),
                title: const Text('批准成功后弹出通知'),
                subtitle: const Text('显示游戏、账号与来源，点击关闭'),
                value: _popupOnApprove?.enabled ?? true,
                onChanged: _busy ? null : _setPopupOnApprove,
              ),
            ),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.help_outline),
                title: const Text('指引'),
                subtitle: const Text('开启后在界面显示操作说明与提示文本；关闭则只留操作与状态'),
                value: _guide?.enabled ?? widget.state.guide,
                onChanged: _busy ? null : _setGuide,
              ),
            ),
            const SizedBox(height: 12),
            Card(
              child: SwitchListTile(
                secondary: const Icon(Icons.code_rounded),
                title: const Text('开发者选项'),
                subtitle: Text(
                  (_devMode?.enabled ?? widget.state.devMode)
                      ? '已开启：导航栏出现「开发者」页（虚拟设备管理）'
                      : '关闭。普通用户只需登录与扫码，无需接触虚拟设备。',
                ),
                value: _devMode?.enabled ?? widget.state.devMode,
                onChanged: _busy ? null : _setDevMode,
              ),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                OutlinedButton.icon(
                  onPressed: _busy ? null : _load,
                  icon: const Icon(Icons.refresh),
                  label: const Text('刷新'),
                ),
              ],
            ),
            if (_busy) ...[
              const SizedBox(height: 16),
              const LinearProgressIndicator(),
            ],
            if (_devMode?.enabled ?? widget.state.devMode) ...[
              const SizedBox(height: 24),
              Row(
                children: [
                  const Icon(Icons.history, size: 20),
                  const SizedBox(width: 6),
                  Text('最近操作', style: Theme.of(context).textTheme.titleMedium),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                info == null
                    ? '审计文件是数据目录下的 audit.log（NDJSON）。'
                    : '数据目录：${info.dataDir}　审计文件：audit.log（NDJSON）。'
                        '仅记录账号标签、动作、结果与服务器返回码。',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(height: 12),
              if (_audit.isEmpty)
                const Text('暂无记录。')
              else
                Card(
                  child: Column(
                    children: [
                      for (final entry in _audit) _AuditRow(entry: entry),
                    ],
                  ),
                ),
            ],
          ],
        );
      },
    );
  }
}

/// Unix seconds → local `MM-DD HH:mm:ss`. Anything unparseable shows a dash.
String _formatTime(String unixSeconds) {
  final seconds = int.tryParse(unixSeconds);
  if (seconds == null) return '—';
  final t = DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
  String two(int v) => v.toString().padLeft(2, '0');
  return '${two(t.month)}-${two(t.day)} '
      '${two(t.hour)}:${two(t.minute)}:${two(t.second)}';
}

class _AuditRow extends StatelessWidget {
  const _AuditRow({required this.entry});

  final AuditEntryDto entry;

  @override
  Widget build(BuildContext context) {
    final retcode = entry.retcode;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            entry.ok ? Icons.check_circle_outline : Icons.error_outline,
            size: 18,
            color: entry.ok ? null : Theme.of(context).colorScheme.error,
          ),
          const SizedBox(width: 10),
          SizedBox(
            width: 104,
            child: Text(
              _formatTime(entry.at),
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
            ),
          ),
          SizedBox(
            width: 110,
            child: Text(entry.account, overflow: TextOverflow.ellipsis),
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('${entry.action}　${entry.detail}'),
                if (retcode != null)
                  Text(
                    'retcode=$retcode',
                    style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
