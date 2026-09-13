import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'rust/frb_generated.dart';

/// Environment override for the cdylib path.
///
/// Useful when the executable sits somewhere the search order cannot reach,
/// e.g. `flutter test` (where the "executable" is the Dart VM).
const String kBridgeDllEnv = 'MHYQR_BRIDGE_DLL';

/// Resolve the cdylib, or null when it cannot be found.
///
/// Search order:
///  1. `$MHYQR_BRIDGE_DLL` (explicit override)
///  2. `mhy_qrscanner_bridge.dll` next to the executable (packaged layout)
///  3. the workspace `target/release` / `target/debug` (development layout)
String? resolveBridgeLibrary() {
  final override = Platform.environment[kBridgeDllEnv];
  if (override != null && override.isNotEmpty && File(override).existsSync()) {
    return override;
  }
  for (final candidate in [
    _nextToExecutable('mhy_qrscanner_bridge.dll'),
    _repoPath('target/release/mhy_qrscanner_bridge.dll'),
    _repoPath('target/debug/mhy_qrscanner_bridge.dll'),
  ]) {
    if (candidate != null && File(candidate).existsSync()) return candidate;
  }
  return null;
}

bool _initialized = false;

/// Loads the Rust cdylib and initialises the bridge.
///
/// Idempotent: the bridge refuses a second `init`, so repeated calls (a widget
/// test and an integration test in the same process) are safe.
///
/// The DLL is built with `cargo build --release -p mhy-qrscanner-bridge`. Build it in
/// release: QR decoding is roughly 20x slower in a debug build.
Future<void> initRust() async {
  if (_initialized) return;
  final path = resolveBridgeLibrary();
  if (path != null) {
    await RustLib.init(externalLibrary: ExternalLibrary.open(path));
  } else {
    // Fall back to the generated loader so the error names what it looked for.
    await RustLib.init();
  }
  _initialized = true;
}

String? _nextToExecutable(String name) {
  final exe = File(Platform.resolvedExecutable);
  final candidate = File('${exe.parent.path}${Platform.pathSeparator}$name');
  return candidate.existsSync() ? candidate.path : null;
}

/// Resolve a path relative to the workspace root.
///
/// During `flutter run` the CWD is `app/`, so the root is one level up.
String? _repoPath(String relative) {
  final cwd = Directory.current;
  for (final root in [cwd, cwd.parent, cwd.parent.parent]) {
    final candidate = File('${root.path}${Platform.pathSeparator}$relative');
    if (candidate.existsSync()) return candidate.path;
  }
  return null;
}
