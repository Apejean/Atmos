// 시작 때 음원 미리 읽기가 엔진·OSC 시작을 붙잡지 않는지, 그 사이 저장이 미리 읽기 뒤로 미뤄지는지 본다.
//
// 2026-11-28 Mac: 느린 파일 하나(macOS 파일 접근 확인 60~70초)가 미리 읽기를 붙잡아 엔진·OSC·공연 이어 가기가 모두
// 늦었다. 미리 읽기는 끝날 때 엔진의 전역 설정을 읽은 설정으로 다시 쓰므로, 그 사이 저장한 새 설정을 덮지 않게 저장은
// 미리 읽기가 끝난 뒤에 한다.
import 'dart:async';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' show OutputDeviceInfo;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';
import 'package:shared_preferences/shared_preferences.dart';

class _FakePaths extends PathProviderPlatform with MockPlatformInterfaceMixin {
  @override
  Future<String?> getApplicationSupportPath() async => '/tmp/atmos_startup_order_test';
}

/// 시험에 필요한 몇 가지만 흉내 내고 나머지 엔진 호출은 아무것도 하지 않는다.
class _FakeApi implements RustLibApi {
  _FakeApi(this.config);
  final AppConfig config;
  final preload = Completer<void>();
  final calls = <String>[];

  @override
  Future<AppConfig> crateApiSimpleApiGetConfig({required String path}) async => config;
  @override
  Future<void> crateApiSimpleApiPreloadAllSounds({required AppConfig config}) {
    calls.add('preload');
    return preload.future;
  }

  @override
  Future<void> crateApiSimpleApiInitAudioSystem({String? deviceName}) async => calls.add('init');
  @override
  Future<void> crateApiSimpleApiStartOscListener({required int port}) async => calls.add('osc');
  @override
  Future<List<OutputDeviceInfo>> crateApiSimpleApiGetOutputDevices() async => const [];
  @override
  Future<void> crateApiSimpleApiSaveConfig({required String path, required AppConfig config}) async =>
      calls.add('save');

  @override
  // 흉내 내지 않은 호출(엔진 상태 다시 밀어 넣기 등)은 끝나지 않는 Future로 둔다. 기다리지 않는 호출이라 시험에 영향이 없다.
  dynamic noSuchMethod(Invocation invocation) => Completer<Never>().future;
}

AppConfig _config() => AppConfig(
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
      oscPort: 8000,
      bufferSize: 512,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: const {},
      stereoConfigs: const {},
      multiConfigs: const {},
      rooms: const [],
      roomZones: const [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      oscReplyTargets: const [],
      duckingEnabled: false,
      duckingReductionDb: 0.0,
      outputRoutingMode: OutputRoutingMode.values.first,
    );

void main() {
  test('미리 읽기가 끝나지 않아도 엔진·OSC를 시작하고, 저장은 미리 읽기 뒤로 미룬다', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    SharedPreferences.setMockInitialValues({});
    PathProviderPlatform.instance = _FakePaths();
    final api = _FakeApi(_config());
    RustLib.initMock(api: api);
    final container = ProviderContainer();
    addTearDown(container.dispose);

    await container.read(configProvider.notifier).loadConfigAsync().timeout(const Duration(seconds: 2));
    expect(api.calls, containsAllInOrder(['init', 'preload', 'osc']));
    expect(api.preload.isCompleted, isFalse);

    container.read(configProvider.notifier).saveConfig(_config(), skipPreload: true);
    await Future<void>.delayed(const Duration(milliseconds: 50));
    expect(api.calls.contains('save'), isFalse, reason: '미리 읽기가 끝나기 전에 저장했다');

    api.preload.complete();
    await Future<void>.delayed(const Duration(milliseconds: 50));
    expect(api.calls.contains('save'), isTrue, reason: '미리 읽기가 끝났는데 저장하지 않았다');
  });
}
