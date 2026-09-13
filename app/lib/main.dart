import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_acrylic/flutter_acrylic.dart' as acrylic;
import 'package:system_tray/system_tray.dart';
import 'package:window_manager/window_manager.dart';

import 'src/app_state.dart';
import 'src/rust/api/auth.dart' as auth_api;
import 'src/rust/api/race.dart' as race_api;
import 'src/flags.dart';
import 'src/rust/api/settings.dart' as settings_api;
import 'src/rust_loader.dart';
import 'src/screens/about_screen.dart';
import 'src/screens/dev_screen.dart';
import 'src/screens/login_screen.dart';
import 'src/screens/qr_screen.dart';
import 'src/screens/settings_screen.dart';
import 'src/theme.dart';

const _windowTitle = '米游抢码器';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  const options = WindowOptions(
    title: _windowTitle,
    minimumSize: Size(760, 560),
    titleBarStyle: TitleBarStyle.hidden,
  );
  await windowManager.waitUntilReadyToShow(options, () async {
    await windowManager.show();
    await windowManager.focus();
  });
  runApp(const QmdApp());
}

/// Windows shell over the Rust core, styled after WinUI / WPF-UI
/// (BetterGI-style): hidden title bar integrated with a translucent nav pane,
/// pill-selected destinations with an accent bar, and a Mica backdrop.
///
/// The core is created asynchronously; the shell shows a loading state until the
/// cdylib is loaded and `coreNew()` has run.
class QmdApp extends StatefulWidget {
  const QmdApp({super.key});

  @override
  State<QmdApp> createState() => _QmdAppState();
}

class _QmdAppState extends State<QmdApp> with WindowListener {
  final AppState _state = AppState();
  final SystemTray _tray = SystemTray();
  Object? _error;
  ThemeMode _themeMode = ThemeMode.dark;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    _bootstrap();
  }

  /// 关闭拦截：开启「关闭时最小化到托盘」后，关闭按钮/系统关闭都隐藏窗口，
  /// 由托盘图标恢复；未开启时直接关闭。
  @override
  void onWindowClose() {
    if (_state.closeToTray) {
      windowManager.hide();
      return;
    }
    // 未勾选「保留登录态」时，退出即清除当前账号会话。
    if (!_state.keepSession && _state.account.isNotEmpty && _state.ready) {
      auth_api
          .authLogout(core: _state.core, account: _state.account)
          .whenComplete(() => _exitApp());
      return;
    }
    _exitApp();
  }

  /// 退出应用。顺序很讲究：
  /// 1) 先把窗口藏起来——用户视角立刻消失，绝不见到「卡住一段时间的窗口」；
  /// 2) app_shutdown 停掉竞速源与单路扫描，让 ffmpeg 子进程随源线程退出被 kill
  ///    （直接 exit(0) 不会跑 Rust 析构，不先停会留下孤儿 ffmpeg 继续拉流）；
  /// 3) 撤托盘图标、清 preventClose，然后硬退出，不等引擎析构。
  Future<void> _exitApp() async {
    await windowManager.hide();
    try {
      await race_api.appShutdown();
    } catch (_) {}
    try {
      await _tray.destroy();
    } catch (_) {}
    await windowManager.setPreventClose(false);
    exit(0);
  }

  Future<void> _initTray() async {
    final exeDir = File(Platform.resolvedExecutable).parent.path;
    await _tray.initSystemTray(
      title: '米游抢码器',
      iconPath: '$exeDir\\data\\flutter_assets\\assets\\app_icon.ico',
      toolTip: '米游抢码器（后台运行中）',
    );
    final menu = Menu();
    await menu.buildFrom([
      MenuItemLabel(label: '显示主窗口', onClicked: (_) => _showFromTray()),
      MenuSeparator(),
      MenuItemLabel(label: '退出', onClicked: (_) => _exitApp()),
    ]);
    await _tray.setContextMenu(menu);
    _tray.registerSystemTrayEventHandler((eventName) {
      if (eventName == kSystemTrayEventClick) {
        _showFromTray();
      } else if (eventName == kSystemTrayEventRightClick) {
        _tray.popUpContextMenu();
      }
    });
  }

  Future<void> _showFromTray() async {
    await windowManager.show();
    await windowManager.focus();
  }

  Future<void> _bootstrap() async {
    // The core path is load-bearing; the backdrop is cosmetic and must never
    // block or fail the shell (it throws in environments without a real
    // window, e.g. widget tests).
    try {
      await initRust();
      await _state.init();
      final dev = await settings_api.flagGet(core: _state.core, key: kFlagDevMode);
      _state.setDevMode(dev.enabled);
      final tray = await settings_api.flagGet(core: _state.core, key: kFlagCloseToTray);
      _state.setCloseToTray(tray.enabled);
    } catch (e) {
      _error = e;
    }
    try {
      await acrylic.Window.initialize();
      await _applyBackdrop(_themeMode == ThemeMode.dark);
    } catch (_) {}
    // 托盘与关闭拦截同样是纯增强：失败不阻塞主流程
    try {
      await windowManager.setPreventClose(true);
      await _initTray();
    } catch (_) {}
    if (mounted) setState(() {});
  }

  /// Windows 11 云母 (Mica) backdrop; falls back to plain transparent on
  /// systems that do not support it.
  Future<void> _applyBackdrop(bool dark) async {
    try {
      await acrylic.Window.setEffect(
        effect: acrylic.WindowEffect.mica,
        dark: dark,
      );
    } catch (_) {
      try {
        await acrylic.Window.setEffect(effect: acrylic.WindowEffect.transparent);
      } catch (_) {}
    }
  }

  void _toggleTheme(bool dark) {
    setState(() => _themeMode = dark ? ThemeMode.dark : ThemeMode.light);
    _applyBackdrop(dark);
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: _windowTitle,
      theme: buildTheme(Brightness.light),
      darkTheme: buildTheme(Brightness.dark),
      themeMode: _themeMode,
      home: _error != null
          ? _LoadFailure(error: _error!)
          : (_state.ready
              ? _Shell(
                  state: _state,
                  themeMode: _themeMode,
                  onToggleTheme: _toggleTheme,
                )
              : const _Loading()),
    );
  }
}

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) => const Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator(),
              SizedBox(height: 16),
              Text('正在加载核心…'),
            ],
          ),
        ),
      );
}

