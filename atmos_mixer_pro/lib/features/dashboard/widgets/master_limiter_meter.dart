import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

/// Master Limiter Gain Reduction (GR) & LED Status Meter Widget
class MasterLimiterMeterWidget extends ConsumerStatefulWidget {
  final double initialGainReductionDb; // 0.0 to -12.0 dB
  final bool enableSimulationToggle;

  const MasterLimiterMeterWidget({
    super.key,
    this.initialGainReductionDb = 0.0,
    this.enableSimulationToggle = true,
  });

  @override
  ConsumerState<MasterLimiterMeterWidget> createState() =>
      _MasterLimiterMeterWidgetState();
}

class _MasterLimiterMeterWidgetState
    extends ConsumerState<MasterLimiterMeterWidget> {
  double _currentGrDb = 0.0;
  bool _isSimulating = false;
  Timer? _simTimer;

  @override
  void initState() {
    super.initState();
    _currentGrDb = widget.initialGainReductionDb;
  }

  @override
  void dispose() {
    _simTimer?.cancel();
    super.dispose();
  }

  void _toggleSimulation() {
    setState(() {
      _isSimulating = !_isSimulating;
      if (_isSimulating) {
        _simTimer = Timer.periodic(const Duration(milliseconds: 150), (t) {
          if (!mounted) return;
          setState(() {
            // Random GR burst between 0.0 and -8.5 dB
            _currentGrDb =
                (t.tick % 5 == 0)
                    ? -((t.tick * 1.7) % 8.5)
                    : (_currentGrDb * 0.7);
            if (_currentGrDb.abs() < 0.1) _currentGrDb = 0.0;
          });
        });
      } else {
        _simTimer?.cancel();
        _simTimer = null;
        _currentGrDb = 0.0;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    // 백엔드 리미터가 실제로 적용한 게인 리덕션(dB)을 그대로 표시한다.
    //
    // 예전에는 shortTermLufs를 받아 "-((lufs + 12) * 1.5)"로 **추정**했다
    // (원 주석에도 Rough approximation). 그래서 리미터가 전혀 동작하지 않아도
    // 라우드니스가 높으면 게인 리덕션이 걸린 것처럼 보였고, 반대로 리미터가
    // 실제로 물려도 LUFS가 낮으면 0으로 표시됐다. Rust는 이미
    // EngineStateUpdate.gain_reduction_db로 실측값을 보내고 있었고
    // Dart EngineState에만 그 필드가 없었다.
    //
    // Rust는 리덕션 양을 0 이상의 양수 dB로 보내고 이 위젯은 음수(0 ~ -12dB)
    // 규약을 쓰므로 부호를 맞춘다.
    final double engineGrDb =
        ref.watch(engineStateProvider.select((s) => s.gainReductionDb));

    if (!_isSimulating) {
      _currentGrDb = -engineGrDb.abs();
    }

    final bool isCompressing = _currentGrDb.abs() > 0.1;
    final double grRatio = (_currentGrDb.abs() / 12.0).clamp(0.0, 1.0);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: AppColors.cardSurface,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(
          color: isCompressing ? Colors.amber.shade700 : Colors.white12,
          width: 1,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Limiter LED Indicator
          AnimatedContainer(
            duration: const Duration(milliseconds: 100),
            width: 10,
            height: 10,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: isCompressing ? Colors.amberAccent : Colors.grey.shade700,
              boxShadow: isCompressing
                  ? [
                      BoxShadow(
                        color: Colors.amberAccent.withValues(alpha: 0.8),
                        blurRadius: 6,
                        spreadRadius: 2,
                      ),
                    ]
                  : [],
            ),
          ),
          const SizedBox(width: 8),
          // Label
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'TRUE-PEAK LIMITER',
                    style: TextStyle(
                      color: isCompressing ? Colors.amberAccent : Colors.white70,
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 0.8,
                    ),
                  ),
                  if (isCompressing) ...[
                    const SizedBox(width: 4),
                    Text(
                      'ISP -${_currentGrDb.abs().toStringAsFixed(1)} dB',
                      style: const TextStyle(
                        color: Colors.amberAccent,
                        fontSize: 9,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ],
              ),
              const SizedBox(height: 3),
              // Gain Reduction Meter Bar (Horizontal)
              SizedBox(
                width: 70,
                height: 5,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: Stack(
                    children: [
                      // Background Track
                      Container(color: Colors.black45),
                      // Active GR fill (right to left or left to right)
                      FractionallySizedBox(
                        widthFactor: grRatio,
                        child: Container(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              colors: [
                                Colors.amber.shade400,
                                Colors.redAccent,
                              ],
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          if (widget.enableSimulationToggle) ...[
            const SizedBox(width: 6),
            Tooltip(
              message: _isSimulating ? 'Stop Test' : 'Simulate GR Test',
              child: InkWell(
                onTap: _toggleSimulation,
                borderRadius: BorderRadius.circular(4),
                child: Padding(
                  padding: const EdgeInsets.all(2.0),
                  child: Icon(
                    _isSimulating ? Icons.science : Icons.science_outlined,
                    size: 14,
                    color:
                        _isSimulating ? AppColors.primaryNeon : Colors.white38,
                  ),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
