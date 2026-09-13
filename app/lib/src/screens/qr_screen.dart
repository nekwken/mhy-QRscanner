import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../app_state.dart';
import '../flags.dart';
import '../rust/api/qr.dart' as qr_api;
import '../rust/api/qr.dart';
import '../rust/api/race.dart';
import '../rust/api/settings.dart';
import '../rust/dto.dart';

/// 扫码竞速仪表盘：多源（B站直播间/屏幕捕获/截图/链接）并发监控，
/// 第一个稳定二维码即触发批准（或按源模式仅扫描）。
class QrScreen extends StatefulWidget {
  const QrScreen({super.key, required this.state});

  final AppState state;

  @override
  State<QrScreen> createState() => _QrScreenState();
}

/// 页签：`sources` 是所有监控来源（本机屏幕 + 各直播间，同场竞速）；
/// `bilibili` 只管登记直播间；`image`/`url` 是一次性单路扫描。
enum _SourceKind { sources, bilibili, image, url }

/// 一个已登记的直播间（持久化于 settings/rooms）。竞速里的「本机屏幕」
/// 也用同一个结构承载实时状态，只是 `kind == 'screen'`、不进房间列表。
class RoomEntry {
  RoomEntry({
    required this.roomId,
    required this.label,
    this.enabled = true,
    this.mode = 'approve',
    this.kind = 'bilibili',
  });

  String roomId;
  String label;
  bool enabled;
  String mode; // approve | scan
  String kind; // bilibili | screen

  /// 竞速卡片上的来源标签：直播 / 屏幕。
  String get sourceLabel => kind == 'screen' ? '屏幕' : '直播';

  IconData get sourceIcon =>
      kind == 'screen' ? Icons.desktop_windows_outlined : Icons.live_tv_outlined;

  // 竞速实时状态（非持久化）
  bool alive = false;
  bool winner = false;
  bool scanned = false;
  bool approved = false;
  String? error;
  String? pendingTicket;
  List<String> pendingTokenTypes = <String>[];
  int frames = 0;
  /// 解出过二维码的帧数。
  int decodedFrames = 0;
  /// 最近一帧的解码说明，如「本帧未解出二维码」。
  String hint = '';
  String resolution = '';
  double fps = 0;
  int? pingMs;

  Map<String, dynamic> toJson() => {
        'roomId': roomId,
        'label': label,
        'enabled': enabled,
        'mode': mode,
      };

  static RoomEntry fromJson(Map<String, dynamic> j) => RoomEntry(
        roomId: (j['roomId'] ?? '').toString(),
        label: (j['label'] ?? '').toString(),
        enabled: (j['enabled'] ?? true) == true,
        mode: (j['mode'] ?? 'approve').toString(),
      );
}

class _QrScreenState extends State<QrScreen> {
  final _urlController = TextEditingController();
  final _imageController = TextEditingController();
  final _newRoomController = TextEditingController();
  final _newLabelController = TextEditingController();

  _SourceKind _kind = _SourceKind.sources;
  bool _busy = false;
  ApprovalOutcome? _outcome;
  /// 待批准票据所属的直播房间（竞速来源）；单路来源时为 null。
  RoomEntry? _pendingRoom;
  /// 竞速结果（结果卡 + 弹窗）每轮只呈现一次。
  bool _raceOutcomeShown = false;

  List<RoomEntry> _rooms = <RoomEntry>[];
  /// 把本机屏幕作为竞速来源（与直播同场竞速，先到先得）。
  final RoomEntry _screenWatcher =
      RoomEntry(roomId: '', label: '本机屏幕', kind: 'screen', enabled: false);
  int? _raceId;
  Timer? _raceTimer;
  String _raceStatus = '';
  bool _raceUnlimitedWait = true;
  int _raceWaitSeconds = 60;