class _LoadFailure extends StatelessWidget {
  const _LoadFailure({required this.error});

  final Object error;

  @override
  Widget build(BuildContext context) => Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(32),
          child: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.error_outline, size: 40),
                const SizedBox(height: 16),
                const Text('无法加载核心'),
                const SizedBox(height: 8),
                SelectableText('$error', textAlign: TextAlign.center),
                const SizedBox(height: 16),
                const Text(
                  '请先在仓库根目录执行：\n'
                  'cargo build --release -p mhy-qrscanner-bridge',
                  textAlign: TextAlign.center,
                ),
              ],
            ),
          ),
        ),
      );
}

/// One nav destination: (label, icon).
typedef _NavEntry = (String, IconData);

class _Shell extends StatefulWidget {
  const _Shell({
    required this.state,
    required this.themeMode,
    required this.onToggleTheme,
  });

  final AppState state;
  final ThemeMode themeMode;
  final void Function(bool dark) onToggleTheme;

  @override
  State<_Shell> createState() => _ShellState();
}

class _ShellState extends State<_Shell> with WindowListener {
  int _index = 0;
  bool _paneExpanded = true;
  bool _maximized = false;
  bool _alwaysOnTop = false;

  static const _mainEntries = <_NavEntry>[
    ('登录', Icons.person_rounded),
    ('扫码', Icons.qr_code_scanner_rounded),
  ];
  static const _trailingEntries = <_NavEntry>[
    ('设置', Icons.settings_rounded),
    ('关于', Icons.info_rounded),
    ('开发者', Icons.code_rounded),
  ];

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    _syncMaximized();
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  Future<void> _syncMaximized() async {
    try {
      final m = await windowManager.isMaximized();
      if (m != _maximized && mounted) setState(() => _maximized = m);
    } catch (_) {}
  }

  @override
  void onWindowMaximize() => setState(() => _maximized = true);

  @override
  void onWindowUnmaximize() => setState(() => _maximized = false);

  /// 登录 → 扫码 为主流程；设置/关于/开发者 钉在面板底部（BetterGI 风格）。
  List<_NavEntry> _visibleTrailing(bool devMode) => [
        ..._trailingEntries.take(2),
        if (devMode) _trailingEntries[2],
      ];

  int get _pageCount =>
      _mainEntries.length + _visibleTrailing(widget.state.devMode).length;

  Widget _pageFor(int index, AppState state) {
    final trailingStart = _mainEntries.length;
    if (index < trailingStart) {
      return [LoginScreen(state: state), QrScreen(state: state)][index];
    }
    final labels =
        _visibleTrailing(state.devMode).map((e) => e.$1).toList();
    return switch (labels[index - trailingStart]) {
      '设置' => SettingsScreen(
          state: state,
          dark: widget.themeMode == ThemeMode.dark,
          onToggleTheme: widget.onToggleTheme,
        ),
      '关于' => AboutScreen(state: state),
      _ => DevScreen(state: state),
    };
  }

