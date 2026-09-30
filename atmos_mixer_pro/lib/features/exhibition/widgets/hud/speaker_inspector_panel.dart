import 'package:flutter/material.dart';
import 'dart:math' as math;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/reverb_settings_modal.dart';

class CrossoverCurveIcon extends StatelessWidget {
  final Color color;
  final double size;
  const CrossoverCurveIcon({
    super.key,
    this.color = const Color(0xFF00E5FF),
    this.size = 18.0,
  });

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      size: Size(size, size),
      painter: _CrossoverCurvePainter(color: color),
    );
  }
}

class _CrossoverCurvePainter extends CustomPainter {
  final Color color;
  _CrossoverCurvePainter({required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final basePaint = Paint()
      ..color = Colors.white24
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke;
    canvas.drawLine(Offset(0, size.height * 0.7), Offset(size.width, size.height * 0.7), basePaint);

    final curvePaint = Paint()
      ..color = color
      ..strokeWidth = 1.8
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final path = Path()
      ..moveTo(0, size.height * 0.3)
      ..lineTo(size.width * 0.35, size.height * 0.3)
      ..cubicTo(size.width * 0.65, size.height * 0.3, size.width * 0.7, size.height * 0.85, size.width, size.height * 0.85);

    canvas.drawPath(path, curvePaint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final String speakerId;
  final VoidCallback onClose;

  const SpeakerInspectorPanel({
    super.key,
    required this.speakerId,
    required this.onClose,
  });

  @override
  ConsumerState<SpeakerInspectorPanel> createState() => _SpeakerInspectorPanelState();
}

class _SpeakerInspectorPanelState extends ConsumerState<SpeakerInspectorPanel> {
  Widget _buildControlBox(
    String iconPath,
    String label,
    double value,
    String unit,
    double min,
    double max,
    Function(double)? onChanged,
  ) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.only(bottom: 4),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.05),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Column(
        children: [
          Row(
            children: [
              Container(
                width: 48,
                height: 48,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  border: Border(right: BorderSide(color: Colors.white.withValues(alpha: 0.1))),
                ),
                child: SvgPicture.asset(iconPath, width: 24, height: 24, colorFilter: const ColorFilter.mode(Colors.lightBlueAccent, BlendMode.srcIn)),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  child: GestureDetector(
                    onDoubleTap: onChanged == null ? null : () {
                      _showEditDialog(label, value, min, max, onChanged);
                    },
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Expanded(child: Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12), overflow: TextOverflow.ellipsis)),
                        Flexible(child: Text('${value.toStringAsFixed(1)}$unit', style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold), overflow: TextOverflow.ellipsis)),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
          SizedBox(
            height: 24,
            child: SliderTheme(
              data: SliderThemeData(
                trackHeight: 2.0,
                activeTrackColor: Colors.lightBlueAccent,
                inactiveTrackColor: Colors.white.withValues(alpha: 0.1),
                thumbColor: Colors.lightBlueAccent,
                overlayColor: Colors.lightBlueAccent.withValues(alpha: 0.2),
                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6.0),
                overlayShape: const RoundSliderOverlayShape(overlayRadius: 14.0),
              ),
              child: GestureDetector(
                onDoubleTap: onChanged == null ? null : () {
                  final middle = (min + max) / 2;
                  onChanged(middle);
                },
                child: Slider(
                  value: value.clamp(min, max),
                  min: min,
                  max: max,
                  onChanged: onChanged,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _showEditDialog(String label, double currentValue, double min, double max, Function(double) onChanged) async {
    final controller = TextEditingController(text: currentValue.toString());
    await showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: const Color(0xFF1E2632),
        title: Text('Edit $label', style: const TextStyle(color: Colors.white)),
        content: TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          style: const TextStyle(color: Colors.white),
          autofocus: true,
          decoration: InputDecoration(
            hintText: 'Min: $min, Max: $max',
            hintStyle: const TextStyle(color: Colors.white30),
            enabledBorder: const UnderlineInputBorder(borderSide: BorderSide(color: Colors.white30)),
            focusedBorder: const UnderlineInputBorder(borderSide: BorderSide(color: Colors.lightBlueAccent)),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel', style: TextStyle(color: Colors.white54)),
          ),
          TextButton(
            onPressed: () {
              final val = double.tryParse(controller.text);
              if (val != null) {
                onChanged(val.clamp(min, max));
              }
              Navigator.pop(context);
            },
            child: const Text('Apply', style: TextStyle(color: Colors.lightBlueAccent)),
          ),
        ],
      ),
    );
  }

  /// 채널 목록을 아직 신뢰할 수 없을 때 채널 번호 뒤에 붙일 짧은 상태 문구.
  String _channelStatusNote(OutputChannelsState channels) {
    switch (channels.status) {
      case OutputChannelsStatus.loading:
        return '채널 조회 중';
      case OutputChannelsStatus.noDevice:
        return '출력 장치 없음';
      case OutputChannelsStatus.error:
        return '채널 조회 실패';
      case OutputChannelsStatus.ready:
        return '현재 장치에 없음';
    }
  }

  /// 개별 출력 채널 1개의 드롭다운 항목. 스피커 레이아웃과 FX는 스테레오/멀티
  /// 그룹핑 없이 개별 채널만 잡으므로 `buildChannelRoutingItems`(트랙 라우팅용)
  /// 를 쓰지 않고 평면 목록을 만든다.
  ///
  /// 라벨은 `Ch-${i + 1}` 규약을 다른 UI와 공유하고, 인터페이스가 보고한
  /// 채널 이름이 있으면 뒤에 덧붙인다.
  DropdownMenuItem<int> _channelItem(
    int i,
    List<String> channelNames,
    List<SpeakerNode> speakers,
    SpeakerNode speaker,
  ) {
    final inUseBy = speakers
        .where((s) => s.channel == i && s.id != speaker.id)
        .firstOrNull;

    // 라벨 템플릿을 여기서 만들지 않는다. 세 UI가 channelDisplayName 하나만
    // 쓰도록 모아 같은 채널이 같은 이름으로 보이는 것을 보장한다.
    var label = channelDisplayName(i, channelNames);
    if (inUseBy != null) {
      final shortId = inUseBy.id.substring(0, math.min(3, inUseBy.id.length));
      label += ' (In Use: $shortId)';
    }

    return DropdownMenuItem(
      value: i,
      child: Row(
        children: [
          Text(
            label,
            style: TextStyle(
              color: inUseBy != null ? Colors.white54 : Colors.white,
            ),
          ),
          if (speaker.channel == i)
            const Padding(
              padding: EdgeInsets.only(left: 8.0),
              child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
            ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final layout = ref.watch(speakerLayoutProvider);
    final rooms = ref.watch(roomZoneProvider);
    // 채널 목록은 실제 인식된 오디오 인터페이스 출력이 유일한 진실 원천이다.
    // 예전에는 engineState.outputChannelCount(개수만)를 써서 채널 이름을 알 수
    // 없었고, 다른 UI와 라벨이 달랐다.
    final outputChannels = ref.watch(outputChannelsProvider);
    final channelNames = outputChannels.channelNames;
    // 드라이버 내부 가상 채널(DAW 리턴 등)은 스피커를 물리적으로 연결할 수
    // 없으므로 목록에서 뺀다. 인덱스는 원래 하드웨어 인덱스를 그대로 쓴다.
    final selectableChannels = physicalOutputChannelIndices(channelNames);
    final speakers = layout;
    final speaker = layout.where((s) => s.id == widget.speakerId).firstOrNull;

    if (speaker == null) return const SizedBox.shrink();

    // 같은 채널을 다른 방에도 쓰면 그 채널에는 **지금 보고 있는 방**의 스피커 설정이
    // 적용된다(speaker_node.dart channelRepresentatives). 방을 바꾸면 값이 바뀌는 이유를 알린다.
    final sharedRoomLabels = <String>{
      for (final s in layout)
        if (s.id != speaker.id && s.channel == speaker.channel && s.roomId != speaker.roomId)
          rooms.where((r) => r.id == s.roomId).firstOrNull?.label ?? '방 없음',
    };

    double roomW = 10.0;
    double roomD = 10.0;
    double roomH = 5.0;
    
    if (rooms.isNotEmpty) {
      final activeRoom = rooms.firstWhere((r) => r.id == speaker.roomId, orElse: () => rooms.first);
      roomW = activeRoom.physicalWidth;
      roomD = activeRoom.physicalHeight;
      roomH = activeRoom.ceilingHeight;
    }

    return Container(
      width: 320,
      decoration: BoxDecoration(
        color: const Color(0xFF151921).withValues(alpha: 0.85),
        border: Border(left: BorderSide(color: Colors.white.withValues(alpha: 0.1), width: 1)),
        boxShadow: [BoxShadow(color: Colors.black.withValues(alpha: 0.5), blurRadius: 20)],
      ),
      child: Column(
        children: [
          // Header
          Container(
            height: 60,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            decoration: BoxDecoration(border: Border(bottom: BorderSide(color: Colors.white.withValues(alpha: 0.1)))),
            child: Row(
              children: [
                Expanded(
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<int>(
                      // 저장된 채널이 현재 장치의 범위를 넘으면(더 작은 장치로
                      // 교체된 경우) 조용히 Ch-1로 바꿔 보여주지 않는다. 아래에서
                      // "범위 초과" 항목을 따로 만들어 실제 저장값을 그대로
                      // 유지하고 사용자에게 드러낸다.
                      value: speaker.channel,
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: [
                        // 드라이버 내부 가상 채널(DAW 리턴 등)은 스피커를
                        // 물리적으로 연결할 수 없으므로 목록에서 뺀다.
                        // 인덱스는 원래 하드웨어 인덱스를 그대로 쓴다.
                        for (final i in selectableChannels)
                          _channelItem(i, channelNames, speakers, speaker),
                        // 저장값이 범위를 넘으면 그 값 자체를 항목으로 추가해야
                        // DropdownButton이 assert로 죽지 않는다.
                        if (!selectableChannels.contains(speaker.channel))
                          DropdownMenuItem(
                            value: speaker.channel,
                            child: Text(
                              // 채널 목록을 아직 못 받은 상태(조회 중/장치
                              // 미인식/조회 실패)에서 "현재 장치에 없음"이라고
                              // 하면 거짓이다. 그 채널이 없는 게 아니라 목록을
                              // 모르는 것이므로 상태를 그대로 알린다.
                              outputChannels.status ==
                                      OutputChannelsStatus.ready
                                  ? channelOutOfRangeName(speaker.channel)
                                  : 'Ch-${speaker.channel + 1} '
                                        '(${_channelStatusNote(outputChannels)})',
                              style: const TextStyle(color: Colors.orangeAccent),
                            ),
                          ),
                      ],
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(channel: val));
                        }
                      },
                    ),
                  ),
                ),
                IconButton(icon: const Icon(Icons.close, color: Colors.white54), onPressed: widget.onClose),
              ],
            ),
          ),
          
          // Body
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                if (sharedRoomLabels.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Text(
                      'CH${speaker.channel + 1}을(를) ${sharedRoomLabels.join(', ')} 방에도 씁니다 · '
                      '이 채널에는 지금 보고 있는 방의 스피커 설정(위치·EQ·게인·딜레이)이 적용됩니다.',
                      style: const TextStyle(color: Colors.orangeAccent, fontSize: 10),
                    ),
                  ),
                // 1. Speaker Inspector Accordion
                Theme(
                  data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF1E2632).withValues(alpha: 0.8),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                    ),
                    child: Material(
                      color: Colors.transparent,
                      child: ExpansionTile(
                        initiallyExpanded: true,
                        collapsedIconColor: Colors.white54,
                        iconColor: Colors.white,
                        leading: const Icon(Icons.speaker_outlined, size: 18, color: Colors.white70),
                        title: const Text('Speaker Inspector', style: TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.bold)),
                        children: [
                          Padding(
                            padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
                            child: Column(
                              children: [
                                _buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', 0.25, roomW - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, x: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', 0.25, roomD - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, y: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, z: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_yaw.svg', 'Yaw (Rotation)', speaker.rotation, '°', -180.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, rot: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Pitch (Tilt)', speaker.pitchTilt, '°', -90.0, 90.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, tilt: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', 10.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, disp: v)),
                                _buildControlBox('assets/3d_simulator/icons/icon_pan.svg', 'Pan Trim', speaker.panDeg, '°', -45.0, 45.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, pan: v)),
                                // 스피커 저역 한계: 자동 EQ가 이 주파수에 보호용 로우컷(12dB/oct)을
                                // 건다. 서브우퍼로 지정하거나 베이스 매니지먼트로 저역을 서브에
                                // 넘기는 동안에는 쓰지 않는다(position_eq.dart).
                                _buildControlBox('assets/3d_simulator/icons/icon_lowcut.svg', 'Low Limit', speaker.lowCutHz, 'Hz', 20.0, 200.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, lowCut: v)),
                                const SizedBox(height: 8),
                                // Auto-Aim Button
                                          if (!speaker.isFixed)
                                            Padding(
                                              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                                              child: SizedBox(
                                                width: double.infinity,
                                                child: OutlinedButton.icon(
                                                  onPressed: () {
                                                    // 각도를 여기서 직접 계산해 박아넣지 않고 자동 조준을
                                                    // 켜기만 한다. 실제 yaw/pitch는 speaker_layout_state의
                                                    // _applyAutoAim이 채우므로, 이후 스피커를 옮겨도 계속
                                                    // 청취자를 따라 조준한다(버튼 이름 그대로).
                                                    _updateSpeaker(speaker, autoAimOn: true);
                                                  },
                                                  icon: const Icon(Icons.my_location_rounded, size: 18),
                                                  label: const Text('Auto-Aim to Listener'),
                                                  style: OutlinedButton.styleFrom(
                                                    foregroundColor: const Color(0xFF22C55E),
                                                    side: const BorderSide(color: Color(0xFF22C55E)),
                                                    padding: const EdgeInsets.symmetric(vertical: 12),
                                                  ),
                                                ),
                                              ),
                                            ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                
                const SizedBox(height: 8),
                // 2. Spatial Reverb Card
                _buildReverbSendCard(context, speaker),
                
                // 3. Bass Management Accordion
                _buildBassManagementCard(context, speaker),
              ],
            ),
          ),

          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    onPressed: () {
                      _updateSpeaker(speaker, isFixed: !speaker.isFixed);
                    },
                    icon: Icon(
                      speaker.isFixed ? Icons.lock_rounded : Icons.lock_open_rounded,
                      size: 18,
                      color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                    ),
                    label: Text(
                      speaker.isFixed ? 'FIX ON' : 'FIX OFF',
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                      ),
                    ),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: const Color(0xFF0F172A),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                        side: BorderSide(
                          color: speaker.isFixed ? const Color(0xFFF59E0B) : const Color(0xFF334155),
                          width: 1.5,
                        ),
                      ),
                      elevation: 0,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: () {
                      ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker.id);
                      widget.onClose();
                    },
                    icon: const Icon(Icons.delete_outline_rounded, size: 18, color: Colors.redAccent),
                    label: const Text('Remove', style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: Colors.redAccent, width: 1.5),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildReverbSendCard(BuildContext context, SpeakerNode speaker) {
    final targetCh = speaker.channel >= 0 ? speaker.channel + 1 : 1;
    final reverbState = ref.watch(spatialReverbProvider);
    final chSettings = reverbState.getSettingsForChannel(targetCh);
    final isEnabled = chSettings.isEnabled;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1E2632).withValues(alpha: 0.8),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isEnabled
              ? const Color(0xFFFFA000).withValues(alpha: 0.35)
              : Colors.white.withValues(alpha: 0.1),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 30,
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: const Color(0xFFFFA000).withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(
                  Icons.waves_rounded,
                  size: 16,
                  color: Color(0xFFFFA000),
                ),
              ),
              const SizedBox(width: 8),
              const Expanded(
                child: Text(
                  'SPATIAL REVERB',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: const Color(0xFF0E1219),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: isEnabled
                        ? const Color(0xFFFFA000).withValues(alpha: 0.5)
                        : Colors.white24,
                  ),
                ),
                child: Text(
                  isEnabled
                      ? '${chSettings.reverbType.label.toUpperCase()} (${chSettings.dryWetPercent.toInt()}%)'
                      : 'OFF',
                  style: TextStyle(
                    color: isEnabled ? const Color(0xFFFFA000) : Colors.white38,
                    fontSize: 11,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Open Per-Channel Reverb Rack Shortcut
          SizedBox(
            width: double.infinity,
            child: OutlinedButton.icon(
              onPressed: () {
                showDialog(
                  context: context,
                  builder: (_) => ReverbSettingsModal(initialChannel: targetCh),
                );
              },
              icon: const Icon(Icons.tune_rounded, size: 14, color: Color(0xFFFFA000)),
              label: Text(
                'Open Output CH $targetCh Reverb Rack',
                style: const TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.bold,
                  color: Color(0xFFFFA000),
                ),
              ),
              style: OutlinedButton.styleFrom(
                side: BorderSide(color: const Color(0xFFFFA000).withValues(alpha: 0.5)),
                padding: const EdgeInsets.symmetric(vertical: 8),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
                backgroundColor: const Color(0xFF131923),
              ),
            ),
          ),
          const SizedBox(height: 10),

          // Reverb Send: 이 스피커가 채널 리버브(잔향)에 보내는 양.
          // 실제로 걸리는 잔향 = 랙 MIX x Send. 초기 반사에는 영향이 없다.
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Reverb Send', style: TextStyle(color: Colors.white70, fontSize: 12)),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                child: Text('${(speaker.reverbSendNormalized * 100).round()}%', style: const TextStyle(color: Color(0xFFFFA000), fontSize: 11, fontWeight: FontWeight.bold)),
              ),
            ],
          ),
          SliderTheme(
            data: SliderTheme.of(context).copyWith(
              activeTrackColor: const Color(0xFFFFA000),
              thumbColor: const Color(0xFFFFA000),
              trackHeight: 2.0,
            ),
            child: Slider(
              value: speaker.reverbSendNormalized,
              min: 0.0,
              max: 1.0,
              onChanged: (v) => _updateSpeaker(speaker, rev: v),
            ),
          ),
          const SizedBox(height: 6),

          // 연출용 초기반사 효과.
          //
          // 방 자체의 반사(방 크기·재질로 계산)는 바이노럴을 켤 때 엔진이 자동으로
          // 적용한다(rust/src/audio/mixer.rs의 refresh_early_ref_mix). 현장에서는 실제
          // 벽이 그 역할을 하므로 자동으로 꺼지고, 이 슬라이더만 남는다.
          // 100%가 직접음 대비 -6dB로 정규화돼 방이 달라도 감각이 같다.
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Early Reflections', style: TextStyle(color: Colors.white70, fontSize: 12)),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                child: Text('${(speaker.earlyRefMix * 100).toInt()}%', style: const TextStyle(color: Color(0xFFFFA000), fontSize: 11, fontWeight: FontWeight.bold)),
              ),
            ],
          ),
          SliderTheme(
            data: SliderTheme.of(context).copyWith(
              activeTrackColor: const Color(0xFFFFA000),
              thumbColor: const Color(0xFFFFA000),
              trackHeight: 2.0,
            ),
            child: Slider(
              value: speaker.earlyRefMix.clamp(0.0, 1.0),
              min: 0.0,
              max: 1.0,
              onChanged: (v) => _updateSpeaker(speaker, earlyRef: v),
            ),
          ),
          const Text(
            '연출용입니다. 방 반사는 바이노럴을 켜면 자동 적용됩니다.',
            style: TextStyle(color: Colors.white38, fontSize: 10),
          ),
        ],
      ),
    );
  }

  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, double? earlyRef, double? lowCut, bool? isFixed, bool? autoAimOn}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      // Yaw나 Pitch를 직접 돌리면 자동 조준을 해제하고 지정한 각도를 쓴다.
      // 'Auto-Aim to Listener' 버튼은 반대로 다시 켠다.
      autoAim: autoAimOn ?? ((rot != null || tilt != null) ? false : speaker.autoAim),
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
      earlyRefMix: earlyRef ?? speaker.earlyRefMix,
      lowCutHz: lowCut ?? speaker.lowCutHz,
      isFixed: isFixed ?? speaker.isFixed,
    ));
    // Trigger real-time sync via global state or similar if needed.
  }

  Widget _buildBassManagementCard(BuildContext context, SpeakerNode speaker) {
    final bmState = ref.watch(bassManagementProvider);
    // 서브 지정은 스피커 속성이다(방마다 하나). 크로스오버·LFE +10dB는 모든 방 공통.
    final isLfe = speaker.isSubwoofer;

    return Theme(
      data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
      child: Container(
        margin: const EdgeInsets.only(top: 8),
        decoration: BoxDecoration(
          color: const Color(0xFF1E2632).withValues(alpha: 0.8),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isLfe
                ? const Color(0xFFFF5722).withValues(alpha: 0.35)
                : Colors.white.withValues(alpha: 0.1),
          ),
        ),
        child: Material(
          color: Colors.transparent,
          child: ExpansionTile(
            initiallyExpanded: false,
            collapsedIconColor: Colors.white54,
            iconColor: Colors.white,
            leading: const CrossoverCurveIcon(color: Color(0xFF00E5FF), size: 18),
            title: Row(
              children: [
                const Expanded(child: Text('Bass Management', style: TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.bold), overflow: TextOverflow.ellipsis)),
                if (isLfe) ...[
                  const SizedBox(width: 4),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                    decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                    child: const Text('LFE', style: TextStyle(color: Color(0xFFFF5722), fontSize: 10, fontWeight: FontWeight.bold)),
                  ),
                ],
              ],
            ),
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
                child: Column(
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Set as LFE Subwoofer', style: TextStyle(color: Colors.white70, fontSize: 12)),
                        Switch(
                          value: isLfe,
                          activeThumbColor: const Color(0xFFFF5722),
                          onChanged: (val) {
                            ref.read(speakerLayoutProvider.notifier).setSubwoofer(speaker.id, val);
                          },
                        ),
                      ],
                    ),
                    const Text(
                      '이 방의 서브우퍼로 씁니다 · 방마다 하나(같은 방 다른 스피커를 켜면 옮겨집니다) · 이 방 메인의 저역만 받습니다.',
                      style: TextStyle(color: Colors.white38, fontSize: 10),
                    ),
                    // LFE +10dB: 서브 채널 자기 신호(.1 LFE 트랙)를 120Hz 로우패스
                    // **이후**에 +10dB 올린다. 메인에서 넘어온 저역에는 걸지 않는다.
                    // 서브 지정과 상관없이 켜고 끌 수 있게 항상 보여준다(전체 설정이다).
                    // 서브 레벨은 원래 출력단·하드웨어에서 맞추고, 이건 바이노럴
                    // 미리듣기나 소프트웨어로 맞춰야 할 때 쓴다.
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('LFE +10dB', style: TextStyle(color: Colors.white70, fontSize: 12)),
                        Switch(
                          value: bmState.lfeBoostEnabled,
                          activeThumbColor: const Color(0xFFFF5722),
                          onChanged: (val) {
                            ref.read(bassManagementProvider.notifier).setLfeBoostEnabled(val);
                          },
                        ),
                      ],
                    ),
                    const Text(
                      '.1(LFE) 트랙에만 · 120Hz 로우패스 이후 적용 · 서브우퍼로 지정된 채널에서 동작 · 모든 방 공통',
                      style: TextStyle(color: Colors.white38, fontSize: 10),
                    ),
                    if (isLfe) ...[
                      const Divider(color: Colors.white10, height: 16),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text('Crossover Freq', style: TextStyle(color: Colors.white70, fontSize: 12)),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                            decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                            child: Text('${bmState.crossoverFreq.toInt()} Hz', style: const TextStyle(color: Color(0xFFFF5722), fontSize: 12, fontWeight: FontWeight.bold)),
                          ),
                        ],
                      ),
                      SliderTheme(
                        data: SliderTheme.of(context).copyWith(
                          activeTrackColor: const Color(0xFFFF5722),
                          thumbColor: const Color(0xFFFF5722),
                          trackHeight: 2.0,
                        ),
                        child: Slider(
                          value: bmState.crossoverFreq,
                          min: 40.0,
                          max: 200.0,
                          divisions: 16,
                          onChanged: (val) {
                            ref.read(bassManagementProvider.notifier).setCrossoverFreq(val);
                          },
                        ),
                      ),
                      // 크로스오버는 메인 스피커 쪽 설정이다: 메인의 이 주파수 아래를 잘라
                      // 자기 방 서브로 보낸다. 서브 자기 신호(.1 LFE 트랙)는 영향을 받지 않고
                      // 120Hz 대역까지 그대로 나간다. 값은 모든 방 공통이다.
                      const Text(
                        '메인 스피커의 이 주파수 아래를 잘라 자기 방 서브로 보냅니다 · 모든 방 공통 · .1(LFE) 트랙은 영향 없이 120Hz까지',
                        style: TextStyle(color: Colors.white38, fontSize: 10),
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
