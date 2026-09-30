import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

/// 베이스 매니지먼트 전체 설정(모든 방 공통).
///
/// 서브우퍼 지정은 여기가 아니라 스피커 속성이다(SpeakerNode.isSubwoofer, 방마다 하나).
/// 예전에는 여기 lfeChannel 하나로 서브를 정해서, 모든 방 스피커의 저역이 그 채널
/// 하나로 모였다.
class BassManagementState {
  /// 크로스오버 주파수(Hz). 서브가 있는 방의 메인 스피커는 이 주파수 아래를 잘라
  /// 자기 방 서브로 보낸다.
  final double crossoverFreq;
  /// LFE +10dB 토글. 서브 채널 자기 신호(.1 LFE 트랙)를 120Hz 로우패스 **이후**에
  /// +10dB 올린다. 기본은 꺼짐 — 서브 레벨은 출력단·하드웨어에서 맞춘다.
  final bool lfeBoostEnabled;

  const BassManagementState({
    this.crossoverFreq = 80.0,
    this.lfeBoostEnabled = false,
  });

  BassManagementState copyWith({
    double? crossoverFreq,
    bool? lfeBoostEnabled,
  }) {
    return BassManagementState(
      crossoverFreq: crossoverFreq ?? this.crossoverFreq,
      lfeBoostEnabled: lfeBoostEnabled ?? this.lfeBoostEnabled,
    );
  }
}

const String kBassManagementPrefsKey = 'bass_management_state';

/// 예전 저장값(서브가 전역 lfeChannel 하나였던 시절)에서 켜져 있던 서브 채널(내부
/// 0-based 번호)을 꺼낸다. 없거나 꺼져 있었으면 null. 스피커 배치를 불러올 때 그 채널
/// 스피커로 옮겨 적는다(SpeakerLayoutState).
int? legacyLfeChannelOf(String? bassJson) {
  if (bassJson == null) return null;
  try {
    final decoded = jsonDecode(bassJson);
    if (decoded is! Map<String, dynamic>) return null;
    if (decoded['isEnabled'] != true) return null;
    final channel = decoded['lfeChannel'];
    return channel is int ? channel : null;
  } catch (_) {
    return null;
  }
}

/// 예전 키(isEnabled, lfeChannel)를 뺀 저장값. 뺄 키가 없으면(또는 읽을 수 없으면) null.
/// 옮겨 적은 뒤 지워서, 사용자가 해제한 서브가 다음 실행에 되살아나지 않게 한다.
String? withoutLegacyLfeKeys(String? bassJson) {
  if (bassJson == null) return null;
  try {
    final decoded = jsonDecode(bassJson);
    if (decoded is! Map<String, dynamic>) return null;
    if (!decoded.containsKey('isEnabled') && !decoded.containsKey('lfeChannel')) {
      return null;
    }
    decoded
      ..remove('isEnabled')
      ..remove('lfeChannel');
    return jsonEncode(decoded);
  } catch (_) {
    return null;
  }
}

class BassManagementNotifier extends Notifier<BassManagementState> {
  @override
  BassManagementState build() {
    _loadFromPrefs();
    return const BassManagementState();
  }

  /// 재동기화 호출 횟수(테스트 검증용).
  int resyncCount = 0;

  /// 엔진이 (재)기동된 뒤 베이스 매니지먼트 설정을 다시 보낸다.
  /// 엔진(config.json)은 이 값들을 모르므로 여기서 보내야 한다. 서브우퍼 지정은
  /// 공간 설정 payload(spatial_sync_provider.dart)로 따로 간다.
  void resyncToBackend() {
    resyncCount++;
    try {
      rust_api.apiSetCrossoverFrequency(freq: state.crossoverFreq);
      rust_api.apiSetLfeBoostEnabled(enabled: state.lfeBoostEnabled);
    } catch (_) {
      // 엔진 미준비 시 건너뛴다.
    }
  }

  /// 프로젝트 파일을 불러온 뒤 저장소를 다시 읽는다(core/state/project_file.dart).
  Future<void> reloadFromPrefs() => _loadFromPrefs(resetWhenAbsent: true);

  /// 앱을 다시 켜도 크로스오버·LFE +10dB가 유지되도록 저장값을 불러온다.
  Future<void> _loadFromPrefs({bool resetWhenAbsent = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(kBassManagementPrefsKey);
    if (jsonString == null) {
      // 프로젝트 전환으로 다시 읽는 경우에만 기본 상태로 되돌린다. 앱 시작
      // 경로에서 되돌리면 이미 화면에서 조작한 값을 지울 수 있다.
      if (resetWhenAbsent) {
        state = const BassManagementState();
        resyncToBackend();
      }
      return;
    }
    try {
      final decoded = jsonDecode(jsonString) as Map<String, dynamic>;
      // 예전 키(isEnabled, lfeChannel)는 여기서 읽지 않는다. 서브 지정은 스피커 배치를
      // 불러올 때 그 채널 스피커로 옮겨진다(legacyLfeChannelOf).
      state = BassManagementState(
        crossoverFreq: (decoded['crossoverFreq'] as num?)?.toDouble() ?? 80.0,
        lfeBoostEnabled: decoded['lfeBoostEnabled'] as bool? ?? false,
      );
      // 앱 부팅 흐름의 재동기화가 이 비동기 로드보다 먼저 끝날 수 있으므로
      // 로드 완료 시점에 한 번 더 보낸다(spatial_reverb_state와 같은 이유).
      resyncToBackend();
    } catch (_) {
      // 손상된 저장값은 기본 상태로 둔다.
    }
  }

  /// 바로 저장한다. 변경이 드물고(크로스오버 슬라이더도 10Hz 단위 17단계)
  /// 디바운스를 두면 바꾼 직후 앱을 닫을 때 값이 사라질 수 있다.
  Future<void> _saveToPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      kBassManagementPrefsKey,
      jsonEncode({
        'crossoverFreq': state.crossoverFreq,
        'lfeBoostEnabled': state.lfeBoostEnabled,
      }),
    );
  }

  /// LFE +10dB 토글(120Hz 로우패스 이후에만 적용).
  void setLfeBoostEnabled(bool enabled) {
    state = state.copyWith(lfeBoostEnabled: enabled);
    _saveToPrefs();
    try {
      rust_api.apiSetLfeBoostEnabled(enabled: enabled);
    } catch (_) {
      // 엔진 미준비 시 건너뛴다(기동 후 resyncToBackend가 보낸다).
    }
  }

  void setCrossoverFreq(double freq) {
    state = state.copyWith(crossoverFreq: freq);
    _saveToPrefs();
    try {
      rust_api.apiSetCrossoverFrequency(freq: freq.toDouble());
    } catch (_) {
      // 엔진 미준비 시 건너뛴다(기동 후 resyncToBackend가 보낸다).
    }
  }
}

final bassManagementProvider =
    NotifierProvider<BassManagementNotifier, BassManagementState>(
        BassManagementNotifier.new);
