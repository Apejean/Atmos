import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  SharedPreferences.setMockInitialValues({});

  test('AcousticSyncProvider updates tuning state on speaker move', () async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    
    // Add a room
    final room = RoomZone(id: 'room1', x: 0, y: 0, width: 10, height: 10, color: 0xFF0000, physicalWidth: 10, physicalHeight: 10);
    container.read(roomZoneProvider.notifier).updateRoomZone(room, immediate: true);
    
    // Add a speaker
    final speaker = SpeakerNode(id: 'spk1', roomId: 'room1', x: 2.0, y: 2.0, channel: 0);
    container.read(speakerLayoutProvider.notifier).addSpeaker(speaker);
    
    // Initialize acoustic sync
    container.read(acousticSyncProvider);
    
    // Wait for throttle
    await Future.delayed(const Duration(milliseconds: 50));
    
    // Check initial tuning
    var tuning = container.read(tuningStateProvider)[1];
    print('Initial Delay: ${tuning?.delay}, Gain: ${tuning?.gainDb}');
    
    // Move speaker
    final movedSpeaker = speaker.copyWith(x: 5.0, y: 5.0);
    container.read(speakerLayoutProvider.notifier).updateSpeaker(movedSpeaker);
    
    // Wait for throttle
    await Future.delayed(const Duration(milliseconds: 50));
    
    // Check updated tuning
    tuning = container.read(tuningStateProvider)[1];
    print('Updated Delay: ${tuning?.delay}, Gain: ${tuning?.gainDb}');
    
    // Move to wall to test SBIR
    final wallSpeaker = speaker.copyWith(x: 0.1, y: 5.0);
    container.read(speakerLayoutProvider.notifier).updateSpeaker(wallSpeaker);
    
    // Wait for throttle
    await Future.delayed(const Duration(milliseconds: 50));
    
    // Check updated tuning for wall
    tuning = container.read(tuningStateProvider)[1];
    print('Wall SBIR Enabled: ${tuning?.bandEnabled[0]}');
    print('Wall SBIR Gain: ${tuning?.gains[0]}');
  });
}
