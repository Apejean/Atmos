import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart' show ExternalLibrary;

/// 앱에 들어 있는 Rust 라이브러리를 연다.
///
/// FRB 기본 로더는 작업 디렉터리 기준 `rust/target/release/`에 라이브러리가 있으면 앱 안의 것보다
/// 그것을 먼저 연다. 그 파일은 `cargo build --release`를 직접 돌려야만 갱신되므로, 프로젝트 폴더에서
/// `flutter run`하면 방금 빌드한 Rust 대신 옛 Rust가 실린다(API가 같으면 조용히, 다르면 content hash
/// 오류로 시작 실패). 생성 파일(frb_generated.dart)의 ioDirectory를 고치면 codegen이 덮어쓰므로 여기서 정한다.
/// - macOS: 앱 번들 안 프레임워크.
/// - Windows: 실행 파일 옆의 dll. 없으면(예상 밖 배치) 기본 로더에 맡긴다.
ExternalLibrary? bundledRustLibrary() {
  if (Platform.isMacOS) {
    return ExternalLibrary.open('rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro');
  }
  if (Platform.isWindows) {
    final dll = windowsBundledRustDllPath(Platform.resolvedExecutable);
    if (File(dll).existsSync()) return ExternalLibrary.open(dll);
  }
  return null;
}

/// Windows: 실행 파일과 같은 폴더의 Rust 라이브러리 경로.
String windowsBundledRustDllPath(String resolvedExecutable) {
  final cut = resolvedExecutable.lastIndexOf(RegExp(r'[\\/]'));
  final dir = cut < 0 ? '.' : resolvedExecutable.substring(0, cut);
  return '$dir\\rust_lib_atmos_mixer_pro.dll';
}
