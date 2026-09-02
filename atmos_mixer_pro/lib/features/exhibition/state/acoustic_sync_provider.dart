import 'dart:async';
import 'dart:math' as math;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';

class AcousticSyncProvider extends Notifier<void> {
  Timer? _throttleTimer;

  @override
  void build() {
    // Watch speaker and room states to trigger recalculation
    ref.listen(speakerLayoutProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(roomZoneProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(environmentStateProvider, (previous, next) {
      _scheduleRecalculation();
    });
    ref.listen(bassManagementProvider, (previous, next) {
      _scheduleRecalculation();
    });
  }

  void _scheduleRecalculation() {
    if (_throttleTimer?.isActive ?? false) return;
    _throttleTimer = Timer(const Duration(milliseconds: 16), () {
      _recalculateAndSync();
    });
  }

  void _recalculateAndSync() {
    final speakers = ref.read(speakerLayoutProvider);
    final rooms = ref.read(roomZoneProvider);
    final tuningNotifier = ref.read(tuningStateProvider.notifier);

    bool changed = false;

    for (final speaker in speakers) {
      if (rooms.isEmpty) continue;
      final room = rooms.firstWhere((r) => r.id == speaker.roomId, orElse: () => rooms.first);
      final env = ref.read(environmentStateProvider);
      
      final chKey = speaker.channel + 1;
      final currentTuning = tuningNotifier.getTuning(chKey);

      // 1. Lock 튜닝 채널은 무시 (덮어쓰기 방지)
      if (currentTuning.isTuningLocked) {
        continue;
      }

      // 리스너 좌표 (방 정중앙)
      final double xEar = room.physicalWidth / 2.0;
      final double yEar = room.physicalHeight / 2.0;
      final double zEar = room.earLevel;

      // 현재 스피커와 리스너 간의 거리 계산
      final double dx = speaker.x - xEar;
      final double dy = speaker.y - yEar;
      final double dz = speaker.heightZ - zEar;
      final double distance = math.sqrt(dx * dx + dy * dy + dz * dz);

      // 타임 얼라인먼트 딜레이를 위해 가장 먼 스피커 찾기
      double maxDistance = 0.0;
      double refDistance = double.infinity;
      
      for (final spk in speakers) {
         if (spk.roomId == room.id) {
             final sdx = spk.x - xEar;
             final sdy = spk.y - yEar;
             final sdz = spk.heightZ - zEar;
             final dist = math.sqrt(sdx * sdx + sdy * sdy + sdz * sdz);
             if (dist > maxDistance) maxDistance = dist;
             if (dist < refDistance) refDistance = dist;
         }
      }
      
      // 1. 타임 얼라인먼트 딜레이 계산 (ms) - 현재 온도 기준 동적 음속
      // 절대 비행 시간(Time of Flight)을 기준으로 하여 스피커 1개일 때도 변화가 보이도록 함
      final double speedOfSound = env.speedOfSound; 
      double delayMs = (distance / speedOfSound) * 1000.0;
      delayMs = delayMs.clamp(0.0, 50.0);

      // 2. 거리 감쇠 게인 보정 (Inverse Square Law)
      // 기준 거리를 1.0m로 고정하여 절대적인 감쇠량을 보여줌
      double gainDb = 20.0 * math.log(1.0 / distance) / math.ln10;
      
      // [신규 로직] LFE 서브우퍼 채널인 경우 +10dB 부스트 (국제 방송 표준 헤드룸 보상)
      final bmState = ref.read(bassManagementProvider);
      if (bmState.isEnabled && bmState.lfeChannel == speaker.channel) {
          gainDb += 10.0;
      }
      
      gainDb = gainDb.clamp(-24.0, 12.0); // +10dB 부스트를 수용하기 위해 상한선을 12.0dB로 확장

      // 3. SBIR (Speaker Boundary Interference Response) 보정 EQ
      final double wallDistX = math.min(speaker.x, room.physicalWidth - speaker.x);
      final double wallDistY = math.min(speaker.y, room.physicalHeight - speaker.y);
      final double dWall = math.min(wallDistX, wallDistY);

      List<bool> bandEnabled = List.from(currentTuning.bandEnabled);
      List<EqType> bandTypes = List.from(currentTuning.bandTypes);
      List<double> freqs = List.from(currentTuning.freqs);
      List<double> gains = List.from(currentTuning.gains);
      List<double> qs = List.from(currentTuning.qs);

      // Dynamic EQ Allocation helper
      void applyEq(double targetFreq, EqType type, double targetGain, double targetQ) {
         int allocatedIndex = -1;
         for (int i=0; i<8; i++) {
             if (bandEnabled[i] && bandTypes[i] == type && freqs[i] == targetFreq && qs[i] == targetQ) {
                 allocatedIndex = i;
                 break;
             }
         }
         
         if (targetGain == 0.0) {
            if (allocatedIndex != -1) {
                bandEnabled[allocatedIndex] = false;
                gains[allocatedIndex] = 0.0;
            }
            return;
         }

         if (allocatedIndex != -1) {
             gains[allocatedIndex] = targetGain;
             return;
         }

         for (int i=0; i<8; i++) {
             if (!bandEnabled[i] || (gains[i] == 0.0 && bandTypes[i] == EqType.bell)) {
                 bandEnabled[i] = true;
                 bandTypes[i] = type;
                 freqs[i] = targetFreq;
                 qs[i] = targetQ;
                 gains[i] = targetGain;
                 return;
             }
         }
      }

      // 3. SBIR (Speaker Boundary Interference Response) 보정 EQ (저음 부밍)
      double sbirGain = 0.0;
      if (dWall <= 0.5) {
         sbirGain = -4.5;
      } else if (dWall <= 1.0) {
         sbirGain = -2.0;
      }
      applyEq(150.0, EqType.lowShelf, sbirGain, 0.707);

      // 4. Air Absorption (공기 흡음에 의한 고음역 롤오프 보상)
      double airAbsGain = 0.0;
      if (distance > 5.0) {
         airAbsGain = (distance - 5.0) * 0.5; // +0.5 dB per meter beyond 5m
         airAbsGain = airAbsGain.clamp(0.0, 6.0);
      }
      applyEq(10000.0, EqType.highShelf, airAbsGain, 0.707);

      // 5. 마주보는 스피커 간 파형 상쇄 방지를 위한 위상 반전 자동화 (Phase Invert)
      // 단순히 뒷벽(1cm/50cm)에 붙었는지가 아니라, "청취자(yEar)를 기준으로 뒤편에 배치되어 전면 스피커와 마주보게 되는가"가 핵심입니다.
      bool phaseInvert = currentTuning.phaseInvert;
      
      // 청취자 기준 뒤쪽으로 1.0m 이상 배치된 스피커는 서라운드(후면) 스피커로 간주하여 
      // 메인 스피커와의 저음역대 상쇄 간섭(Cancellation)을 막기 위해 위상을 180도 반전시킵니다.
      if (speaker.y > yEar + 1.0) {
         phaseInvert = true;
      } else {
         // 청취자 앞쪽(Front) 또는 중앙에 위치한 경우 정상 위상(Normal) 유지
         phaseInvert = false;
      }

      final updatedTuning = currentTuning.copyWith(
        delay: delayMs,
        phaseInvert: phaseInvert,
        gainDb: gainDb,
        bandEnabled: bandEnabled,
        bandTypes: bandTypes,
        freqs: freqs,
        gains: gains,
        qs: qs,
      );

      debugPrint('AcousticSync: ch $chKey -> delay: $delayMs, gain: $gainDb, sbirGain: $sbirGain, airGain: $airAbsGain, phaseInvert: $phaseInvert');

      tuningNotifier.saveTuning(chKey, updatedTuning);
      changed = true;
    }

    if (changed) {
      tuningNotifier.applyAllToBackend();
      debugPrint('AcousticSync: applyAllToBackend called.');
    }
  }
}

final acousticSyncProvider = NotifierProvider<AcousticSyncProvider, void>(AcousticSyncProvider.new);
