import 'package:flutter/material.dart';

import '../app_state.dart';
import '../rust/api/auth.dart';
import '../rust/api/device.dart';
import '../rust/dto.dart';

/// 开发者选项：虚拟设备档案管理。
///
/// 普通用户不需要这一页——登录会自动按通行证标识建档。这里暴露完整的
/// 档案生命周期（列表 / 按模板创建 / getFp 注册 / 删除），供调试与迁移用。
class DevScreen extends StatefulWidget {
  const DevScreen({super.key, required this.state});

  final AppState state;

  @override
  State<DevScreen> createState() => _DevScreenState();
}

class _DevScreenState extends State<DevScreen> {
  final _accountController = TextEditingController();
  String _template = 'xiaomi14';
  DeviceSummary? _device;
  bool _busy = false;
  List<DeviceSummary> _profiles = const [];
  SessionSummary? _session;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _refreshList());
  }

  @override
  void dispose() {
    _accountController.dispose();
    super.dispose();
  }

  Future<void> _refreshList() async {
    try {
      final list = await deviceList(core: widget.state.core);
      if (!mounted) return;
      SessionSummary? session;
      final account = widget.state.account;
      if (account.isNotEmpty) {
        try {
          session = await authShow(core: widget.state.core, account: account);
        } catch (_) {}
      }
      setState(() {
        _profiles = list;
        _session = session;
      });
    } catch (e) {
      if (mounted) widget.state.setStatus('读取档案列表失败：$e');
    }
  }

  Future<void> _delete(DeviceSummary device) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除设备档案'),
        content: Text(
          '删除账号「${device.account}」的设备档案、注册、会话与审批策略？\n'
          '此操作不可恢复（审计日志保留）。',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    setState(() => _busy = true);
    try {
      await deviceDelete(core: widget.state.core, account: device.account);
      if (!mounted) return;
      if (widget.state.account == device.account) {
        widget.state.selectAccount('');
      }
      if (_device?.account == device.account) {
        setState(() => _device = null);
      }
      widget.state.setStatus('已删除：${device.account}');
    } catch (e) {
      if (mounted) widget.state.setStatus('删除失败：$e');
    } finally {
      await _refreshList();
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _run(Future<DeviceSummary> Function() action) async {
    setState(() => _busy = true);
    try {
      final device = await action();
      if (!mounted) return;
      setState(() => _device = device);
      widget.state.selectAccount(device.account);
      widget.state.setStatus('设备档案：${device.model}');
    } catch (e) {
      if (!mounted) return;
      widget.state.setStatus('失败：$e');
    } finally {
      await _refreshList();
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _create({required bool force}) async {
    final account = _accountController.text.trim();
    if (account.isEmpty) {
      widget.state.setStatus('请先填写账号标签');
      return;
    }
    final core = widget.state.core;
    await _run(
      () => deviceCreate(
        core: core,
        account: account,
        template: _template,
        force: force,
      ),
    );
  }

  Future<void> _register() async {
    final account = _accountController.text.trim();
    if (account.isEmpty) {
      widget.state.setStatus('请先填写账号标签');
      return;
    }
    try {
      setState(() => _busy = true);
      final result = await deviceRegister(core: widget.state.core, account: account);
      if (!mounted) return;
      widget.state.setStatus(
        'getFp 成功：fp ${result.deviceFpMasked}（${result.deviceFpLen} 字符）',
      );
    } catch (e) {
      if (mounted) widget.state.setStatus('注册失败：$e');
    } finally {
      await _refreshList();
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final info = widget.state.info;
    final templates = info?.deviceTemplates ?? const ['xiaomi14'];

    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('开发者选项', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        if (widget.state.guide)
          const Text(
            '虚拟设备档案管理。普通流程不需要这一页：登录会按通行证标识自动建档。'
            '这里的标签也可用于命令行工具，两端共享同一存储。',
          ),
        const SizedBox(height: 20),
        Row(
          children: [
            const Icon(Icons.folder_shared_outlined, size: 20),
            const SizedBox(width: 6),
            Text('已有档案', style: Theme.of(context).textTheme.titleMedium),
            const Spacer(),
            IconButton(
              onPressed: _busy ? null : _refreshList,
              tooltip: '刷新列表',
              icon: const Icon(Icons.refresh),
            ),
          ],
        ),
        const SizedBox(height: 8),
        if (_profiles.isEmpty)
          const Text('暂无档案。')
        else
          Card(
            child: Column(
              children: [
                for (final d in _profiles)
                  ListTile(
                    leading: Icon(
                      d.registered
                          ? Icons.verified_user_outlined
                          : Icons.devices_outlined,
                      color: d.registered ? Colors.green : null,
                    ),
                    title: Text(
                      '${d.account}　${d.brand} ${d.model}',
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      '${d.gameBiz}　${d.registered ? '已注册 getFp' : '未注册'}'
                      '　id ${d.deviceIdMasked}',
                      overflow: TextOverflow.ellipsis,
                    ),
                    onTap: () {
                      _accountController.text = d.account;
                      widget.state.selectAccount(d.account);
                      setState(() => _device = d);
                    },
                    trailing: IconButton(
                      icon: const Icon(Icons.delete_outline),
                      tooltip: '删除档案',
                      onPressed: _busy ? null : () => _delete(d),
                    ),
                  ),
              ],
            ),
          ),
        const SizedBox(height: 24),
        TextField(
          controller: _accountController,
          decoration: const InputDecoration(
            labelText: '本地账号标签',
            helperText: '仅本机使用，例如 acct-1',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 16),
        DropdownButtonFormField<String>(
          initialValue: _template,
          decoration: const InputDecoration(
            labelText: '设备模板',
            border: OutlineInputBorder(),
          ),
          items: [
            for (final t in templates) DropdownMenuItem(value: t, child: Text(t)),
          ],
          onChanged: (v) => setState(() => _template = v ?? _template),
        ),
        const SizedBox(height: 20),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            FilledButton.icon(
              onPressed: _busy ? null : () => _create(force: false),
              icon: const Icon(Icons.add),
              label: const Text('创建设备档案'),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : () => _create(force: true),
              icon: const Icon(Icons.refresh),
              label: const Text('重建（丢弃旧注册）'),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : _register,
              icon: const Icon(Icons.cloud_upload_outlined),
              label: const Text('注册到米游社 getFp'),
            ),
          ],
        ),
        if (_busy) ...[
          const SizedBox(height: 20),
          const LinearProgressIndicator(),
        ],
        const SizedBox(height: 24),
        if (_device != null) _DeviceCard(device: _device!),
        const SizedBox(height: 16),
        if (_session != null) _SessionDetailCard(session: _session!),
        if (_session == null)
          const Text('当前账号无会话。', style: TextStyle(color: Colors.grey)),
      ],
    );
  }
}

class _DeviceCard extends StatelessWidget {
  const _DeviceCard({required this.device});

  final DeviceSummary device;

  @override
  Widget build(BuildContext context) {
    final rows = <(String, String)>[
      ('账号', device.account),
      ('机型', '${device.brand} ${device.model}'),
      ('系统', 'Android ${device.androidVersion} (SDK ${device.sdkVersion})'),
      ('业务', device.gameBiz),
      ('device_id', '${device.deviceIdMasked}  (${device.deviceIdLen})'),
      ('device_fp', '${device.deviceFpMasked}  (${device.deviceFpLen})'),
      ('注册', device.registered ? '已注册' : '未注册'),
    ];
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('当前档案', style: Theme.of(context).textTheme.titleMedium),
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
          ],
        ),
      ),
    );
  }
}


/// 会话详情（掩码）——原本在登录页的信息，收进开发者页。
class _SessionDetailCard extends StatelessWidget {
  const _SessionDetailCard({required this.session});

  final SessionSummary session;

  @override
  Widget build(BuildContext context) {
    final rows = <(String, String)>[
      ('账号', session.account),
      ('stoken', '${session.stokenLen} 字符（前缀 ${session.stokenPrefix}）'),
      ('登录 mid', session.midMasked),
      ('扫码 mid', session.qrMidMasked),
      ('cookie_token', '${session.cookieTokenLen} 字符'),
      ('ltoken', '${session.ltokenLen} 字符'),
      ('更新时间', session.updatedAt),
    ];
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('会话详情', style: Theme.of(context).textTheme.titleMedium),
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
          ],
        ),
      ),
    );
  }
}
