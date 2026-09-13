import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mhy_qrscanner/src/app_state.dart';
import 'package:mhy_qrscanner/src/screens/about_screen.dart';
import 'package:mhy_qrscanner/src/screens/login_screen.dart';
import 'package:mhy_qrscanner/src/screens/qr_screen.dart';
import 'package:mhy_qrscanner/src/screens/settings_screen.dart';

void main() {
  group('AppState', () {
    test('starts logged-out and an empty selection clears it', () {
      final state = AppState();
      expect(state.account, '');
      state.selectAccount('  acct-1  ');
      expect(state.account, 'acct-1');
      state.selectAccount('   ');
      expect(state.account, '');
    });

    test('accepts a non-empty label and trims it', () {
      final state = AppState();
      state.selectAccount('  acct-1  ');
      expect(state.account, 'acct-1');
    });

    test('status is observable', () {
      final state = AppState();
      var notified = 0;
      state.addListener(() => notified++);
      state.setStatus('hello');
      expect(state.status, 'hello');
      expect(notified, 1);
    });

    test('core throws before init', () {
      final state = AppState();
      expect(state.ready, isFalse);
      expect(() => state.core, throwsStateError);
    });
  });

  testWidgets('AboutScreen 只保留版本与数据目录', (tester) async {
    final state = AppState();
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: AboutScreen(state: state))));

    expect(find.text('关于'), findsOneWidget);
    expect(find.textContaining('核心尚未初始化'), findsOneWidget);
  });

  testWidgets('SettingsScreen states the safe default without a core', (tester) async {
    final state = AppState();
    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: SettingsScreen(state: state))),
    );
    await tester.pump();

    expect(find.text('设置'), findsOneWidget);
    // 指引开关默认关闭
    expect(find.text('指引'), findsOneWidget);
    final guide = tester.widget<SwitchListTile>(
      find.ancestor(
        of: find.text('指引'),
        matching: find.byType(SwitchListTile),
      ),
    );
    expect(guide.value, isFalse);

    // 拨动开关必须同步到 AppState，否则各页面读到的还是旧值（真实出现过的缺陷）
    await tester.tap(find.ancestor(
      of: find.text('指引'),
      matching: find.byType(SwitchListTile),
    ));
    await tester.pumpAndSettle();
    expect(state.guide, isTrue, reason: '指引开关应把状态同步给 AppState');
    // 弹窗开关的默认语义写在副标题里，且无需核心即可渲染
    expect(find.text('仅扫描成功后弹出批准窗口'), findsOneWidget);
    expect(find.text('批准成功后弹出通知'), findsOneWidget);
    final popupScan = tester.widget<SwitchListTile>(
      find.ancestor(
        of: find.text('仅扫描成功后弹出批准窗口'),
        matching: find.byType(SwitchListTile),
      ),
    );
    expect(popupScan.value, isTrue); // 弹窗默认开启（未设置视为开）
  });

  /// SMS login takes a phone number, and only a phone number. These paths never
  /// touch the core, so they run without the cdylib.
  group('LoginScreen SMS input', () {
    /// The form is taller than the default 600px test viewport, and a tap
    /// outside the viewport lands nowhere without failing.
    setUp(() {
      final view = TestWidgetsFlutterBinding.ensureInitialized().platformDispatcher.views.first;
      view.physicalSize = const Size(1200, 2000);
      view.devicePixelRatio = 1.0;
      addTearDown(view.reset);
    });
    Future<void> pumpSmsMode(WidgetTester tester) async {
      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: LoginScreen(state: AppState()))),
      );
      await tester.tap(find.text('短信登录'));
      await tester.pumpAndSettle();
    }

    Future<void> tapRequest(WidgetTester tester) async {
      await tester.tap(find.text('获取验证码'));
      await tester.pumpAndSettle();
    }

    testWidgets('rejects an email address', (tester) async {
      await pumpSmsMode(tester);
      await tester.enterText(find.widgetWithText(TextField, '手机号'), 'me@example.com');
      await tapRequest(tester);
      expect(find.textContaining('不能填邮箱'), findsOneWidget);
    });

    testWidgets('rejects a phone number carrying the country code', (tester) async {
      await pumpSmsMode(tester);
      await tester.enterText(find.widgetWithText(TextField, '手机号'), '+8613800000000');
      await tapRequest(tester);
      expect(find.textContaining('不要把国家/地区代码填在这里'), findsOneWidget);
    });

    testWidgets('requires a phone number at all', (tester) async {
      await pumpSmsMode(tester);
      await tapRequest(tester);
      expect(find.textContaining('请填写手机号'), findsOneWidget);
    });

    testWidgets('accepts a plain number and asks for a code', (tester) async {
      await pumpSmsMode(tester);
      await tester.enterText(find.widgetWithText(TextField, '手机号'), '138 0000 0000');
      await tapRequest(tester);
      // past validation, so the failure is now the missing core, not the input
      expect(find.textContaining('请填写手机号'), findsNothing);
      expect(find.textContaining('不能填邮箱'), findsNothing);
    });
  });

  testWidgets('QrScreen 区分屏幕与直播来源', (tester) async {
    final state = AppState();
    // 与 shell 一致：AppState 变化时重建页面（指引开关靠它生效）
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AnimatedBuilder(
            animation: state,
            builder: (context, _) => QrScreen(state: state),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('监控源'), findsOneWidget);
    // 「本机屏幕」是竞速里的一个来源，与直播间并列
    expect(find.text('本机屏幕'), findsOneWidget);
    expect(find.text('屏幕'), findsWidgets);
    // 指引默认关闭：说明性段落不出现
    expect(find.textContaining('同场竞速'), findsNothing);
    state.setGuide(true);
    await tester.pump();
    expect(find.textContaining('同场竞速'), findsWidgets);
    // 直播页签只登记直播间：切过去应看到「添加直播间」
    await tester.tap(find.text('B站直播'));
    await tester.pumpAndSettle();
    expect(find.text('添加直播间'), findsOneWidget);
    expect(find.text('直播间号'), findsOneWidget);
  });
}
