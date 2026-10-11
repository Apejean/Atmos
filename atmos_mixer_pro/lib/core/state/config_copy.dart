import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 설정(AppConfig)을 일부만 바꿔 새로 만들 때는 이것만 쓴다. FRB가 만든 클래스에는 copyWith가 없어서, 예전에는 화면
/// 33곳이 칸을 하나하나 다시 적으며 일부를 고정값으로 넣거나(전역 리버브 0%·1.0, 방 카드의 OSC 허용 목록 빈 목록)
/// 빠뜨렸다(머리 추적 OSC 주소). 설정에 칸을 더하면 여기 한 곳만 고친다 — test/config_copy_test.dart가 모든 칸이
/// 보존되는지 본다. 비어 있을 수 있는 칸(deviceName·trackingOscAddress·globalTrajectory)은 넘기지 않으면 그대로,
/// null을 넘기면 비운다.
extension AppConfigCopy on AppConfig {
  AppConfig copyWith({
    int? oscPort,
    Object? deviceName = _keep,
    int? bufferSize,
    String? themeStartOscAddress,
    String? systemResetOscAddress,
    Object? trackingOscAddress = _keep,
    Map<int, ChannelSetting>? monoConfigs,
    Map<int, ChannelSetting>? stereoConfigs,
    Map<int, ChannelSetting>? multiConfigs,
    List<RoomConfig>? rooms,
    Object? globalTrajectory = _keep,
    List<RoomZone>? roomZones,
    bool? isExhibitionMode,
    double? masterHeadroomDb,
    bool? peakLimiterEnabled,
    List<String>? oscWhitelist,
    double? globalReverbMix,
    double? globalReverbDecay,
  }) =>
      AppConfig(
        oscPort: oscPort ?? this.oscPort,
        deviceName: identical(deviceName, _keep) ? this.deviceName : deviceName as String?,
        bufferSize: bufferSize ?? this.bufferSize,
        themeStartOscAddress: themeStartOscAddress ?? this.themeStartOscAddress,
        systemResetOscAddress: systemResetOscAddress ?? this.systemResetOscAddress,
        trackingOscAddress:
            identical(trackingOscAddress, _keep) ? this.trackingOscAddress : trackingOscAddress as String?,
        monoConfigs: monoConfigs ?? this.monoConfigs,
        stereoConfigs: stereoConfigs ?? this.stereoConfigs,
        multiConfigs: multiConfigs ?? this.multiConfigs,
        rooms: rooms ?? this.rooms,
        globalTrajectory:
            identical(globalTrajectory, _keep) ? this.globalTrajectory : globalTrajectory as Trajectory?,
        roomZones: roomZones ?? this.roomZones,
        isExhibitionMode: isExhibitionMode ?? this.isExhibitionMode,
        masterHeadroomDb: masterHeadroomDb ?? this.masterHeadroomDb,
        peakLimiterEnabled: peakLimiterEnabled ?? this.peakLimiterEnabled,
        oscWhitelist: oscWhitelist ?? this.oscWhitelist,
        globalReverbMix: globalReverbMix ?? this.globalReverbMix,
        globalReverbDecay: globalReverbDecay ?? this.globalReverbDecay,
      );
}

/// copyWith에서 "넘기지 않음"을 null과 구분하는 표시.
const Object _keep = Object();
