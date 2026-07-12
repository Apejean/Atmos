import 'dart:math' as math;
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../../core/theme/colors.dart';
import '../../../src/rust/api/simple.dart';
import '../../../src/rust/common/config.dart';

class TuningModal extends ConsumerStatefulWidget {
  const TuningModal({super.key});

  @override
  ConsumerState<TuningModal> createState() => _TuningModalState();
}

class _TuningModalState extends ConsumerState<TuningModal> {
  int _selectedChannel = 1;
  final TextEditingController _delayController = TextEditingController(text: '0.0');
  
  int _activeBandIndex = 0;
  int _hoverIndex = -1;
  bool _isDragging = false;
  
  // 8 bands state
  final List<bool> _bandEnabled = List.filled(8, false);
  final List<EqType> _bandTypes = List.filled(8, EqType.bell);
  final List<TextEditingController> _freqControllers = [];
  final List<TextEditingController> _gainControllers = [];
  final List<TextEditingController> _qControllers = [];

  Timer? _throttleTimer;

  static const double minFreq = 20.0;
  static const double maxFreq = 20000.0;
  static const double minGain = -15.0;
  static const double maxGain = 15.0;

  @override
  void initState() {
    super.initState();
    for (int i = 0; i < 8; i++) {
      _freqControllers.add(TextEditingController(text: '${math.min(100 * math.pow(2, i), maxFreq).toInt()}'));
      _gainControllers.add(TextEditingController(text: '0.0'));
      _qControllers.add(TextEditingController(text: '0.707'));
    }
    // Listen to changes in controllers to trigger repaint
    for (int i = 0; i < 8; i++) {
      _freqControllers[i].addListener(() => setState(() {}));
      _gainControllers[i].addListener(() => setState(() {}));
      _qControllers[i].addListener(() => setState(() {}));
    }
  }

  @override
  void dispose() {
    _throttleTimer?.cancel();
    _delayController.dispose();
    for (var c in _freqControllers) {
      c.dispose();
    }
    for (var c in _gainControllers) {
      c.dispose();
    }
    for (var c in _qControllers) {
      c.dispose();
    }
    super.dispose();
  }

  void _sendThrottledUpdate() {
    if (_throttleTimer?.isActive ?? false) return;
    _throttleTimer = Timer(const Duration(milliseconds: 50), () {
      _applyTuning(silent: true);
    });
  }

