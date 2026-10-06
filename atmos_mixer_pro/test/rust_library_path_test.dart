import 'package:atmos_mixer_pro/core/utils/rust_library.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Windows: Rust 라이브러리는 실행 파일과 같은 폴더의 dll이다', () {
    expect(
      windowsBundledRustDllPath(r'C:\Program Files\Atmos Mixer Pro\atmos_mixer_pro.exe'),
      r'C:\Program Files\Atmos Mixer Pro\rust_lib_atmos_mixer_pro.dll',
    );
    expect(
      windowsBundledRustDllPath(r'D:\dev\atmos\build\windows\x64\runner\Debug\atmos_mixer_pro.exe'),
      r'D:\dev\atmos\build\windows\x64\runner\Debug\rust_lib_atmos_mixer_pro.dll',
      reason: 'flutter run으로 띄운 개발 빌드도 exe 옆의 dll을 연다(작업 폴더의 rust/target/release가 아니라)',
    );
  });
}