  bool _popupOnScan = true;
  bool _popupOnApprove = true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _initSettings());
  }

  @override
  void dispose() {
    _raceTimer?.cancel();
    _urlController.dispose();
    _imageController.dispose();
    _newRoomController.dispose();
    _newLabelController.dispose();
    super.dispose();
  }

  Future<void> _initSettings() async {
    try {
      final core = widget.state.core;
      final scanFlag = await flagGet(core: core, key: kFlagPopupOnScan);
      final approveFlag = await flagGet(core: core, key: kFlagPopupOnApprove);
      final rooms = await textGet(core: core, key: kTextRooms);
      final screen = await textGet(core: core, key: kTextScreenWatcher);
      if (!mounted) return;
      setState(() {
        _popupOnScan = scanFlag.updatedAt.isEmpty ? true : scanFlag.enabled;
        _popupOnApprove =
            approveFlag.updatedAt.isEmpty ? true : approveFlag.enabled;
        _rooms = _parseRooms(rooms.value);
        if (screen.value.isNotEmpty) {
          try {
            final j = jsonDecode(screen.value) as Map<String, dynamic>;
            _screenWatcher.enabled = j['enabled'] == true;
            _screenWatcher.mode = (j['mode'] ?? 'approve').toString();
          } catch (_) {}
        }
      });
    } catch (_) {}
  }

  List<RoomEntry> _parseRooms(String raw) {
    if (raw.isEmpty) return <RoomEntry>[];
    try {
      final list = jsonDecode(raw) as List<dynamic>;
      return list
          .map((e) => RoomEntry.fromJson((e as Map).cast<String, dynamic>()))
          .toList();
    } catch (_) {
      return <RoomEntry>[];
    }
  }

  Future<void> _saveRooms() async {
    await textSet(
      core: widget.state.core,
      key: kTextRooms,
      value: jsonEncode([for (final r in _rooms) r.toJson()]),
    );
  }

  Future<void> _saveScreenWatcher() async {
    await textSet(
      core: widget.state.core,
      key: kTextScreenWatcher,
      value: jsonEncode({
        'enabled': _screenWatcher.enabled,
        'mode': _screenWatcher.mode,
      }),
    );
  }

  String get _account => widget.state.account;

  QrSource? get _source {
    switch (_kind) {
      case _SourceKind.url:
        final url = _urlController.text.trim();
        return url.isEmpty ? null : QrSource.url(url);
      case _SourceKind.image:
        final path = _imageController.text.trim();
        return path.isEmpty ? null : QrSource.image(path);
      // 屏幕与直播都作为「源」页签里的监控源跑竞速，这里不再单独构造。
      case _SourceKind.sources:
      case _SourceKind.bilibili:
        return null;
    }
  }

  // ---------- 单路扫描（image/url） ----------

  Future<void> _run({required bool confirm}) async {
    if (_account.isEmpty) {
      widget.state.setStatus('请先在「登录」页登录米哈游通行证账号');
      return;
    }
    final source = _source;
    if (source == null) {
      widget.state.setStatus('请填写二维码来源');
      return;
    }
    setState(() => _busy = true);
    try {
      final outcome = await qrLoginGame(
        core: widget.state.core,
        account: _account,
        source: source,
        confirm: confirm,
      );
      if (!mounted) return;
      setState(() {
        _outcome = outcome;
        // 单路来源没有房间卡，票据的取消只影响这张结果卡。
        _pendingRoom = null;
      });
      widget.state.setStatus(
        confirm ? '已批准：${outcome.appName}' : '已扫描（待手动批准）：${outcome.appName}',
      );
      await _afterOutcome(outcome);
    } catch (e) {
      if (mounted) widget.state.setStatus('失败：$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// 扫描/批准完成后的弹窗（受设置开关控制）。[room] 非空表示结果来自竞速源，
  /// 手动批准成功后要同步刷新那张房间卡。
  Future<void> _afterOutcome(ApprovalOutcome outcome, {RoomEntry? room}) async {
    if (outcome.approved && _popupOnApprove) {
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (dialogContext) => AlertDialog(
          title: const Text('已批准登录'),
          content: Text(
            '游戏：${_display(outcome.appName)}\n'
            '账号：${_display(outcome.accountDispName)}\n'
            '来源：${outcome.source}',
          ),
          actions: [
            FilledButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('知道了'),
            ),
          ],
        ),
      );
    } else if (!outcome.approved &&
        outcome.pendingTicket != null &&
        _popupOnScan) {
      await _showPendingApprovalDialog(outcome, room: room);
    }
  }

  /// 仅扫描成功后的手动批准弹窗：批准与取消并列，取消即不批准（票据作废）。
  Future<void> _showPendingApprovalDialog(
    ApprovalOutcome outcome, {
    RoomEntry? room,
  }) async {
    if (!mounted) return;
    _pendingRoom = room;
    final approved = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('扫描成功——是否批准登录'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('游戏：${_display(outcome.appName)}'),
            Text('账号：${_display(outcome.accountDispName)}'),
            const SizedBox(height: 8),
            Text('来源：${outcome.source}'),
            const SizedBox(height: 8),
            const Text('批准后目标游戏客户端会立即以该账号登录；取消则什么都不做。'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: const Text('批准登录'),
          ),
        ],
      ),
    );
    if (approved != true || !mounted) {
      if (mounted && approved == false) {
        _dropPending(outcome, room);
        widget.state.setStatus('已取消批准（未登录任何账号）');
      }
      return;
    }
    try {
      final done = await qr_api.qrConfirmPending(
        core: widget.state.core,
        account: _account,
        ticket: outcome.pendingTicket!,
        tokenTypes: outcome.pendingTokenTypes,
      );
      if (!mounted) return;
      // 确认接口只做批准，不带游戏/账号信息：沿用扫描阶段已知的值。
      final merged = _mergeOutcome(outcome, done);
      setState(() {
        _outcome = merged;
        _pendingRoom = null;
        if (room != null) {
          room.approved = true;
          room.scanned = true;
          room.pendingTicket = null;
          room.pendingTokenTypes = <String>[];
          _raceStatus = '已批准（手动确认）：${room.label.isEmpty ? room.roomId : room.label}'
              '——游戏客户端正在登录你的账号';
        }
      });
      widget.state.setStatus('已批准（手动确认）');
      await _afterOutcome(merged, room: room);
    } catch (e) {
      if (mounted) widget.state.setStatus('批准失败：$e');
    }
  }

  /// 放弃一张待批准票据：只清本地状态，服务端票据本就未批准。
  void _dropPending(ApprovalOutcome outcome, RoomEntry? room) {
    if (!mounted) return;
    setState(() {
      _pendingRoom = null;
      if (room != null) {
        room.pendingTicket = null;
        room.pendingTokenTypes = <String>[];
      }
      if (_outcome?.pendingTicket == outcome.pendingTicket) _outcome = null;
    });
  }

  /// `done` 缺字段时回退到 `base`（扫描阶段的已知信息）。
  ApprovalOutcome _mergeOutcome(ApprovalOutcome base, ApprovalOutcome done) =>
      ApprovalOutcome(
        steps: done.steps,
        appName: done.appName.isEmpty ? base.appName : done.appName,
        accountDispName: done.accountDispName.isEmpty
            ? base.accountDispName
            : done.accountDispName,
        riskNote: done.riskNote.isEmpty ? base.riskNote : done.riskNote,
        scanned: done.scanned,
        approved: done.approved,
        source: base.source,
        pendingTicket: done.pendingTicket,
        pendingTokenTypes: done.pendingTokenTypes,
      );

  static String _display(String v) => v.isEmpty ? '（未识别）' : v;

  // ---------- 多房间竞速 ----------

  Future<void> _startRace() async {
    if (_account.isEmpty) {
      widget.state.setStatus('请先在「登录」页登录米哈游通行证账号');
      return;
    }
    final enabled =
        _rooms.where((r) => r.enabled && r.roomId.trim().isNotEmpty).toList();
    final withScreen = _screenWatcher.enabled;
    if (enabled.isEmpty && !withScreen) {
      widget.state.setStatus('请先启用至少一个来源：直播间或本机屏幕');
      return;
    }
    try {
      final id = await qrRaceStart(
        core: widget.state.core,
        account: _account,
        specs: [
          if (withScreen)
            WatcherSpecDto(
              kind: 'screen',
              label: _screenWatcher.label,
              mode: _screenWatcher.mode,
            ),
          for (final r in enabled)
            WatcherSpecDto(
              kind: 'bilibili',
              room: r.roomId,
              label: r.label.isEmpty ? r.roomId : r.label,
              mode: r.mode,
            ),
        ],
        waitSecs: _raceUnlimitedWait ? null : BigInt.from(_raceWaitSeconds),
      );
      setState(() {
        _raceId = id;
        _raceOutcomeShown = false;
        _outcome = null;
        _pendingRoom = null;
        _raceStatus = '抢码进行中：${enabled.length} 个直播间…';
      });
      widget.state.setStatus('多直播间抢码已启动');
      _raceTimer?.cancel();
      _raceTimer = Timer.periodic(const Duration(milliseconds: 600), (_) async {
        final id = _raceId;
        if (id == null) return;
        try {
          final s = await qrRaceStatus(id: id);
          if (!mounted) return;
          _applyRaceStatus(s);
          // 结果落地（已批准 / 待批准 / 出错）或所有源都退出后才收工：
          // 否则胜者刚锁定、批准结果还没回来就停止轮询，弹窗与结果卡都会落空。
          if (!s.running && (_raceOutcomeShown || s.watchersAlive == 0)) {
            _raceTimer?.cancel();
            setState(() => _raceId = null);
          }
        } catch (_) {}
      });
    } catch (e) {
      widget.state.setStatus('抢码启动失败：$e');
    }
  }

  void _applyRaceStatus(RaceStatusDto s) {
    setState(() {
      // 直播房间卡与「本机屏幕」卡共用一套实时状态字段。
      final cards = <RoomEntry>[..._rooms, _screenWatcher];
      for (final w in s.watchers) {
        for (final r in cards) {
          if ((r.label.isEmpty ? r.roomId : r.label) != w.label) continue;
          r.alive = w.alive;
          r.winner = w.winner;
          r.scanned = w.scanned;
          r.approved = w.approved;
          r.error = w.error;
          r.pendingTicket = w.pendingTicket;
          r.pendingTokenTypes = w.pendingTokenTypes;
          r.frames = w.frames.toInt();
          r.decodedFrames = w.decodedFrames.toInt();
          r.hint = w.hint;
          r.resolution = w.resolution;
          r.fps = w.fps;
          r.pingMs = w.pingMs?.toInt();
        }
      }
      final winners = s.watchers.where((w) => w.winner).toList();
      final winner = winners.isEmpty ? null : winners.first.label;
      _raceStatus = s.approved
          ? '已批准！来源：$winner——游戏客户端正在登录你的账号'
          : s.watchers.any((w) => w.pendingTicket != null)
              ? '已扫描（来自 $winner），等待手动批准'
              : s.running
                  ? '抢码进行中：存活 ${s.watchersAlive}/${s.watchersTotal} 个源'
                  : '抢码结束${s.error == null ? '' : '：${s.error}'}';
    });
    // 胜者出结果：写结果卡 + 弹对应弹窗（每轮竞速只呈现一次）。
    if (_raceOutcomeShown) return;
    final settled = s.watchers.where(
      (w) => w.winner && (w.scanned || w.approved || w.pendingTicket != null || w.error != null),
    );
    if (settled.isEmpty) return;
    final w = settled.first;
    _raceOutcomeShown = true;
    if (w.error != null && !w.scanned) {
      widget.state.setStatus('抢码失败：${w.error}');
      return;
    }
    final room = _roomForLabel(w.label);
    final outcome = ApprovalOutcome(
      steps: const [],
      appName: w.appName,
      accountDispName: w.accountDispName,
      riskNote: w.riskNote,
      scanned: w.scanned,
      approved: w.approved,
      source: '${w.kind == 'screen' ? '屏幕捕获' : '直播'}·${w.label}',
      pendingTicket: w.pendingTicket,
      pendingTokenTypes: w.pendingTokenTypes,
    );
    setState(() {
      _outcome = outcome;
      _pendingRoom = outcome.pendingTicket == null ? null : room;
    });
    unawaited(_afterOutcome(outcome, room: room));
  }

  RoomEntry? _roomForLabel(String label) {
    for (final r in _rooms) {
      if ((r.label.isEmpty ? r.roomId : r.label) == label) return r;
    }
    return null;
  }

  Future<void> _stopRace() async {
    final id = _raceId;
    if (id == null) return;
    try {
      await qrRaceStop(id: id);
    } finally {
      _raceTimer?.cancel();
      if (mounted) {
        setState(() {
          _raceId = null;
          for (final r in [..._rooms, _screenWatcher]) {
            r.alive = false;
            r.winner = false;
            r.scanned = false;
            r.approved = false;
            r.pendingTicket = null;
            r.pendingTokenTypes = <String>[];
          }
          _raceStatus = '$_raceStatus（已手动停止）';
        });
        widget.state.setStatus('多直播间抢码已停止');
      }
    }
  }

  // ---------- 直播间管理 ----------

  Future<void> _addRoom() async {
    final room = _newRoomController.text.trim();
    if (room.isEmpty) {
      widget.state.setStatus('请填写直播间号');
      return;
    }
    final label = _newLabelController.text.trim();
    setState(() {
      _rooms.add(RoomEntry(roomId: room, label: label));
      _newRoomController.clear();
      _newLabelController.clear();
    });
    await _saveRooms();
    widget.state
        .setStatus(label.isEmpty ? '已添加直播间 $room' : '已添加直播间 $room（$label）');
  }

  Future<void> _deleteRoom(RoomEntry room) async {
    setState(() => _rooms.remove(room));
    await _saveRooms();
    widget.state.setStatus('已删除直播间 ${room.roomId}');
  }

  // ---------- 构建 ----------

  Widget _waitSelector({
    required bool unlimited,
    required int seconds,
    required ValueChanged<bool> onUnlimited,
    required ValueChanged<int> onSeconds,
  }) =>
      Row(
        children: [
          const Text('捕获时长'),
          const SizedBox(width: 12),
          SegmentedButton<bool>(
            showSelectedIcon: false,
            segments: const [
              ButtonSegment(
                value: true,
                label: SizedBox(width: 84, child: Text('无限', textAlign: TextAlign.center)),
              ),
              ButtonSegment(
                value: false,
                label: SizedBox(width: 84, child: Text('倒计时', textAlign: TextAlign.center)),
              ),
            ],
            selected: {unlimited},
            onSelectionChanged: (v) => onUnlimited(v.first),
          ),
          if (!unlimited) ...[
            const SizedBox(width: 12),
            SizedBox(
              width: 200,
              child: Slider(
                value: seconds.toDouble(),
                min: 5,
                max: 120,
                divisions: 23,
                label: '$seconds 秒',
                onChanged: (v) => onSeconds(v.round()),
              ),
            ),
            Text('$seconds 秒'),
          ],
        ],
      );

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('扫码', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        if (widget.state.guide)
          const Text(
            '「源」页签登记所有监控源：本机屏幕与各直播间同场竞速，第一个出现稳定二维码的源'
            '按自己的模式行动——「扫描并批准」自动登录，「仅扫描」先弹窗确认再批准。'
            '直播间在「B站直播」页签添加。',
          ),
        if (_busy || _raceId != null) ...[
          const SizedBox(height: 16),
          _runningStrip(context),
        ],
        const SizedBox(height: 20),
        SegmentedButton<_SourceKind>(
          showSelectedIcon: false,
          segments: const [
            ButtonSegment(
              value: _SourceKind.sources,
              label: Text('源'),
              icon: Icon(Icons.hub_outlined),
            ),
            ButtonSegment(
              value: _SourceKind.bilibili,
              label: Text('B站直播'),
              icon: Icon(Icons.live_tv_outlined),
            ),
            ButtonSegment(
              value: _SourceKind.image,
              label: Text('截图文件'),
              icon: Icon(Icons.image_outlined),
            ),
            ButtonSegment(
              value: _SourceKind.url,
              label: Text('二维码链接'),
              icon: Icon(Icons.link),
            ),
          ],
          selected: {_kind},
          onSelectionChanged: (v) => setState(() => _kind = v.first),
        ),
        const SizedBox(height: 16),
        switch (_kind) {
          _SourceKind.sources => _buildRoomDashboard(context),
          _SourceKind.bilibili => _buildLiveRooms(context),
          _SourceKind.image => TextField(
              controller: _imageController,
              decoration: const InputDecoration(
                labelText: '截图文件路径',
                border: OutlineInputBorder(),
              ),
            ),
          _SourceKind.url => TextField(
              controller: _urlController,
              decoration: const InputDecoration(
                labelText: '游戏二维码 URL（qr_code_in_game.html）',
                border: OutlineInputBorder(),
              ),
            ),
        },
        if (_kind == _SourceKind.image || _kind == _SourceKind.url) ...[
          const SizedBox(height: 20),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              OutlinedButton.icon(
                onPressed: _busy ? null : () => _run(confirm: false),
                icon: const Icon(Icons.search),
                label: const Text('仅扫描'),
              ),
              FilledButton.icon(
                onPressed: _busy ? null : () => _run(confirm: true),
                icon: const Icon(Icons.verified_outlined),
                label: const Text('扫描并批准'),
              ),
            ],
          ),
        ],
        if (_busy && _kind != _SourceKind.sources) ...[
          const SizedBox(height: 20),
          const LinearProgressIndicator(),
          const SizedBox(height: 8),
          const Text('正在执行…'),
        ],
        const SizedBox(height: 24),
        if (_outcome != null) ...[
          _OutcomeCard(outcome: _outcome!),
          if (_outcome!.pendingTicket != null && !_busy) ...[
            const SizedBox(height: 12),
            Wrap(
              spacing: 12,
              children: [
                FilledButton.icon(
                  onPressed: () => _showPendingApprovalDialog(
                    _outcome!,
                    room: _pendingRoom,
                  ),
                  icon: const Icon(Icons.verified_outlined),
                  label: const Text('批准登录'),
                ),
                OutlinedButton.icon(
                  onPressed: () {
                    _dropPending(_outcome!, _pendingRoom);
                    widget.state.setStatus('已取消批准（未登录任何账号）');
                  },
                  icon: const Icon(Icons.close),
                  label: const Text('取消'),
                ),
              ],
            ),
          ],
        ],
      ],
    );
  }

  /// 直播间竞速仪表盘：卡片列表 + 添加行 + 启停按钮。
  Widget _buildRoomDashboard(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('监控源', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        // 本机屏幕与每个直播间都是竞速的一个源，卡片上标明来源、先到先得。
        _roomCard(context, _screenWatcher, isScreen: true),
        if (_rooms.isEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text(widget.state.guide ? '暂无直播间。添加后会持久化保存，重启不丢。' : '暂无直播间。'),
          )
        else
          for (final room in _rooms) _roomCard(context, room),
        const SizedBox(height: 16),
        _waitSelector(
          unlimited: _raceUnlimitedWait,
          seconds: _raceWaitSeconds,
          onUnlimited: (v) => setState(() => _raceUnlimitedWait = v),
          onSeconds: (v) => setState(() => _raceWaitSeconds = v),
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 12,
          children: [
            FilledButton.icon(
              onPressed: _raceId == null ? _startRace : null,
              icon: const Icon(Icons.play_arrow),
              label: const Text('开始捕获'),
            ),
            OutlinedButton.icon(
              onPressed: _raceId == null ? null : _stopRace,
              icon: const Icon(Icons.stop),
              label: const Text('停止捕获'),
            ),
          ],
        ),
        if (_raceStatus.isNotEmpty) ...[
          const SizedBox(height: 8),
          Text(_raceStatus),
        ],
        if (widget.state.guide) ...[
          const SizedBox(height: 8),
          Text(
            '每个启用的源各跑一个监控：直播（需 ffmpeg 在 PATH 或设 MHYQR_FFMPEG）与'
            '本机屏幕同场竞速，第一个出现稳定二维码的源按它的模式行动——'
            '「扫描并批准」自动批准，「仅扫描」先出批准弹窗（可批准也可取消）。',
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ],
    );
  }

  /// B站直播页签：只负责登记直播间。启用、模式与删除都在「源」页签的卡片上。
  Widget _buildLiveRooms(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('添加直播间', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        if (widget.state.guide)
          const Text(
            '填直播间号（标签选填）后点「添加」，会持久化保存、重启不丢。'
            '登记好的直播间出现在「源」页签，在那里启用、切换模式或删除。',
          ),
        const SizedBox(height: 12),
        Row(
          children: [
            SizedBox(
              width: 150,
              child: TextField(
                controller: _newRoomController,
                keyboardType: TextInputType.number,
                decoration: const InputDecoration(
                  labelText: '直播间号',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _newLabelController,
                decoration: const InputDecoration(
                  labelText: '标签（选填）',
                  hintText: '如：主播A',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: _addRoom,
              icon: const Icon(Icons.add),
              label: const Text('添加'),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Text(
          widget.state.guide
              ? '已登记 ${_rooms.length} 个直播间。拉流需要 ffmpeg 在 PATH（或设 MHYQR_FFMPEG）。'
              : '已登记 ${_rooms.length} 个直播间。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }

  /// 运行中的捕获源一览（每个页签都可见）：单路捕获与竞速各源分开标注，
  /// 图标 + 来源名区分「屏幕」与「直播」。
  Widget _runningStrip(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final chips = <Widget>[
      if (_busy && (_kind == _SourceKind.image || _kind == _SourceKind.url))
        _sourceChip(
          context,
          icon: _kind == _SourceKind.image
              ? Icons.image_outlined
              : Icons.link,
          title: _kind == _SourceKind.image ? '单路扫描·截图' : '单路扫描·链接',
          detail: '进行中',
          color: scheme.primary,
        ),
      if (_raceId != null)
        for (final r in [_screenWatcher, ..._rooms])
          if (r.enabled)
            _sourceChip(
              context,
              icon: r.sourceIcon,
              title: '${r.sourceLabel}·'
                  '${r.kind == 'screen' ? '竞速' : (r.label.isEmpty ? r.roomId : r.label)}',
              detail: _stateText(r),
              color: _stateColor(r, scheme),
            ),
    ];
    if (chips.isEmpty) return const SizedBox.shrink();
    return Wrap(spacing: 8, runSpacing: 8, children: chips);
  }

  Widget _sourceChip(
    BuildContext context, {
    required IconData icon,
    required String title,
    required String detail,
    required Color color,
  }) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: color.withValues(alpha: 0.35)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 6),
          Text(
            title,
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: scheme.onSurface,
            ),
          ),
          const SizedBox(width: 6),
          Text(detail, style: TextStyle(fontSize: 12, color: color)),
        ],
      ),
    );
  }

  /// 一个竞速源的状态短句（卡片与运行条共用）。
  String _stateText(RoomEntry room) {
    if (room.approved) return '已批准登录 ✓';
    if (room.pendingTicket != null) return '已扫描，等待手动批准';
    if (room.winner) return '胜者：二维码已锁定';
    if (room.error != null) return '错误：${room.error}';
    if (room.alive) return '监控中…';
    return '待命';
  }

  Color _stateColor(RoomEntry room, ColorScheme scheme) {
    if (room.approved) return Colors.green;
    if (room.pendingTicket != null || room.winner) return scheme.primary;
    if (room.error != null) return scheme.error;
    return scheme.onSurfaceVariant;
  }

  /// 来源标签（直播 / 屏幕）：两者同场竞速时用来区分。
  Widget _sourceTag(BuildContext context, RoomEntry room) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: scheme.outlineVariant.withValues(alpha: 0.6)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(room.sourceIcon, size: 12, color: scheme.onSurfaceVariant),
          const SizedBox(width: 4),
          Text(
            room.sourceLabel,
            style: TextStyle(fontSize: 10.5, color: scheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }

  Widget _roomCard(BuildContext context, RoomEntry room, {bool isScreen = false}) {
    final scheme = Theme.of(context).colorScheme;
    final status = _stateText(room);
    final statusColor = _stateColor(room, scheme);
    final name = isScreen
        ? room.label
        : (room.label.isEmpty ? '直播间 ${room.roomId}' : room.label);
    final metrics = <String>[
      if (!isScreen) '房间 ${room.roomId}',
      if (isScreen && room.frames > 0) '${room.frames} 帧',
      if (room.alive && room.resolution.isNotEmpty) room.resolution,
      if (room.alive) '${room.fps.toStringAsFixed(1)} fps',
      if (room.alive && room.pingMs != null) '延迟 ${room.pingMs}ms',
      if (room.alive && room.hint.isNotEmpty) room.hint,
    ].join('　');
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Flexible(
                        child: Text(
                          name,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontWeight: FontWeight.w600),
                        ),
                      ),
                      const SizedBox(width: 8),
                      _sourceTag(context, room),
                    ],
                  ),
                  Text(
                    metrics.isEmpty ? status : '$metrics　$status',
                    style: TextStyle(fontSize: 12, color: statusColor),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 10),
            SegmentedButton<String>(
              showSelectedIcon: false,
              segments: const [
                ButtonSegment(
                  value: 'approve',
                  label: SizedBox(width: 84, child: Text('扫描并批准', textAlign: TextAlign.center)),
                ),
                ButtonSegment(
                  value: 'scan',
                  label: SizedBox(width: 84, child: Text('仅扫描', textAlign: TextAlign.center)),
                ),
              ],
              selected: {room.mode},
              onSelectionChanged: (v) {
                setState(() => room.mode = v.first);
                if (isScreen) {
                  _saveScreenWatcher();
                } else {
                  _saveRooms();
                }
              },
            ),
            const SizedBox(width: 10),
            Tooltip(
              message: isScreen ? '把本机屏幕加入/移出竞速' : '启用/停用该直播间',
              child: Switch(
                value: room.enabled,
                onChanged: (v) {
                  setState(() => room.enabled = v);
                  if (isScreen) {
                    _saveScreenWatcher();
                  } else {
                    _saveRooms();
                  }
                },
              ),
            ),
            if (isScreen)
              const SizedBox(width: 36)
            else
              IconButton(
                icon: const Icon(Icons.delete_outline),
                tooltip: '删除',
                onPressed: () => _deleteRoom(room),
              ),
          ],
        ),
      ),
    );
  }
}

class _OutcomeCard extends StatelessWidget {
  const _OutcomeCard({required this.outcome});

  final ApprovalOutcome outcome;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('结果', style: Theme.of(context).textTheme.titleMedium),
            const Divider(),
            Text('游戏：${outcome.appName}'),
            Text('账号：${outcome.accountDispName}'),
            Text('来源：${outcome.source}'),
            if (outcome.riskNote.isNotEmpty) Text('风控备注：${outcome.riskNote}'),
            Text(
                '已扫描：${outcome.scanned ? '是' : '否'}　已批准：${outcome.approved ? '是' : '否'}'),
          ],
        ),
      ),
    );
  }
}