  void _applyTuning({bool silent = false}) {
    try {
      final double delay = double.tryParse(_delayController.text) ?? 0.0;
      final List<EqBand> bands = [];
      for (int i = 0; i < 8; i++) {
        final double freq = double.tryParse(_freqControllers[i].text) ?? 1000.0;
        final double gain = double.tryParse(_gainControllers[i].text) ?? 0.0;
        final double q = double.tryParse(_qControllers[i].text) ?? 0.707;
        bands.add(EqBand(
          enabled: _bandEnabled[i],
          freq: freq,
          gain: gain,
          qFactor: q,
          filterType: _bandTypes[i],
        ));
      }
      
      apiApplyChannelTuning(channel: _selectedChannel, delayMs: delay, eqBands: bands);
      
      if (!silent && mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('튜닝 설정이 적용되었습니다.'),
            backgroundColor: AppColors.success,
            duration: Duration(seconds: 2),
          ),
        );
        Navigator.of(context).pop();
      }
    } catch (e) {
      if (!silent && mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('잘못된 입력값입니다. 숫자를 입력해주세요. ($e)'),
            backgroundColor: AppColors.danger,
          ),
        );
      }
    }
  }

  // Coordinate mapping
  double _freqToX(double freq, double width) {
    freq = freq.clamp(minFreq, maxFreq);
    final minLog = math.log(minFreq);
    final maxLog = math.log(maxFreq);
    return ((math.log(freq) - minLog) / (maxLog - minLog)) * width;
  }

  double _xToFreq(double x, double width) {
    final t = (x / width).clamp(0.0, 1.0);
    final minLog = math.log(minFreq);
    final maxLog = math.log(maxFreq);
    return math.exp(minLog + t * (maxLog - minLog));
  }

  double _gainToY(double gain, double height) {
    gain = gain.clamp(minGain, maxGain);
    return height - ((gain - minGain) / (maxGain - minGain)) * height;
  }

  double _yToGain(double y, double height) {
    final t = 1.0 - (y / height).clamp(0.0, 1.0);
    return minGain + t * (maxGain - minGain);
  }

  int _findClosestBandIndex(Offset localPosition, Size size) {
    double bestDist = double.infinity;
    int bestIndex = -1;
    
    for (int i = 0; i < 8; i++) {
      if (!_bandEnabled[i]) continue;
      double freq = double.tryParse(_freqControllers[i].text) ?? 1000.0;
      double gain = double.tryParse(_gainControllers[i].text) ?? 0.0;
      
      double px = _freqToX(freq, size.width);
      double py = _gainToY(gain, size.height);
      
      double dist = math.sqrt(math.pow(px - localPosition.dx, 2) + math.pow(py - localPosition.dy, 2));
      if (dist < 30.0 && dist < bestDist) {
        bestDist = dist;
        bestIndex = i;
      }
    }
    return bestIndex;
  }

  void _handleHover(PointerEvent event, BoxConstraints constraints) {
    if (_isDragging) return;
    int closest = _findClosestBandIndex(event.localPosition, Size(constraints.maxWidth, constraints.maxHeight));
    if (_hoverIndex != closest) {
      setState(() {
        _hoverIndex = closest;
      });
    }
  }

  void _handlePanDown(DragDownDetails details, BoxConstraints constraints) {
    int bestIndex = _findClosestBandIndex(details.localPosition, Size(constraints.maxWidth, constraints.maxHeight));
    if (bestIndex != -1) {
      setState(() {
        _activeBandIndex = bestIndex;
        _isDragging = true;
      });
    }
  }

  void _handlePanUpdate(DragUpdateDetails details, BoxConstraints constraints) {
    if (!_isDragging || !_bandEnabled[_activeBandIndex]) return;
    
    double newFreq = _xToFreq(details.localPosition.dx, constraints.maxWidth);
    double newGain = _yToGain(details.localPosition.dy, constraints.maxHeight);
    
    _freqControllers[_activeBandIndex].text = newFreq.toStringAsFixed(1);
    _gainControllers[_activeBandIndex].text = newGain.toStringAsFixed(1);
    
    _sendThrottledUpdate();
  }

  void _handlePanEnd(DragEndDetails details) {
    setState(() {
      _isDragging = false;
    });
    _applyTuning(silent: true);
  }

  // --- UI Components ---

  Widget _buildBandToggle(int index) {
    final bool isActive = _activeBandIndex == index;
    final bool isEnabled = _bandEnabled[index];
    
    return GestureDetector(
      onTap: () {
        setState(() {
          _activeBandIndex = index;
        });
      },
      child: Container(
        width: 44,
        height: 44,
        margin: const EdgeInsets.only(right: 8),
        decoration: BoxDecoration(
          color: isActive ? AppColors.primaryNeon.withValues(alpha: 0.15) : AppColors.background,
          border: Border.all(
            color: isActive ? AppColors.primaryNeon : Colors.white24,
            width: 1,
          ),
          borderRadius: BorderRadius.circular(4),
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(
              '${index + 1}',
              style: TextStyle(
                color: isActive ? AppColors.primaryNeon : AppColors.textPrimary,
                fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
              ),
            ),
            const SizedBox(height: 2),
            GestureDetector(
              onTap: () {
                setState(() {
                  _bandEnabled[index] = !isEnabled;
                  _activeBandIndex = index;
                  _sendThrottledUpdate();
                });
              },
              child: Container(
                width: 14,
                height: 14,
                decoration: BoxDecoration(
                  color: isEnabled ? AppColors.primaryNeon : Colors.transparent,
                  border: Border.all(color: AppColors.primaryNeon),
                  shape: BoxShape.circle,
                ),
                child: isEnabled 
                  ? const Icon(Icons.check, size: 10, color: AppColors.background)
                  : null,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildActiveBandControls() {
    final int idx = _activeBandIndex;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: AppColors.background,
        border: Border.all(color: Colors.white12, width: 1),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            flex: 3,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Type', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                const SizedBox(height: 8),
                InputDecorator(
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  ),
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<EqType>(
                      value: _bandTypes[idx],
                      dropdownColor: AppColors.cardSurface,
                      isDense: true,
                      isExpanded: true,
                      style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                      items: EqType.values.map((type) {
                        return DropdownMenuItem(
                          value: type,
                          child: Text(type.name),
                        );
                      }).toList(),
                      onChanged: (val) {
                        if (val != null) {
                          setState(() {
                            _bandTypes[idx] = val;
                            _sendThrottledUpdate();
                          });
                        }
                      },
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            flex: 2,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Freq (Hz)', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                const SizedBox(height: 8),
                TextField(
                  controller: _freqControllers[idx],
                  style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  ),
                  inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*'))],
                  onChanged: (v) => _sendThrottledUpdate(),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            flex: 2,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Gain (dB)', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                const SizedBox(height: 8),
                TextField(
                  controller: _gainControllers[idx],
                  style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                  keyboardType: const TextInputType.numberWithOptions(decimal: true, signed: true),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  ),
                  inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'^-?\d*\.?\d*'))],
                  onChanged: (v) => _sendThrottledUpdate(),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            flex: 2,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Q', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                const SizedBox(height: 8),
                TextField(
                  controller: _qControllers[idx],
                  style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  ),
                  inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*'))],
                  onChanged: (v) => _sendThrottledUpdate(),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildInteractiveEqCurve() {
    List<double> freqs = [];
    List<double> gains = [];
    List<double> qs = [];
    for (int i = 0; i < 8; i++) {
      freqs.add(double.tryParse(_freqControllers[i].text) ?? 1000.0);
      gains.add(double.tryParse(_gainControllers[i].text) ?? 0.0);
      qs.add(double.tryParse(_qControllers[i].text) ?? 0.707);
    }

    return Container(
      height: 220,
      width: double.infinity,
      decoration: BoxDecoration(
        color: AppColors.cardSurface,
        border: Border.all(color: Colors.white12, width: 1),
        borderRadius: BorderRadius.circular(4),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(4),
        child: LayoutBuilder(
          builder: (context, constraints) {
            return MouseRegion(
              onHover: (event) => _handleHover(event, constraints),
              onExit: (_) => setState(() => _hoverIndex = -1),
              child: GestureDetector(
                onPanDown: (details) => _handlePanDown(details, constraints),
                onPanUpdate: (details) => _handlePanUpdate(details, constraints),
                onPanEnd: _handlePanEnd,
                onPanCancel: () {
                  setState(() => _isDragging = false);
                  _applyTuning(silent: true);
                },
                child: Stack(
                  children: [
                    // Static Background Grid protected by RepaintBoundary
                    RepaintBoundary(
                      child: CustomPaint(
                        painter: _EqGridPainter(
                          minFreq: minFreq, maxFreq: maxFreq,
                          minGain: minGain, maxGain: maxGain,
                        ),
                        size: Size(constraints.maxWidth, constraints.maxHeight),
                      ),
                    ),
                    // Dynamic Curve and Dots
                    CustomPaint(
                      painter: _EqCurvePainter(
                        activeBandIndex: _activeBandIndex,
                        hoverIndex: _hoverIndex,
                        isDragging: _isDragging,
                        bandEnabled: _bandEnabled,
                        bandTypes: _bandTypes,
                        freqs: freqs,
                        gains: gains,
                        qs: qs,
                        minFreq: minFreq,
                        maxFreq: maxFreq,
                        minGain: minGain,
                        maxGain: maxGain,
                      ),
                      size: Size(constraints.maxWidth, constraints.maxHeight),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: AppColors.cardSurfaceSolid,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(4),
        side: BorderSide(color: Colors.white.withValues(alpha: 0.08), width: 1),
      ),
      child: Container(
        width: 650,
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '출력 채널 튜닝 (Delay & EQ Eight)',
              style: TextStyle(
                color: AppColors.textPrimary,
                fontSize: 20,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(height: 16),
            
            // Channel & Delay Row
            Row(
              children: [
                const SizedBox(
                  width: 60,
                  child: Text('채널', style: TextStyle(color: AppColors.textSecondary)),
                ),
                Expanded(
                  flex: 2,
                  child: InputDecorator(
                    decoration: const InputDecoration(
                      filled: true,
                      fillColor: AppColors.background,
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                    ),
                    child: DropdownButtonHideUnderline(
                      child: DropdownButton<int>(
                        value: _selectedChannel,
                        dropdownColor: AppColors.cardSurface,
                        isDense: true,
                        isExpanded: true,
                        style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                        items: List.generate(24, (index) => index + 1).map((ch) {
                          return DropdownMenuItem<int>(
                            value: ch,
                            child: Text('Channel $ch'),
                          );
                        }).toList(),
                        onChanged: (val) {
                          if (val != null) setState(() => _selectedChannel = val);
                        },
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 24),
                const SizedBox(
                  width: 80,
                  child: Text('Delay (ms)', style: TextStyle(color: AppColors.textSecondary)),
                ),
                Expanded(
                  flex: 2,
                  child: TextField(
                    controller: _delayController,
                    style: const TextStyle(color: AppColors.textPrimary, fontSize: 14),
                    keyboardType: const TextInputType.numberWithOptions(decimal: true),
                    decoration: const InputDecoration(
                      filled: true,
                      fillColor: AppColors.background,
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                    ),
                    inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*'))],
                    onChanged: (v) => _sendThrottledUpdate(),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            
            // Interactive EQ Curve
            _buildInteractiveEqCurve(),

            const SizedBox(height: 16),

            // 8 Bands Toggles
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: List.generate(8, (i) => _buildBandToggle(i)),
              ),
            ),
            
            const SizedBox(height: 12),
            
            // Active Band Controls
            _buildActiveBandControls(),

            const SizedBox(height: 24),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('닫기', style: TextStyle(color: AppColors.textSecondary)),
                ),
                const SizedBox(width: 12),
                ElevatedButton(
                  style: ElevatedButton.styleFrom(
                    backgroundColor: AppColors.primaryNeon,
                    foregroundColor: AppColors.background,
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
                    padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
                  ),
                  onPressed: () => _applyTuning(silent: false),
                  child: const Text('적용 (Apply)', style: TextStyle(fontWeight: FontWeight.bold)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// --------------------------------------------------------------------------
// Painters
// --------------------------------------------------------------------------

class _EqGridPainter extends CustomPainter {
  final double minFreq;
  final double maxFreq;
  final double minGain;
  final double maxGain;

  _EqGridPainter({
    required this.minFreq,
    required this.maxFreq,
    required this.minGain,
    required this.maxGain,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final minLog = math.log(minFreq);
    final maxLog = math.log(maxFreq);

    double freqToX(double f) {
      f = f.clamp(minFreq, maxFreq);
      return ((math.log(f) - minLog) / (maxLog - minLog)) * size.width;
    }
    double gainToY(double g) {
      g = g.clamp(minGain, maxGain);
      return size.height - ((g - minGain) / (maxGain - minGain)) * size.height;
    }

    final gridPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.05)
      ..strokeWidth = 1;
    final zeroLinePaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.2)
      ..strokeWidth = 1;

    // Horizontal lines (Gain)
    for (double g in [-12.0, -6.0, 0.0, 6.0, 12.0]) {
      double y = gainToY(g);
      canvas.drawLine(Offset(0, y), Offset(size.width, y), g == 0.0 ? zeroLinePaint : gridPaint);
    }

    // Vertical lines (Freq)
    for (double f in [100.0, 1000.0, 10000.0]) {
      double x = freqToX(f);
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), gridPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _EqGridPainter oldDelegate) => false;
}

class _EqCurvePainter extends CustomPainter {
  final int activeBandIndex;
  final int hoverIndex;
  final bool isDragging;
  final List<bool> bandEnabled;
  final List<EqType> bandTypes;
  final List<double> freqs;
  final List<double> gains;
  final List<double> qs;
  final double minFreq;
  final double maxFreq;
  final double minGain;
  final double maxGain;

  _EqCurvePainter({
    required this.activeBandIndex,
    required this.hoverIndex,
    required this.isDragging,
    required this.bandEnabled,
    required this.bandTypes,
    required this.freqs,
    required this.gains,
    required this.qs,
    required this.minFreq,
    required this.maxFreq,
    required this.minGain,
    required this.maxGain,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final minLog = math.log(minFreq);
    final maxLog = math.log(maxFreq);

    double freqToX(double f) {
      f = f.clamp(minFreq, maxFreq);
      return ((math.log(f) - minLog) / (maxLog - minLog)) * size.width;
    }
    double gainToY(double g) {
      g = g.clamp(minGain, maxGain);
      return size.height - ((g - minGain) / (maxGain - minGain)) * size.height;
    }

    // Calculate curve points
    int resolution = 100;
    List<Offset> points = [];
    
    for (int i = 0; i <= resolution; i++) {
      double x = (i / resolution) * size.width;
      double t = i / resolution;
      double f = math.exp(minLog + t * (maxLog - minLog));

      double totalDb = 0.0;
      for (int b = 0; b < 8; b++) {
        if (!bandEnabled[b]) continue;
        double bandF = freqs[b].clamp(minFreq, maxFreq);
        double bandG = gains[b];
        double bandQ = qs[b].clamp(0.1, 10.0);
        double distanceOctaves = (math.log(f) - math.log(bandF)) / math.ln2;
        
        switch (bandTypes[b]) {
          case EqType.bell:
            double width = 1.0 / bandQ;
            totalDb += bandG * math.exp(-(distanceOctaves * distanceOctaves) / (width * width));
            break;
          case EqType.lowShelf:
            double width = 1.0 / bandQ;
            totalDb += bandG / (1.0 + math.exp(distanceOctaves * 4.0 / width));
            break;
          case EqType.highShelf:
            double width = 1.0 / bandQ;
            totalDb += bandG / (1.0 + math.exp(-distanceOctaves * 4.0 / width));
            break;
          case EqType.lowCut:
            if (f < bandF) {
               totalDb -= (math.log(bandF) - math.log(f)) * 10.0 * bandQ;
            }
            break;
          case EqType.highCut:
            if (f > bandF) {
               totalDb -= (math.log(f) - math.log(bandF)) * 10.0 * bandQ;
            }
            break;
          case EqType.notch:
            double width = 0.5 / bandQ;
            if (distanceOctaves.abs() < width) {
                totalDb -= 15.0 * (1.0 - (distanceOctaves.abs() / width));
            }
            break;
        }
      }
      points.add(Offset(x, gainToY(totalDb)));
    }

    if (points.isNotEmpty) {
      Path curvePath = Path();
      curvePath.moveTo(points.first.dx, points.first.dy);

      // Bezier Smoothing
      for (int i = 0; i < points.length - 1; i++) {
        final p0 = points[i];
        final p1 = points[i + 1];
        final mid = Offset((p0.dx + p1.dx) / 2, (p0.dy + p1.dy) / 2);
        curvePath.quadraticBezierTo(p0.dx, p0.dy, mid.dx, mid.dy);
        if (i == points.length - 2) {
          curvePath.lineTo(p1.dx, p1.dy);
        }
      }

      // Fill Path
      Path fillPath = Path.from(curvePath);
      fillPath.lineTo(size.width, gainToY(0));
      fillPath.lineTo(0, gainToY(0));
      fillPath.close();

      // Neon Gradient Fill
      final fillPaint = Paint()
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [
            AppColors.primaryNeon.withValues(alpha: 0.35),
            AppColors.primaryNeon.withValues(alpha: 0.0),
          ],
          stops: const [0.0, 1.0],
        ).createShader(Rect.fromLTRB(0, 0, size.width, size.height));
      
      canvas.drawPath(fillPath, fillPaint);

      final curvePaint = Paint()
        ..color = AppColors.primaryNeon
        ..strokeWidth = 2.5
        ..style = PaintingStyle.stroke
        ..isAntiAlias = true;
      
      canvas.drawPath(curvePath, curvePaint);
    }

    // Draw Dots and Tooltips
    for (int b = 0; b < 8; b++) {
      if (!bandEnabled[b]) continue;
      
      bool isActive = b == activeBandIndex;
      bool isHovered = b == hoverIndex;
      double px = freqToX(freqs[b]);
      double py = gainToY(gains[b]);

      final dotPaint = Paint()
        ..color = isActive || isHovered ? AppColors.primaryNeon : AppColors.cardSurfaceSolid
        ..style = PaintingStyle.fill;
      
      final borderPaint = Paint()
        ..color = isActive || isHovered ? Colors.white : AppColors.primaryNeon
        ..strokeWidth = isActive || isHovered ? 2.5 : 1.5
        ..style = PaintingStyle.stroke;

      canvas.drawCircle(Offset(px, py), isActive ? 7.0 : 5.0, dotPaint);
      canvas.drawCircle(Offset(px, py), isActive ? 7.0 : 5.0, borderPaint);

      // Draw Tooltip for Active/Hovered dot
      if ((isDragging && isActive) || (!isDragging && isHovered)) {
        _drawTooltip(canvas, px, py, b, size);
      } else {
        // Draw just the number if not tooltip
        final textPainter = TextPainter(
          text: TextSpan(
            text: '${b + 1}',
            style: TextStyle(
              color: isActive ? Colors.black : Colors.white,
              fontSize: 9,
              fontWeight: FontWeight.bold,
            ),
          ),
          textDirection: TextDirection.ltr,
        );
        textPainter.layout();
        textPainter.paint(
          canvas, 
          Offset(px - textPainter.width / 2, py - textPainter.height / 2 - 16)
        );
      }
    }
  }

  void _drawTooltip(Canvas canvas, double px, double py, int bandIndex, Size size) {
    final String text = '${freqs[bandIndex].toStringAsFixed(1)} Hz\n${gains[bandIndex].toStringAsFixed(1)} dB';
    
    final textPainter = TextPainter(
      text: TextSpan(
        text: text,
        style: const TextStyle(
          color: Colors.white,
          fontSize: 11,
          fontWeight: FontWeight.w500,
          height: 1.3,
        ),
      ),
      textAlign: TextAlign.center,
      textDirection: TextDirection.ltr,
    );
    textPainter.layout();

    final padding = const EdgeInsets.symmetric(horizontal: 10, vertical: 6);
    final width = textPainter.width + padding.horizontal;
    final height = textPainter.height + padding.vertical;

    // Tooltip position (above the dot)
    double tx = px - width / 2;
    double ty = py - height - 12;

    // Clamp to screen
    if (tx < 0) tx = 0;
    if (tx + width > size.width) tx = size.width - width;
    if (ty < 0) ty = py + 12; // Draw below if no space above

    final rect = RRect.fromRectAndRadius(
      Rect.fromLTWH(tx, ty, width, height),
      const Radius.circular(4),
    );

    // Shadow
    canvas.drawRRect(
      rect.shift(const Offset(0, 2)), 
      Paint()
        ..color = Colors.black.withValues(alpha: 0.5)
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 4)
    );

    // Box
    canvas.drawRRect(
      rect,
      Paint()..color = const Color(0xFF2D2D2D)
    );
    canvas.drawRRect(
      rect,
      Paint()
        ..color = Colors.white.withValues(alpha: 0.1)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1
    );

    textPainter.paint(canvas, Offset(tx + padding.left, ty + padding.top));
  }

  @override
  bool shouldRepaint(covariant _EqCurvePainter oldDelegate) => true;
}
