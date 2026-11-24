import 'dart:async';

import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/core/utils/track_routes.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter/material.dart';

/// 표 창이 값을 바꿀 때 부르는 콜백. [commit]이 false면 드래그 중(엔진에만 반영), true면 확정(저장까지).
typedef RoutesChanged = void Function(List<TrackRoute> routes, {required bool commit});

/// 표 창 `?` 도움말(설계 7.2 문구 그대로).
const String trackRoutingHelpText = '''여러 출력으로 보내기
• 왼쪽은 파일의 채널, 위쪽은 출력(스피커)입니다.
• 빈 칸을 누르면 그 파일 채널이 그 출력으로 나갑니다. 한 줄에서 여러 칸을 켜면 같은 소리가 여러 출력에서 동시에 납니다.
• 연결된 칸을 누르면 아래에서 음량(1~100%)을 바꾸거나 연결을 끊을 수 있습니다.
• 여러 파일 채널을 같은 출력에 연결하면 소리가 더해집니다. 너무 크면 음량을 낮추세요.
• 재생 중에 바꿔도 바로 반영됩니다.
• 방 볼륨·정지·방 클리어는 이 트랙의 모든 출력에 함께 적용됩니다.
• 출력 설정(Output Config)에서 닫은 출력은 여기에 나오지 않습니다. 보내려면 먼저 출력 설정에서 여세요.
• 주황색은 지금 장치에 없거나 닫힌 출력, 또는 파일에 없는 채널입니다. 그 칸은 소리가 나지 않습니다.
• 한 출력의 음량 합이 100%를 넘으면 주황색으로 표시됩니다.
• "매핑과 같게 채우기"는 이 트랙의 표를 트랙별 출력 매핑과 같은 연결로 바꿉니다. "처음 상태로"는 이 창을 열었을 때로 되돌립니다.''';

/// 트랙 연결 표 창(매트릭스 라우팅 설계 7.2, 개정 3). 값을 스스로 저장하지 않고 [onChanged]를 부른다.
class TrackRoutingDialog extends StatefulWidget {
  const TrackRoutingDialog({
    super.key,
    required this.title,
    required this.fileChannels,
    required this.initialRoutes,
    required this.channelNames,
    required this.enabled,
    required this.limiterOn,
    required this.seedRoutes,
    required this.onChanged,
  });

  /// 예: `방 1 · 1번 트랙`.
  final String title;

  /// 파일 채널 수. 모르면 null(표에 있는 가장 높은 파일 채널까지 보인다).
  final int? fileChannels;
  final List<TrackRoute> initialRoutes;

  /// 지금 장치의 출력 채널 이름(길이 = 장치 출력 수).
  final List<String> channelNames;

  /// 엔진과 같은 규칙으로 구한 열린 출력(`computeEnabledChannels`).
  final List<bool> enabled;

  /// 프로젝트의 피크 리미터가 켜져 있나(꺼져 있으면 합친 소리 경고).
  final bool limiterOn;

  /// "매핑과 같게 채우기"의 표. 채널 수를 몰라 만들 수 없으면 null을 돌려준다.
  final List<TrackRoute>? Function() seedRoutes;
  final RoutesChanged onChanged;

  @override
  State<TrackRoutingDialog> createState() => _TrackRoutingDialogState();
}

class _TrackRoutingDialogState extends State<TrackRoutingDialog> {
  static const double _cellW = 64;
  static const double _cellH = 34;
  static const double _rowHeaderW = 96;
  static const double _colHeaderH = 58;

  late List<TrackRoute> _routes;
  (int, int)? _selected;
  bool _helpOpen = false;
  DateTime _lastDragSend = DateTime.fromMillisecondsSinceEpoch(0);
  final _hHeader = ScrollController();
  final _hBody = ScrollController();
  final _vHeader = ScrollController();
  final _vBody = ScrollController();

  @override
  void initState() {
    super.initState();
    _routes = normalizeRoutes(widget.initialRoutes);
    _hBody.addListener(() {
      if (_hHeader.hasClients) _hHeader.jumpTo(_hBody.offset);
    });
    _vBody.addListener(() {
      if (_vHeader.hasClients) _vHeader.jumpTo(_vBody.offset);
    });
  }

  @override
  void dispose() {
    _hHeader.dispose();
    _hBody.dispose();
    _vHeader.dispose();
    _vBody.dispose();
    super.dispose();
  }

  int get _deviceChannels => widget.channelNames.length;

  /// 열린 물리 출력(표의 열).
  List<int> get _openColumns => [
        for (var i = 0; i < _deviceChannels; i++)
          if (i < widget.enabled.length && widget.enabled[i] && isPhysicalOutputChannel(widget.channelNames[i])) i,
      ];

