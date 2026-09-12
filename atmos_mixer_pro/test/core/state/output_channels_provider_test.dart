import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/error.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// [OutputChannelsNotifier] 상태 전이 테스트.
///
/// `hardwareChannelsProvider`(FutureProvider)는 실패 시 조용히 `[]`을
/// 반환해서 "채널이 0개"와 "조회 실패"를 구분할 수 없었다. 이 테스트는
/// loading/ready/noDevice/error 네 상태가 올바르게 갈라지고,
/// lastKnownGoodChannelNames가 순간적인 재조회 실패/로딩 중에도 유지되는지
/// 검증한다.
void main() {
  tearDown(() {
    // GlobalDeviceCache는 static이라 테스트 간에 공유된다.
    GlobalDeviceCache.channels.clear();
  });

  AppConfig config({String? deviceName}) => AppConfig(
    oscPort: 9000,
    deviceName: deviceName,
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
    oscWhitelist: const [],
    globalReverbMix: 0.0,
    globalReverbDecay: 1.0,
  );

  /// [outputChannelsProvider]의 상태가 `status`에 도달할 때까지 기다린다.
  /// 이미 그 상태라면 즉시 반환한다.
  Future<OutputChannelsState> waitForStatus(
    ProviderContainer container,
    OutputChannelsStatus status,
  ) async {
    final current = container.read(outputChannelsProvider);
    if (current.status == status) return current;

    final completer = Completer<OutputChannelsState>();
    final sub = container.listen(outputChannelsProvider, (previous, next) {
      if (next.status == status && !completer.isCompleted) {
        completer.complete(next);
      }
    });
    try {
      return await completer.future.timeout(const Duration(seconds: 2));
    } finally {
      sub.close();
    }
  }

  ProviderContainer buildContainer({
    required String? deviceName,
    required Future<List<String>> Function(String?) fetch,
  }) {
    final container = ProviderContainer(
      overrides: [
        configProvider.overrideWith(() => _MockConfigNotifier(config(deviceName: deviceName))),
        deviceEventStreamProvider.overrideWith((ref) => const Stream.empty()),
        outputChannelsProvider.overrideWith(() => _TestOutputChannelsNotifier(fetch)),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  test('조회 성공: ready 상태가 되고 lastKnownGoodChannelNames가 채워진다', () async {
    final container = buildContainer(
      deviceName: 'DeviceA',
      fetch: (_) async => ['Ch 1', 'Ch 2'],
    );

    final state = await waitForStatus(container, OutputChannelsStatus.ready);

    expect(state.channelNames, ['Ch 1', 'Ch 2']);
    expect(state.lastKnownGoodChannelNames, ['Ch 1', 'Ch 2']);
    expect(state.deviceName, 'DeviceA');
    expect(state.errorMessage, isNull);
  });

  test('조회 결과가 빈 목록이면 noDevice 상태가 되고 channelNames가 비어있다', () async {
    final container = buildContainer(
      deviceName: 'DeviceA',
      fetch: (_) async => const [],
    );

    final state = await waitForStatus(container, OutputChannelsStatus.noDevice);

    expect(state.channelNames, isEmpty);
  });

  test('"No default output device" 예외는 noDevice로 매핑된다(무음 실패 금지)', () async {
    final container = buildContainer(
      deviceName: null,
      fetch: (_) async => throw const AtmosError(message: 'No default output device'),
    );

    final state = await waitForStatus(container, OutputChannelsStatus.noDevice);

    expect(state.channelNames, isEmpty);
    expect(state.errorMessage, isNull);
  });

  test('"Device not found" 예외(장치 연결 해제)는 noDevice로 매핑된다', () async {
    final container = buildContainer(
      deviceName: 'UnpluggedDevice',
      fetch: (_) async => throw const AtmosError(message: 'Device not found: UnpluggedDevice'),
    );

    final state = await waitForStatus(container, OutputChannelsStatus.noDevice);

    expect(state.channelNames, isEmpty);
  });

  test('그 외 예외는 error 상태가 되고 lastKnownGoodChannelNames를 유지한다', () async {
    var callCount = 0;
    final container = buildContainer(
      deviceName: 'DeviceA',
      fetch: (_) async {
        callCount++;
        if (callCount == 1) return ['Ch 1', 'Ch 2'];
        throw Exception('driver crashed');
      },
    );

    // 1차 조회 성공 확인.
    final ready = await waitForStatus(container, OutputChannelsStatus.ready);
    expect(ready.channelNames, ['Ch 1', 'Ch 2']);

    // 장치 전환으로 재조회를 트리거하고, 이번엔 예외가 나게 한다.
    container.read(configProvider.notifier).saveConfig(config(deviceName: 'DeviceB'));

    final errored = await waitForStatus(container, OutputChannelsStatus.error);

    expect(errored.errorMessage, contains('driver crashed'));
    // 무음 실패 금지: 실패해도 마지막으로 성공한 채널 목록을 UI가 계속 볼 수 있어야 한다.
    expect(errored.channelNames, ['Ch 1', 'Ch 2']);
    expect(errored.lastKnownGoodChannelNames, ['Ch 1', 'Ch 2']);
  });

  test('로딩 중에는 이전 lastKnownGoodChannelNames를 그대로 보여준다', () async {
    final blocker = Completer<List<String>>();
    var callCount = 0;
    final container = buildContainer(
      deviceName: 'DeviceA',
      fetch: (_) async {
        callCount++;
        if (callCount == 1) return ['Ch 1', 'Ch 2'];
        return blocker.future; // 두 번째 조회는 완료시키기 전까지 멈춰있다.
      },
    );

    await waitForStatus(container, OutputChannelsStatus.ready);

    container.read(configProvider.notifier).saveConfig(config(deviceName: 'DeviceB'));

    final loading = await waitForStatus(container, OutputChannelsStatus.loading);
    expect(loading.channelNames, ['Ch 1', 'Ch 2']);
    expect(loading.lastKnownGoodChannelNames, ['Ch 1', 'Ch 2']);

    blocker.complete(['Ch 3', 'Ch 4']);
    final ready = await waitForStatus(container, OutputChannelsStatus.ready);
    expect(ready.channelNames, ['Ch 3', 'Ch 4']);
  });

  test('GlobalDeviceCache에 캐시된 장치는 네트워크 조회 없이 즉시 ready가 된다', () async {
    GlobalDeviceCache.channels['DeviceA'] = ['Cached 1', 'Cached 2'];

    final container = buildContainer(
      deviceName: 'DeviceA',
      fetch: (_) => throw StateError('캐시가 있으면 fetch가 호출되면 안 된다'),
    );

    final state = await waitForStatus(container, OutputChannelsStatus.ready);
    expect(state.channelNames, ['Cached 1', 'Cached 2']);
  });
}

class _MockConfigNotifier extends ConfigNotifier {
  _MockConfigNotifier(this._initial);
  final AppConfig _initial;

  @override
  AppConfig? build() => _initial;

  @override
  void saveConfig(
    AppConfig newConfig, {
    bool forceRestart = false,
    bool skipPreload = false,
  }) {
    state = newConfig;
  }
}

class _TestOutputChannelsNotifier extends OutputChannelsNotifier {
  _TestOutputChannelsNotifier(this._fetch);
  final Future<List<String>> Function(String? deviceName) _fetch;

  @override
  Future<List<String>> fetchChannelNames(String? deviceName) =>
      _fetch(deviceName);
}
