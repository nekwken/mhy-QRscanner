import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mhy_qrscanner/main.dart';
import 'package:mhy_qrscanner/src/rust/api/auth.dart';
import 'package:mhy_qrscanner/src/rust/api/core.dart';
import 'package:mhy_qrscanner/src/rust/api/device.dart';
import 'package:mhy_qrscanner/src/rust/api/qr.dart';
import 'package:mhy_qrscanner/src/rust/error.dart';
import 'package:mhy_qrscanner/src/rust_loader.dart';

/// End-to-end tests across the FFI boundary: Dart → Rust → DPAPI storage.
///
/// Requires the release cdylib. Run with:
///
/// ```powershell
/// cargo build --release -p mhy-qrscanner-bridge
/// $env:MHYQR_BRIDGE_DLL = 'E:\抢码工具\repo\target\release\mhy_qrscanner_bridge.dll'
/// flutter test
/// ```
///
/// When the library or the environment variable is missing the whole group is
/// skipped, so a plain `flutter test` on a machine without the DLL still passes.
///
/// Nothing here touches the network: the QR cases assert the *preconditions*
/// Rust checks before the first request goes out.

/// A V1 game QR URL. Shape only — these tests never reach the network.
const _gameUrl =
    'https://user.mihoyo.com/qr_code_in_game.html?app_id=4&ticket=tk1&biz_key=hk4e_cn';

/// A V2 passport QR URL, the form the panda scan step returns.
const _passportUrl =
    'https://user.mihoyo.com/login-platform/mobile.html?expire=1&tk=tk-9&token_types=1#/login/qr';

/// Environment override for the CLI used by the storage-interop group.
const String kCliExeEnv = 'MHYQR_CLI_EXE';