  /// 표에는 있는데 열에 없는 출력(주황 열).
  List<int> _problemColumns(List<int> open) {
    final set = open.toSet();
    return (_routes.map((r) => r.outputChannel).where((o) => !set.contains(o)).toSet().toList()..sort());
  }

  String _problemColumnName(int o) {
    if (o >= _deviceChannels) return channelOutOfRangeName(o);
    if (!isPhysicalOutputChannel(widget.channelNames[o])) return 'Ch-${o + 1} (가상 채널)';
    return 'Ch-${o + 1} (닫힘)';
  }

  int get _rowCount {
    final fromRoutes = _routes.fold<int>(0, (m, r) => r.fileChannel + 1 > m ? r.fileChannel + 1 : m);
    final known = widget.fileChannels ?? 0;
    return (known > fromRoutes ? known : fromRoutes).clamp(1, 256);
  }

  TrackRoute? _cell(int f, int o) {
    for (final r in _routes) {
      if (r.fileChannel == f && r.outputChannel == o) return r;
    }
    return null;
  }

  void _set(List<TrackRoute> routes, {required bool commit}) {
    setState(() => _routes = routes);
    widget.onChanged(routes, commit: commit);
  }

  void _tapCell(int f, int o) {
    if (_cell(f, o) == null) {
      _set(toggleRoute(_routes, f, o), commit: true);
    }
    setState(() => _selected = (f, o));
  }

  void _dragGain(double percent) {
    final sel = _selected;
    if (sel == null) return;
    final routes = setRouteGain(_routes, sel.$1, sel.$2, percent / 100);
    setState(() => _routes = routes);
    // 드래그 중에는 초당 30회 이하로 엔진에만 보낸다(설계 7.2).
    final now = DateTime.now();
    if (now.difference(_lastDragSend) >= const Duration(milliseconds: 33)) {
      _lastDragSend = now;
      widget.onChanged(routes, commit: false);
    }
  }

  void _endGain(double percent) {
    final sel = _selected;
    if (sel == null) return;
    _set(setRouteGain(_routes, sel.$1, sel.$2, percent / 100), commit: true);
  }

  void _disconnect() {
    final sel = _selected;
    if (sel == null || _routes.length <= 1) return;
    _set(toggleRoute(_routes, sel.$1, sel.$2), commit: true);
    setState(() => _selected = null);
  }