  @override
  Widget build(BuildContext context) {
    final state = widget.state;
    final dark = widget.themeMode == ThemeMode.dark;

    return AnimatedBuilder(
      animation: state,
      builder: (context, _) {
        final trailing = _visibleTrailing(state.devMode);
        if (_index >= _pageCount) _index = 0;
        final pages = [
          for (var i = 0; i < _pageCount; i++) _pageFor(i, state),
        ];
        return Scaffold(
          backgroundColor: Colors.transparent,
          body: Column(
            children: [
              _TitleBar(
                dark: dark,
                maximized: _maximized,
                alwaysOnTop: _alwaysOnTop,
                onToggleTheme: widget.onToggleTheme,
                onTogglePin: () async {
                  final next = !_alwaysOnTop;
                  await windowManager.setAlwaysOnTop(next);
                  if (mounted) setState(() => _alwaysOnTop = next);
                },
              ),
              Expanded(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _NavPane(
                      dark: dark,
                      expanded: _paneExpanded,
                      mainEntries: _mainEntries,
                      trailingEntries: trailing,
                      selectedIndex: _index,
                      onSelect: (i) => setState(() => _index = i),
                      onToggleExpand: () =>
                          setState(() => _paneExpanded = !_paneExpanded),
                    ),
                    Expanded(
                      child: Container(
                        // 内容区比面板亮一档，构成 BetterGI 式的面板/内容分层
                        color: dark
                            ? Colors.white.withValues(alpha: 0.03)
                            : Colors.black.withValues(alpha: 0.02),
                        child: ClipRect(
                          child: IndexedStack(index: _index, children: pages),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              _StatusBar(state: state),
            ],
          ),
        );
      },
    );
  }
}

/// 自绘标题栏：与面板同色，左侧小图标+短标题，右侧主题切换与窗口按钮。
class _TitleBar extends StatelessWidget {
  const _TitleBar({
    required this.dark,
    required this.maximized,
    required this.alwaysOnTop,
    required this.onToggleTheme,
    required this.onTogglePin,
  });

  final bool dark;
  final bool maximized;
  final bool alwaysOnTop;
  final void Function(bool dark) onToggleTheme;
  final Future<void> Function() onTogglePin;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      height: 36,
      color: Colors.transparent,
      child: DragToMoveArea(
        child: Row(
          children: [
            const SizedBox(width: 12),
            Icon(Icons.qr_code_rounded, size: 16, color: scheme.primary),
            const SizedBox(width: 8),
            Text(
              '米游抢码器',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: scheme.onSurface.withValues(alpha: 0.75),
                  ),
            ),
            const Spacer(),
            _TitleButton(
              icon: alwaysOnTop
                  ? Icons.push_pin
                  : Icons.push_pin_outlined,
              tooltip: alwaysOnTop ? '取消窗口置顶' : '窗口置顶',
              onTap: onTogglePin,
            ),
            const SizedBox(width: 6),
            _TitleButton(
              icon: dark ? Icons.light_mode_outlined : Icons.dark_mode_outlined,
              tooltip: dark ? '切换浅色' : '切换深色',
              onTap: () => onToggleTheme(!dark),
            ),
            _TitleButton(
              icon: Icons.horizontal_rule,
              tooltip: '最小化',
              onTap: windowManager.minimize,
            ),
            _TitleButton(
              icon: maximized ? Icons.filter_center_focus : Icons.crop_square,
              iconSize: 14,
              tooltip: maximized ? '还原' : '最大化',
              onTap: () async {
                if (maximized) {
                  await windowManager.unmaximize();
                } else {
                  await windowManager.maximize();
                }
              },
            ),
            const _TitleCloseButton(),
          ],
        ),
      ),
    );
  }
}

class _TitleButton extends StatelessWidget {
  const _TitleButton({
    required this.icon,
    required this.tooltip,
    required this.onTap,
    this.iconSize = 16,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;
  final double iconSize;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          width: 44,
          height: 36,
          child: Icon(
            icon,
            size: iconSize,
            color: Theme.of(context).iconTheme.color,
          ),
        ),
      ),
    );
  }
}

class _TitleCloseButton extends StatelessWidget {
  const _TitleCloseButton();

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: '关闭',
      child: InkWell(
        onTap: windowManager.close,
        hoverColor: const Color(0xFFC42B1C),
        child: SizedBox(
          width: 46,
          height: 36,
          child: Icon(
            Icons.close,
            size: 16,
            color: Theme.of(context).iconTheme.color,
          ),
        ),
      ),
    );
  }
}