void main() {
  final library = resolveBridgeLibrary();
  final skipReason = library == null
      ? 'mhy_qrscanner_bridge.dll not found; build it and set $kBridgeDllEnv'
      : null;
  final cli = resolveCliBinary();
  final interopSkip = skipReason ??
      (cli == null ? 'mhyqr.exe not found; build it and set $kCliExeEnv' : null);

  group('FFI', () {
    late Core core;
    late Directory dataDir;

    setUpAll(initRust);

    setUp(() async {
      dataDir = await Directory.systemTemp.createTemp('mhyqr-ffi-');
      core = await coreNewAt(dataDir: dataDir.path);
    });

    tearDown(() async {
      if (dataDir.existsSync()) {
        await dataDir.delete(recursive: true);
      }
    });

    test('info describes the core it was built with', () async {
      final info = await core.info();
      expect(info.coreVersion, isNotEmpty);
      expect(info.games, contains('hk4e_cn'));
      expect(info.deviceTemplates, contains('xiaomi14'));
      expect(info.passportHost, contains('mihoyo.com'));
      expect(info.dataDir, contains('mhyqr-ffi-'));
    });

    test('device create → show round trip through DPAPI storage', () async {
      final created = await deviceCreate(
        core: core,
        account: 'ffi-acct',
        template: 'xiaomi14',
        force: false,
      );
      expect(created.account, 'ffi-acct');
      expect(created.model, 'Xiaomi 14');
      expect(created.gameBiz, 'hk4e_cn');
      expect(created.registered, isFalse);
      // identity is masked, never raw
      expect(created.deviceIdMasked, contains('...'));
      expect(created.deviceIdLen, BigInt.from(16));

      // A second handle over the same directory proves the encrypted file is
      // readable, which is the same path the CLI writes.
      final second = await coreNewAt(dataDir: dataDir.path);
      final shown = await deviceShow(core: second, account: 'ffi-acct');
      expect(shown.deviceIdMasked, created.deviceIdMasked);
      expect(shown.deviceFpMasked, created.deviceFpMasked);
    });

    test('create refuses to overwrite without force, then replaces', () async {
      await deviceCreate(
        core: core,
        account: 'ffi-acct',
        template: 'samsung-s24',
        force: false,
      );

      await expectLater(
        deviceCreate(
          core: core,
          account: 'ffi-acct',
          template: 'samsung-s24',
          force: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );

      final replaced = await deviceCreate(
        core: core,
        account: 'ffi-acct',
        template: 'samsung-s24',
        force: true,
      );
      expect(replaced.model, 'SM-S9280');
    });

    test('account labels are validated in Rust, not in Dart', () async {
      await expectLater(
        deviceCreate(
          core: core,
          account: '   ',
          template: 'xiaomi14',
          force: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );
      await expectLater(
        deviceCreate(
          core: core,
          account: 'a/b',
          template: 'xiaomi14',
          force: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );
    });

    test('unknown template is rejected without writing a profile', () async {
      await expectLater(
        deviceCreate(
          core: core,
          account: 'ffi-acct',
          template: 'nokia-3310',
          force: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );
      await expectLater(
        deviceShow(core: core, account: 'ffi-acct'),
        throwsA(isA<BridgeError_AccountNotFound>()),
      );
    });

    test('game_biz can be switched without rotating the identity', () async {
      final created = await deviceCreate(
        core: core,
        account: 'ffi-acct',
        template: 'xiaomi14',
        force: false,
      );
      final switched = await deviceSetGameBiz(
        core: core,
        account: 'ffi-acct',
        gameBiz: 'hkrpg_cn',
      );
      expect(switched.gameBiz, 'hkrpg_cn');
      expect(switched.deviceIdMasked, created.deviceIdMasked);

      await expectLater(
        deviceSetGameBiz(core: core, account: 'ffi-acct', gameBiz: 'nope'),
        throwsA(isA<BridgeError_Invalid>()),
      );
    });

    test('session is absent for a fresh device', () async {
      await deviceCreate(
        core: core,
        account: 'ffi-acct',
        template: 'xiaomi14',
        force: false,
      );
      await expectLater(
        authShow(core: core, account: 'ffi-acct'),
        throwsA(isA<BridgeError_AccountNotFound>()),
      );
    });

    test('qr login validates the account and the URL before any request',
        () async {
      // Non-game URL: rejected while resolving the source, before the device
      // lookup.
      await expectLater(
        qrLoginGame(
          core: core,
          account: 'ffi-acct',
          source: const QrSource.url('https://user.mihoyo.com/index.html'),
          confirm: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );

      // Missing screenshot.
      await expectLater(
        qrLoginGame(
          core: core,
          account: 'ffi-acct',
          source: const QrSource.image('definitely-not-here.png'),
          confirm: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );

      // Blank account label.
      await expectLater(
        qrLoginGame(
          core: core,
          account: '   ',
          source: const QrSource.url(_gameUrl),
          confirm: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );
    });

    test('qr login needs a stored device, not just a valid URL', () async {
      // Valid game URL but no profile for this account: the device lookup is
      // the first thing that fails, so no HTTP client is ever built.
      await expectLater(
        qrLoginGame(
          core: core,
          account: 'ghost',
          source: const QrSource.url(_gameUrl),
          confirm: false,
        ),
        throwsA(isA<BridgeError_AccountNotFound>()),
      );
    });

    test('passport QR scan and cancel need device and session', () async {
      await expectLater(
        qrScan(
          core: core,
          account: 'ghost',
          passportQrUrl: 'not-a-passport-url',
          confirm: false,
        ),
        throwsA(isA<BridgeError_Invalid>()),
      );

      await expectLater(
        qrScan(
          core: core,
          account: 'ghost',
          passportQrUrl: _passportUrl,
          confirm: false,
        ),
        throwsA(isA<BridgeError_AccountNotFound>()),
      );

      await expectLater(
        qrCancel(
          core: core,
          account: 'ghost',
          urlOrTicket: 'tk-9',
          tokenTypes: const ['1'],
        ),
        throwsA(isA<BridgeError_AccountNotFound>()),
      );
    });

    testWidgets('the shell navigates to the settings screen', (tester) async {
      // the settings page grew (appearance + developer cards); a tall surface
      // keeps the audit section inside the lazy ListView's build window
      await tester.binding.setSurfaceSize(const Size(900, 1600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(const QmdApp());
      await settleReal(tester, find.byType(NavigationRail));
      // the shell now opens on the login screen; there is no device tab
      // until developer options are switched on in settings
      expect(find.text('登录'), findsWidgets);

      await tester.tap(find.text('设置'));
      await settleReal(tester, find.text('开发者选项'));

      expect(find.text('设置'), findsWidgets);
      expect(find.text('开发者选项'), findsOneWidget);
      expect(find.text('开发者选项'), findsOneWidget);
      expect(find.textContaining('默认每次批准都需要你在此之前确认'), findsOneWidget);
      // the audit source is named, even when the log is still empty
      expect(find.textContaining('audit.log'), findsOneWidget);
    });
  }, skip: skipReason);

  group('FFI storage interop', () {
    late Directory dataDir;

    setUpAll(initRust);

    setUp(() {
      dataDir = Directory.systemTemp.createTempSync('mhyqr-interop-');
    });

    tearDown(() {
      if (dataDir.existsSync()) {
        dataDir.deleteSync(recursive: true);
      }
    });

    /// The two sides must agree on the store layout: `<dir>/accounts/<label>/`.
    File profileFile(String account) => File(
          '${dataDir.path}${Platform.pathSeparator}accounts'
          '${Platform.pathSeparator}$account'
          '${Platform.pathSeparator}device-profile.bin',
        );

    test('a profile written by the CLI is readable from Dart', () async {
      final result = await Process.run(cli!, [
        '--data-dir', dataDir.path,
        'device', 'create',
        '--account', 'interop',
        '--template', 'xiaomi14',
      ]);
      expect(
        result.exitCode,
        0,
        reason: 'mhyqr device create failed:\n${result.stdout}\n${result.stderr}',
      );

      final cliId = _field(result.stdout as String, 'device_id');
      final cliFp = _field(result.stdout as String, 'device_fp');

      // The on-disk file is DPAPI ciphertext, not readable JSON.
      final profile = profileFile('interop');
      expect(profile.existsSync(), isTrue);
      final raw = await profile.readAsBytes();
      expect(raw, isNotEmpty);
      final printable =
          String.fromCharCodes(raw.where((b) => b >= 32 && b < 127));
      expect(printable, isNot(contains('device_id')));
      expect(printable, isNot(contains('Xiaomi')));

      final core = await coreNewAt(dataDir: dataDir.path);
      final shown = await deviceShow(core: core, account: 'interop');
      expect(shown.deviceIdMasked, cliId);
      expect(shown.deviceFpMasked, cliFp);
      expect(shown.model, 'Xiaomi 14');
      expect(shown.deviceIdLen, BigInt.from(16));
      expect(shown.registered, isFalse);
    });

    test('a profile written by the shell is readable by the CLI', () async {
      final core = await coreNewAt(dataDir: dataDir.path);
      final created = await deviceCreate(
        core: core,
        account: 'interop',
        template: 'vivo-x100',
        force: false,
      );
      expect(profileFile('interop').existsSync(), isTrue);

      final result = await Process.run(cli!, [
        '--data-dir', dataDir.path,
        'device', 'show',
        '--account', 'interop',
      ]);
      expect(
        result.exitCode,
        0,
        reason: 'mhyqr device show failed:\n${result.stdout}\n${result.stderr}',
      );

      final out = result.stdout as String;
      expect(_field(out, 'device_id'), created.deviceIdMasked);
      expect(_field(out, 'game_biz'), created.gameBiz);
      expect(out, contains('model=${created.model}'));
    });
  }, skip: interopSkip);
}

/// Wait for a widget that only appears once a real (non-fake-clock) future has
/// completed.
///
/// The bridge calls run on Rust threads, so `pumpAndSettle` cannot see them —
/// it would also never settle while a `CircularProgressIndicator` is on screen.
Future<void> settleReal(
  WidgetTester tester,
  Finder target, {
  Duration timeout = const Duration(seconds: 10),
}) async {
  await tester.runAsync(() async {
    final deadline = DateTime.now().add(timeout);
    while (!tester.any(target) && DateTime.now().isBefore(deadline)) {
      await Future<void>.delayed(const Duration(milliseconds: 50));
      await tester.pump();
    }
  });
  await tester.pump();
}

/// Read `name=value` out of the CLI's line-oriented stdout.
String _field(String stdout, String name) {
  final prefix = '$name=';
  for (final line in stdout.split(RegExp(r'\r?\n'))) {
    if (line.startsWith(prefix)) return line.substring(prefix.length);
  }
  throw StateError('`$prefix` not found in:\n$stdout');
}

/// Locate the release `mhyqr.exe`.
///
/// It normally sits beside the cdylib, so the resolved bridge path is the
/// strongest candidate; the workspace `target/release` walk covers a plain
/// `flutter test` from `app/`.
String? resolveCliBinary() {
  final override = Platform.environment[kCliExeEnv];
  if (override != null && override.isNotEmpty && File(override).existsSync()) {
    return override;
  }
  final sep = Platform.pathSeparator;
  final candidates = <File>[];
  final dll = resolveBridgeLibrary();
  if (dll != null) {
    candidates.add(File('${File(dll).parent.path}${sep}mhyqr.exe'));
  }
  final cwd = Directory.current;
  for (final root in [cwd, cwd.parent, cwd.parent.parent]) {
    candidates.add(File('${root.path}${sep}target${sep}release${sep}mhyqr.exe'));
  }
  for (final exe in candidates) {
    if (exe.existsSync()) return exe.path;
  }
  return null;
}