  Future<void> _seed() async {
    final seeded = widget.seedRoutes();
    if (seeded == null) {
      ScaffoldMessenger.maybeOf(context)?.showSnackBar(
        const SnackBar(content: Text('파일 채널 수를 몰라 매핑과 같은 표를 만들 수 없습니다.')),
      );
      return;
    }
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        content: const Text('이 트랙의 표가 바뀝니다. 계속할까요?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('취소')),
          TextButton(onPressed: () => Navigator.pop(c, true), child: const Text('계속')),
        ],
      ),
    );
    if (ok == true && mounted) {
      _set(normalizeRoutes(seeded), commit: true);
      setState(() => _selected = null);
    }
  }

  void _reset() {
    _set(normalizeRoutes(widget.initialRoutes), commit: true);
    setState(() => _selected = null);
  }

  @override
  Widget build(BuildContext context) {
    final open = _openColumns;
    final problemCols = _problemColumns(open);
    final cols = [...open, ...problemCols];
    final problemColSet = problemCols.toSet();
    final sums = outputGainSums(_routes);
    final counts = <int, int>{};
    for (final r in _routes) {
      counts[r.outputChannel] = (counts[r.outputChannel] ?? 0) + 1;
    }
    final rows = _rowCount;
    final fileKnown = widget.fileChannels != null;
    final selected = _selected;
    final selectedCell = selected == null ? null : _cell(selected.$1, selected.$2);

    return Dialog(
      backgroundColor: AppColors.cardSurfaceSolid,
      insetPadding: const EdgeInsets.all(16),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 1100),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      '라우팅 — ${widget.title}'
                      '${fileKnown ? ' (${widget.fileChannels}채널)' : ''}',
                      style: const TextStyle(color: Colors.white, fontSize: 15, fontWeight: FontWeight.bold),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  IconButton(
                    key: const Key('routing_help_button'),
                    tooltip: '사용법',
                    icon: const Icon(Icons.help_outline, color: Colors.white70),
                    onPressed: () => setState(() => _helpOpen = !_helpOpen),
                  ),
                  TextButton(onPressed: _seed, child: const Text('매핑과 같게 채우기')),
                  TextButton(onPressed: _reset, child: const Text('처음 상태로')),
                  TextButton(onPressed: () => Navigator.of(context).maybePop(), child: const Text('닫기')),
                ],
              ),
              if (_helpOpen)
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 150),
                  child: SingleChildScrollView(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: 6),
                      child: Text(trackRoutingHelpText,
                          style: const TextStyle(color: Colors.white70, fontSize: 12, height: 1.4)),
                    ),
                  ),
                ),
              if (!widget.limiterOn)
                const Padding(
                  padding: EdgeInsets.only(bottom: 4),
                  child: Text('피크 리미터가 꺼져 있습니다. 합친 소리가 크면 찌그러질 수 있습니다.',
                      style: TextStyle(color: Colors.orangeAccent, fontSize: 12)),
                ),
              if (!fileKnown)
                const Padding(
                  padding: EdgeInsets.only(bottom: 4),
                  child: Text('파일을 읽지 못해 채널 수를 모릅니다.', style: TextStyle(color: Colors.orangeAccent, fontSize: 12)),
                ),
              if (open.isEmpty)
                const Padding(
                  padding: EdgeInsets.only(bottom: 4),
                  child: Text('열린 출력이 없습니다. 출력 설정(Output Config)에서 출력을 여세요.',
                      style: TextStyle(color: Colors.orangeAccent, fontSize: 12)),
                ),
              Flexible(
                child: ConstrainedBox(
                  constraints: BoxConstraints(maxHeight: _colHeaderH + _cellH * rows + 16),
                  child: Column(
                    children: [
                      Row(
                        children: [
                          const SizedBox(width: _rowHeaderW, height: _colHeaderH),
                          Expanded(
                            child: SingleChildScrollView(
                              controller: _hHeader,
                              scrollDirection: Axis.horizontal,
                              physics: const NeverScrollableScrollPhysics(),
                              child: Row(
                                children: [
                                  for (final o in cols)
                                    _ColumnHeader(
                                      width: _cellW,
                                      height: _colHeaderH,
                                      label: problemColSet.contains(o) ? _problemColumnName(o) : 'Ch-${o + 1}',
                                      hardware: problemColSet.contains(o) ? '' : hardwareChannelName(widget.channelNames, o),
                                      problem: problemColSet.contains(o),
                                      sumWarning: (counts[o] ?? 0) > 1 && (sums[o] ?? 0) > 1.0 + 1e-9
                                          ? '합 ${((sums[o] ?? 0) * 100).round()}%'
                                          : null,
                                    ),
                                ],
                              ),
                            ),
                          ),
                        ],
                      ),
                      Expanded(
                        child: Row(
                          children: [
                            SizedBox(
                              width: _rowHeaderW,
                              child: ListView.builder(
                                controller: _vHeader,
                                physics: const NeverScrollableScrollPhysics(),
                                itemCount: rows,
                                itemExtent: _cellH,
                                itemBuilder: (_, f) {
                                  final missing = fileKnown && f >= widget.fileChannels!;
                                  return Align(
                                    alignment: Alignment.centerLeft,
                                    child: Text(
                                      missing ? '파일 ${f + 1} (없음)' : '파일 ${f + 1}',
                                      style: TextStyle(
                                          color: missing ? Colors.orangeAccent : Colors.white70, fontSize: 12),
                                    ),
                                  );
                                },
                              ),
                            ),
                            Expanded(
                              child: SingleChildScrollView(
                                controller: _hBody,
                                scrollDirection: Axis.horizontal,
                                child: SizedBox(
                                  width: _cellW * cols.length,
                                  child: ListView.builder(
                                    controller: _vBody,
                                    itemCount: rows,
                                    itemExtent: _cellH,
                                    itemBuilder: (_, f) {
                                      final missingRow = fileKnown && f >= widget.fileChannels!;
                                      return Row(
                                        children: [
                                          for (final o in cols)
                                            _Cell(
                                              key: Key('route_cell_${f}_$o'),
                                              width: _cellW,
                                              height: _cellH,
                                              route: _cell(f, o),
                                              problem: missingRow || problemColSet.contains(o),
                                              selected: selected == (f, o),
                                              onTap: () => _tapCell(f, o),
                                            ),
                                        ],
                                      );
                                    },
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
              const Divider(color: Colors.white24),
              _SelectionBar(
                selected: selected,
                cell: selectedCell,
                isLast: _routes.length <= 1,
                onGainChanged: _dragGain,
                onGainEnd: _endGain,
                onDisconnect: _disconnect,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ColumnHeader extends StatelessWidget {
  const _ColumnHeader({
    required this.width,
    required this.height,
    required this.label,
    required this.hardware,
    required this.problem,
    required this.sumWarning,
  });

  final double width;
  final double height;
  final String label;
  final String hardware;
  final bool problem;
  final String? sumWarning;

  @override
  Widget build(BuildContext context) {
    final color = problem ? Colors.orangeAccent : Colors.white;
    return SizedBox(
      width: width,
      height: height,
      child: Tooltip(
        message: [label, if (hardware.isNotEmpty) hardware, if (sumWarning != null) '소리가 더해져 커집니다. 너무 크면 음량을 낮추세요'].join('\n'),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(label, style: TextStyle(color: color, fontSize: 11), overflow: TextOverflow.ellipsis, maxLines: 1),
            if (hardware.isNotEmpty)
              Text(hardware, style: const TextStyle(color: Colors.white54, fontSize: 10), overflow: TextOverflow.ellipsis),
            if (sumWarning != null)
              Text(sumWarning!, style: const TextStyle(color: Colors.orangeAccent, fontSize: 10)),
          ],
        ),
      ),
    );
  }
}

class _Cell extends StatelessWidget {
  const _Cell({
    super.key,
    required this.width,
    required this.height,
    required this.route,
    required this.problem,
    required this.selected,
    required this.onTap,
  });

  final double width;
  final double height;
  final TrackRoute? route;
  final bool problem;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final r = route;
    final on = r != null;
    final fill = !on
        ? Colors.white.withValues(alpha: 0.04)
        : problem
            ? Colors.orange.withValues(alpha: 0.35)
            : AppColors.primaryBlue.withValues(alpha: 0.55);
    return Padding(
      padding: const EdgeInsets.all(2),
      child: InkWell(
        onTap: onTap,
        child: Container(
          width: width - 4,
          height: height - 4,
          decoration: BoxDecoration(
            color: fill,
            border: Border.all(color: selected ? Colors.white : Colors.white12, width: selected ? 1.5 : 1),
            borderRadius: BorderRadius.circular(3),
          ),
          alignment: Alignment.center,
          child: on ? Text('${(r.gain * 100).round()}%', style: const TextStyle(color: Colors.white, fontSize: 11)) : null,
        ),
      ),
    );
  }
}

class _SelectionBar extends StatelessWidget {
  const _SelectionBar({
    required this.selected,
    required this.cell,
    required this.isLast,
    required this.onGainChanged,
    required this.onGainEnd,
    required this.onDisconnect,
  });

  final (int, int)? selected;
  final TrackRoute? cell;
  final bool isLast;
  final ValueChanged<double> onGainChanged;
  final ValueChanged<double> onGainEnd;
  final VoidCallback onDisconnect;

  @override
  Widget build(BuildContext context) {
    final c = cell;
    if (selected == null || c == null) {
      return const SizedBox(
        height: 44,
        child: Align(
          alignment: Alignment.centerLeft,
          child: Text('칸을 누르면 연결하거나 선택합니다.', style: TextStyle(color: Colors.white54, fontSize: 12)),
        ),
      );
    }
    final percent = (c.gain * 100).clamp(1, 100).toDouble();
    return SizedBox(
      height: 44,
      child: Row(
        children: [
          Text('선택: 파일 ${c.fileChannel + 1} → Ch-${c.outputChannel + 1}',
              style: const TextStyle(color: Colors.white, fontSize: 12)),
          const SizedBox(width: 12),
          const Text('음량', style: TextStyle(color: Colors.white70, fontSize: 12)),
          Expanded(
            child: Slider(
              key: const Key('route_gain_slider'),
              min: 1,
              max: 100,
              divisions: 99,
              value: percent,
              label: '${percent.round()}%',
              onChanged: onGainChanged,
              onChangeEnd: onGainEnd,
            ),
          ),
          SizedBox(width: 44, child: Text('${percent.round()}%', style: const TextStyle(color: Colors.white, fontSize: 12))),
          Tooltip(
            message: isLast ? '연결이 하나는 있어야 합니다. 소리를 끄려면 트랙 음량을 내리세요' : '',
            child: TextButton(
              key: const Key('route_disconnect_button'),
              onPressed: isLast ? null : onDisconnect,
              child: const Text('연결 끊기'),
            ),
          ),
          if (isLast)
            const Flexible(
              child: Text('연결이 하나는 있어야 합니다. 소리를 끄려면 트랙 음량을 내리세요',
                  style: TextStyle(color: Colors.white54, fontSize: 11), overflow: TextOverflow.ellipsis),
            ),
        ],
      ),
    );
  }
}