/// 导航面板：胶囊选中高亮 + 左缘强调竖条，汉堡按钮折叠为图标模式。
class _NavPane extends StatelessWidget {
  const _NavPane({
    required this.dark,
    required this.expanded,
    required this.mainEntries,
    required this.trailingEntries,
    required this.selectedIndex,
    required this.onSelect,
    required this.onToggleExpand,
  });

  final bool dark;
  final bool expanded;
  final List<_NavEntry> mainEntries;
  final List<_NavEntry> trailingEntries;
  final int selectedIndex;
  final void Function(int) onSelect;
  final VoidCallback onToggleExpand;

  Widget _tile(BuildContext context, _NavEntry entry, int index) {
    final (label, icon) = entry;
    final scheme = Theme.of(context).colorScheme;
    final selected = index == selectedIndex;
    final iconColor = scheme.onSurface.withValues(alpha: selected ? 1 : 0.72);
    final content = expanded
        ? Row(
            children: [
              // 左缘强调竖条（WinUI 展开态选中指示器）
              SizedBox(
                width: 3,
                height: 22,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: selected ? scheme.primary : Colors.transparent,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  child: Row(
                    children: [
                      Icon(icon, size: 19, color: iconColor),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          label,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                color: scheme.onSurface
                                    .withValues(alpha: selected ? 1 : 0.78),
                              ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          )
        // 折叠模式：图标外接高亮圆（Fluent 紧凑选中样式）。整块左对齐且定宽，
        // 面板宽度动画期间图标钉在原地，不会随面板一起居中漂移。
        : Align(
            alignment: Alignment.centerLeft,
            child: SizedBox(
              width: 44,
              child: Center(
                child: CircleAvatar(
                  radius: 17,
                  backgroundColor: selected
                      ? scheme.onSurface.withValues(alpha: dark ? 0.10 : 0.08)
                      : Colors.transparent,
                  child: Icon(icon, size: 19, color: iconColor),
                ),
              ),
            ),
          );
    final tileColor = expanded && selected
        ? scheme.onSurface.withValues(alpha: dark ? 0.08 : 0.06)
        : Colors.transparent;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      child: Material(
        color: tileColor,
        borderRadius: BorderRadius.circular(expanded ? 6 : 19),
        child: InkWell(
          borderRadius: BorderRadius.circular(expanded ? 6 : 19),
          hoverColor: scheme.onSurface.withValues(alpha: 0.05),
          onTap: () => onSelect(index),
          child: SizedBox(height: 38, child: content),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final width = expanded ? 176.0 : 56.0;
    final paneChildren = <Widget>[
      Padding(
        padding: const EdgeInsets.fromLTRB(6, 2, 6, 6),
        child: Material(
          color: Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            borderRadius: BorderRadius.circular(6),
            hoverColor: scheme.onSurface.withValues(alpha: 0.05),
            onTap: onToggleExpand,
            child: SizedBox(
              height: 36,
              // 汉堡固定贴左，且与导航图标的左缘对齐（展开/折叠都一致，
              // 图标列左缘 = 6 面板边距 + 3 指示条 + 10 内边距 = 19px）
              child: Align(
                alignment: Alignment.centerLeft,
                child: Padding(
                  padding: const EdgeInsets.only(left: 13),
                  child: Icon(
                    Icons.menu,
                    size: 18,
                    color: scheme.onSurface.withValues(alpha: 0.72),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
      for (var i = 0; i < mainEntries.length; i++)
        _tile(context, mainEntries[i], i),
      const Spacer(),
      for (var i = 0; i < trailingEntries.length; i++)
        _tile(context, trailingEntries[i], mainEntries.length + i),
      const SizedBox(height: 6),
    ];

    return AnimatedContainer(
      duration: const Duration(milliseconds: 180),
      curve: Curves.easeOutCubic,
      width: width,
      decoration: BoxDecoration(
        border: Border(
          right: BorderSide(
            color: scheme.outlineVariant.withValues(alpha: 0.35),
            width: 1,
          ),
        ),
      ),
      child: Column(children: paneChildren),
    );
  }
}

class _StatusBar extends StatelessWidget {
  const _StatusBar({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final account = state.account.isEmpty ? '未登录' : '账号：${state.account}';
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        color: Theme.of(context)
            .colorScheme
            .surfaceContainerHighest
            .withValues(alpha: 0.6),
      ),
      child: Row(
        children: [
          Text(account, style: Theme.of(context).textTheme.bodySmall),
          const SizedBox(width: 16),
          Expanded(
            child: Text(
              state.status ?? '就绪',
              style: Theme.of(context).textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
