import 'dart:math' as math;
import 'dart:async';
import 'dart:convert';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../../../core/theme/colors.dart';
import '../../../core/state/global_state.dart';
import '../../../src/rust/api/simple.dart';
import '../../../src/rust/common/config.dart';

class ChannelTuningState {
  final double delay;
  final List<bool> bandEnabled;
  final List<EqType> bandTypes;
  final List<double> freqs;
  final List<double> gains;
  final List<double> qs;
  final bool isStereoLinked;

  ChannelTuningState({
    required this.delay,
    required this.bandEnabled,
    required this.bandTypes,
    required this.freqs,
    required this.gains,
    required this.qs,
    required this.isStereoLinked,
  });

  factory ChannelTuningState.initial() {
    return ChannelTuningState(
      delay: 0.0,
      bandEnabled: List.filled(8, false),
      bandTypes: List.filled(8, EqType.bell),
      freqs: List.generate(
        8,
        (i) => math.min(100 * math.pow(2, i), 20000.0).toDouble(),
      ),
      gains: List.filled(8, 0.0),
      qs: List.filled(8, 0.707),
      isStereoLinked: true,
    );
  }

  ChannelTuningState copyWith({
    double? delay,
    List<bool>? bandEnabled,
    List<EqType>? bandTypes,
    List<double>? freqs,
    List<double>? gains,
    List<double>? qs,
    bool? isStereoLinked,
  }) {
    return ChannelTuningState(
      delay: delay ?? this.delay,
      bandEnabled: bandEnabled ?? this.bandEnabled,
      bandTypes: bandTypes ?? this.bandTypes,
      freqs: freqs ?? this.freqs,
      gains: gains ?? this.gains,
      qs: qs ?? this.qs,
      isStereoLinked: isStereoLinked ?? this.isStereoLinked,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'delay': delay,
      'bandEnabled': bandEnabled,
      'bandTypes': bandTypes.map((e) => e.index).toList(),
      'freqs': freqs,
      'gains': gains,
      'qs': qs,
      'isStereoLinked': isStereoLinked,
    };
  }

  factory ChannelTuningState.fromJson(Map<String, dynamic> json) {
    return ChannelTuningState(
      delay: (json['delay'] as num).toDouble(),
      bandEnabled: (json['bandEnabled'] as List).cast<bool>(),
      bandTypes: (json['bandTypes'] as List)
          .map((e) => EqType.values[e as int])
          .toList(),
      freqs: (json['freqs'] as List).map((e) => (e as num).toDouble()).toList(),
      gains: (json['gains'] as List).map((e) => (e as num).toDouble()).toList(),
      qs: (json['qs'] as List).map((e) => (e as num).toDouble()).toList(),
      isStereoLinked: json['isStereoLinked'] as bool,
    );
  }
}

class TuningStateNotifier extends Notifier<Map<int, ChannelTuningState>>
    with WidgetsBindingObserver {
  bool _isLoaded = false;
  Timer? _saveTimer;

  @override
  Map<int, ChannelTuningState> build() {
    WidgetsBinding.instance.addObserver(this);
    ref.onDispose(() {
      WidgetsBinding.instance.removeObserver(this);
      _saveTimer?.cancel();
    });
    Future.microtask(ensureLoaded);
    return {};
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.inactive) {
      if (_saveTimer != null && _saveTimer!.isActive) {
        _saveTimer!.cancel();
        _saveToPrefs();
      }
    }
  }

  Future<void> ensureLoaded() async {
    if (_isLoaded) return;
    await _load();
    _isLoaded = true;
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString('tuning_state');
    if (jsonString != null) {
      try {
        final decoded = jsonDecode(jsonString) as Map<String, dynamic>;
        final Map<int, ChannelTuningState> loadedState = {};
        for (final entry in decoded.entries) {
          loadedState[int.parse(entry.key)] = ChannelTuningState.fromJson(
            entry.value as Map<String, dynamic>,
          );
        }
        state = loadedState;

        applyAllToBackend();
      } catch (e) {
        debugPrint('Failed to load tuning state: $e');
      }
    }
  }

  void applyAllToBackend() {
    final tuningsToApply = <ChannelTuningParams>[];
    for (final entry in state.entries) {
      final channelKey = entry.key;
      final targetChannel = channelKey - 1;
      final tuning = entry.value;
      final bands = <EqBand>[];
      for (int i = 0; i < 8; i++) {
        bands.add(
          EqBand(
            enabled: tuning.bandEnabled[i],
            filterType: tuning.bandTypes[i],
            freq: tuning.freqs[i],
            gain: tuning.gains[i],
            qFactor: tuning.qs[i],
          ),
        );
      }
      tuningsToApply.add(
        ChannelTuningParams(
          channel: targetChannel,
          delayMs: tuning.delay,
          eqBands: bands,
        ),
      );
    }
    apiApplyAllChannelTunings(tunings: tuningsToApply);
  }

  void syncFromBackendConfig(AppConfig config) {
    final Map<int, ChannelTuningState> newState = {};

    void processConfigs(Map<int, ChannelSetting> configs, bool isStereo) {
      for (final entry in configs.entries) {
        final chKey = entry.key;
        final setting = entry.value;

        final bandEnabled = List.filled(8, false);
        final bandTypes = List.filled(8, EqType.bell);
        final freqs = List.filled(8, 1000.0);
        final gains = List.filled(8, 0.0);
        final qs = List.filled(8, 0.707);

        for (int i = 0; i < setting.eqBands.length && i < 8; i++) {
          final band = setting.eqBands[i];
          bandEnabled[i] = band.enabled;
          bandTypes[i] = band.filterType;
          freqs[i] = band.freq;
          gains[i] = band.gain;
          qs[i] = band.qFactor;
        }

        newState[chKey] = ChannelTuningState(
          delay: setting.delayMs,
          bandEnabled: bandEnabled,
          bandTypes: bandTypes,
          freqs: freqs,
          gains: gains,
          qs: qs,
          isStereoLinked: isStereo,
        );
      }
    }

    processConfigs(config.monoConfigs, false);
    processConfigs(config.stereoConfigs, true);
    processConfigs(config.multiConfigs, false);

    state = newState;
    _saveToPrefs();
  }

  ChannelTuningState getTuning(int channel) {
    return state[channel] ?? ChannelTuningState.initial();
  }

  void saveTuning(int channel, ChannelTuningState tuning) {
    state = {...state, channel: tuning};
    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 500), _saveToPrefs);
  }

  Future<void> _saveToPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final mapToSave = state.map(
      (key, value) => MapEntry(key.toString(), value.toJson()),
    );
    await prefs.setString('tuning_state', jsonEncode(mapToSave));
  }
}

final tuningStateProvider =
    NotifierProvider<TuningStateNotifier, Map<int, ChannelTuningState>>(
      TuningStateNotifier.new,
    );

class TuningModal extends ConsumerStatefulWidget {
  const TuningModal({super.key});

  @override
  ConsumerState<TuningModal> createState() => _TuningModalState();
}

class _TuningModalState extends ConsumerState<TuningModal> {
  int _selectedChannel = 1;
  bool _isStereoLinked = true;
  final TextEditingController _delayController = TextEditingController(
    text: '0.0',
  );

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
  static const double minGain = -30.0;
  static const double maxGain = 30.0;

  @override
  void initState() {
    super.initState();
    for (int i = 0; i < 8; i++) {
      _freqControllers.add(TextEditingController());
      _gainControllers.add(TextEditingController());
      _qControllers.add(TextEditingController());
    }

    // We delay the load so we don't modify provider state during init.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadStateForChannel(_selectedChannel);
    });
  }

  void _loadStateForChannel(int channel) {
    final tuning = ref.read(tuningStateProvider.notifier).getTuning(channel);
    _delayController.text = tuning.delay.toString();
    _isStereoLinked = tuning.isStereoLinked;
    _bandEnabled.setAll(0, tuning.bandEnabled);
    _bandTypes.setAll(0, tuning.bandTypes);

    for (int i = 0; i < 8; i++) {
      _freqControllers[i].text = tuning.freqs[i].toStringAsFixed(1);
      _gainControllers[i].text = tuning.gains[i].toStringAsFixed(1);
      _qControllers[i].text = tuning.qs[i].toStringAsFixed(3);
    }
    setState(() {});
  }

  void _saveCurrentState() {
    final tuning = ChannelTuningState(
      delay: double.tryParse(_delayController.text) ?? 0.0,
      bandEnabled: List.from(_bandEnabled),
      bandTypes: List.from(_bandTypes),
      freqs: _freqControllers
          .map((c) => double.tryParse(c.text) ?? 1000.0)
          .toList(),
      gains: _gainControllers
          .map((c) => double.tryParse(c.text) ?? 0.0)
          .toList(),
      qs: _qControllers.map((c) => double.tryParse(c.text) ?? 0.707).toList(),
      isStereoLinked: _isStereoLinked,
    );
    ref.read(tuningStateProvider.notifier).saveTuning(_selectedChannel, tuning);

    // Sync partner channel's state if stereo link is involved
    int partnerChannel = _selectedChannel % 2 != 0
        ? _selectedChannel + 1
        : _selectedChannel - 1;

    if (_isStereoLinked) {
      // If linked, partner channel should have the exact same tuning state.
      ref.read(tuningStateProvider.notifier).saveTuning(partnerChannel, tuning);
    } else {
      // If unlinked, just ensure the partner channel also unlinks its UI state without changing its eq values.
      final partnerTuning = ref
          .read(tuningStateProvider.notifier)
          .getTuning(partnerChannel);
      if (partnerTuning.isStereoLinked) {
        ref
            .read(tuningStateProvider.notifier)
            .saveTuning(
              partnerChannel,
              partnerTuning.copyWith(isStereoLinked: false),
            );
      }
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
      _saveCurrentState(); // Save to Riverpod so it persists

      final double delay = double.tryParse(_delayController.text) ?? 0.0;
      final List<EqBand> bands = [];
      for (int i = 0; i < 8; i++) {
        final double freq = double.tryParse(_freqControllers[i].text) ?? 1000.0;
        final double gain = double.tryParse(_gainControllers[i].text) ?? 0.0;
        final double q = double.tryParse(_qControllers[i].text) ?? 0.707;
        bands.add(
          EqBand(
            enabled: _bandEnabled[i],
            freq: freq,
            gain: gain,
            qFactor: q,
            filterType: _bandTypes[i],
          ),
        );
      }

      int targetChannel1 = _selectedChannel - 1;
      final tunings = <ChannelTuningParams>[
        ChannelTuningParams(
          channel: targetChannel1,
          delayMs: delay,
          eqBands: bands,
        ),
      ];

      if (_isStereoLinked) {
        int targetChannel2 = _selectedChannel % 2 != 0
            ? targetChannel1 + 1
            : targetChannel1 - 1;
        tunings.add(
          ChannelTuningParams(
            channel: targetChannel2,
            delayMs: delay,
            eqBands: bands,
          ),
        );
      }
      apiApplyAllChannelTunings(tunings: tunings);

      if (!silent && mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Mixer 설정이 적용되었습니다.'),
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

      double dist = math.sqrt(
        math.pow(px - localPosition.dx, 2) + math.pow(py - localPosition.dy, 2),
      );
      if (dist < 30.0 && dist < bestDist) {
        bestDist = dist;
        bestIndex = i;
      }
    }
    return bestIndex;
  }

  void _handleHover(PointerEvent event, BoxConstraints constraints) {
    if (_isDragging) return;
    int closest = _findClosestBandIndex(
      event.localPosition,
      Size(constraints.maxWidth, constraints.maxHeight),
    );
    if (_hoverIndex != closest) {
      setState(() {
        _hoverIndex = closest;
      });
    }
  }

  void _handlePanDown(DragDownDetails details, BoxConstraints constraints) {
    int bestIndex = _findClosestBandIndex(
      details.localPosition,
      Size(constraints.maxWidth, constraints.maxHeight),
    );
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

    setState(() {
      _freqControllers[_activeBandIndex].text = newFreq.toStringAsFixed(1);
      _gainControllers[_activeBandIndex].text = newGain.toStringAsFixed(1);
    });

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
          color: isActive
              ? AppColors.primaryNeon.withValues(alpha: 0.15)
              : AppColors.background,
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
                  _saveCurrentState();
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
                    ? const Icon(
                        Icons.check,
                        size: 10,
                        color: AppColors.background,
                      )
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
                const Text(
                  'Type',
                  style: TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 8),
                InputDecorator(
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                  ),
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<EqType>(
                      value: _bandTypes[idx],
                      dropdownColor: AppColors.cardSurface,
                      isDense: true,
                      isExpanded: true,
                      style: const TextStyle(
                        color: AppColors.textPrimary,
                        fontSize: 14,
                      ),
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
                            _saveCurrentState();
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
                const Text(
                  'Freq (Hz)',
                  style: TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _freqControllers[idx],
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: 14,
                  ),
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                  ),
                  inputFormatters: [
                    FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*')),
                  ],
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
                const Text(
                  'Gain (dB)',
                  style: TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _gainControllers[idx],
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: 14,
                  ),
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                    signed: true,
                  ),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                  ),
                  inputFormatters: [
                    FilteringTextInputFormatter.allow(RegExp(r'^-?\d*\.?\d*')),
                  ],
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
                const Text(
                  'Q',
                  style: TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _qControllers[idx],
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: 14,
                  ),
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: const InputDecoration(
                    filled: true,
                    fillColor: AppColors.cardSurfaceSolid,
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                  ),
                  inputFormatters: [
                    FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*')),
                  ],
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
            return Listener(
              onPointerSignal: (pointerSignal) {
                if (pointerSignal is PointerScrollEvent) {
                  if (_activeBandIndex != -1 &&
                      _bandEnabled[_activeBandIndex]) {
                    double currentQ =
                        double.tryParse(_qControllers[_activeBandIndex].text) ??
                        0.707;
                    double delta = pointerSignal.scrollDelta.dy > 0
                        ? -0.1
                        : 0.1;
                    double newQ = (currentQ + delta).clamp(0.1, 10.0);
                    setState(() {
                      _qControllers[_activeBandIndex].text = newQ
                          .toStringAsFixed(3);
                    });
                    _sendThrottledUpdate();
                  }
                }
              },
              onPointerPanZoomUpdate: (event) {
                if (_activeBandIndex != -1 && _bandEnabled[_activeBandIndex]) {
                  double currentQ =
                      double.tryParse(_qControllers[_activeBandIndex].text) ??
                      0.707;
                  // Use pan delta dy (trackpad continuous scroll)
                  double delta = event.panDelta.dy > 0 ? -0.05 : 0.05;
                  if (event.panDelta.dy == 0) return;
                  double newQ = (currentQ + delta).clamp(0.1, 10.0);
                  setState(() {
                    _qControllers[_activeBandIndex].text = newQ.toStringAsFixed(
                      3,
                    );
                  });
                  _sendThrottledUpdate();
                }
              },
              child: MouseRegion(
                onHover: (event) => _handleHover(event, constraints),
                onExit: (_) => setState(() => _hoverIndex = -1),
                child: GestureDetector(
                  onSecondaryTapUp: (details) {
                    int bandIdx = _findClosestBandIndex(
                      details.localPosition,
                      Size(constraints.maxWidth, constraints.maxHeight),
                    );
                    if (bandIdx != -1) {
                      _showBandContextMenu(context, details.globalPosition, bandIdx);
                    } else if (_activeBandIndex != -1) {
                      _showBandContextMenu(context, details.globalPosition, _activeBandIndex);
                    }
                  },
                  onPanDown: (details) => _handlePanDown(details, constraints),
                  onPanUpdate: (details) =>
                      _handlePanUpdate(details, constraints),
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
                            minFreq: minFreq,
                            maxFreq: maxFreq,
                            minGain: minGain,
                            maxGain: maxGain,
                          ),
                          size: Size(
                            constraints.maxWidth,
                            constraints.maxHeight,
                          ),
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
              ),
            );
          },
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final config = ref.watch(configProvider);
    final hwChannelsAsync = ref.watch(hardwareChannelsProvider);
    int maxChannels = 24;
    if (config != null) {
      if (config.deviceName != null &&
          GlobalDeviceCache.channels.containsKey(config.deviceName)) {
        maxChannels = GlobalDeviceCache.channels[config.deviceName]!.length;
      } else if (hwChannelsAsync.value != null &&
          hwChannelsAsync.value!.isNotEmpty) {
        maxChannels = hwChannelsAsync.value!.length;
      }
    }

    int safeSelectedChannel = _selectedChannel;
    if (safeSelectedChannel > maxChannels) safeSelectedChannel = 1;

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
              '출력 채널 Mixer (Delay & EQ Eight)',
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
                  child: Text(
                    '채널',
                    style: TextStyle(color: AppColors.textSecondary),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: InputDecorator(
                    decoration: const InputDecoration(
                      filled: true,
                      fillColor: AppColors.background,
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 8,
                      ),
                    ),
                    child: DropdownButtonHideUnderline(
                      child: DropdownButton<int>(
                        value: safeSelectedChannel,
                        dropdownColor: AppColors.cardSurface,
                        isDense: true,
                        isExpanded: true,
                        style: const TextStyle(
                          color: AppColors.textPrimary,
                          fontSize: 14,
                        ),
                        items: List.generate(maxChannels, (index) => index + 1)
                            .map((ch) {
                              String side = ch % 2 != 0 ? "(L)" : "(R)";
                              return DropdownMenuItem<int>(
                                value: ch,
                                child: Text('Channel $ch $side'),
                              );
                            })
                            .toList(),
                        onChanged: (val) {
                          if (val != null) {
                            setState(() {
                              _selectedChannel = val;
                              _loadStateForChannel(val);
                            });
                          }
                        },
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    SizedBox(
                      width: 24,
                      child: Checkbox(
                        value: _isStereoLinked,
                        activeColor: AppColors.primaryNeon,
                        onChanged: (val) {
                          setState(() {
                            _isStereoLinked = val ?? true;
                            _saveCurrentState();
                            _sendThrottledUpdate();
                          });
                        },
                      ),
                    ),
                    const SizedBox(width: 4),
                    const Text(
                      'Link L/R',
                      style: TextStyle(
                        color: AppColors.textPrimary,
                        fontSize: 12,
                      ),
                    ),
                  ],
                ),
                const SizedBox(width: 16),
                const SizedBox(
                  width: 70,
                  child: Text(
                    'Delay (ms)',
                    style: TextStyle(
                      color: AppColors.textSecondary,
                      fontSize: 12,
                    ),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: TextField(
                    controller: _delayController,
                    style: const TextStyle(
                      color: AppColors.textPrimary,
                      fontSize: 14,
                    ),
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      filled: true,
                      fillColor: AppColors.background,
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 8,
                      ),
                    ),
                    inputFormatters: [
                      FilteringTextInputFormatter.allow(RegExp(r'^\d*\.?\d*')),
                    ],
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
                  child: const Text(
                    '닫기',
                    style: TextStyle(color: AppColors.textSecondary),
                  ),
                ),
                const SizedBox(width: 12),
                ElevatedButton(
                  style: ElevatedButton.styleFrom(
                    backgroundColor: AppColors.primaryNeon,
                    foregroundColor: AppColors.background,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(4),
                    ),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 24,
                      vertical: 12,
                    ),
                  ),
                  onPressed: () => _applyTuning(silent: false),
                  child: const Text(
                    '적용 (Apply)',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
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
    for (double g in [-24.0, -12.0, 0.0, 12.0, 24.0]) {
      double y = gainToY(g);
      canvas.drawLine(
        Offset(0, y),
        Offset(size.width, y),
        g == 0.0 ? zeroLinePaint : gridPaint,
      );
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
    int resolution = 300;
    List<Offset> points = [];

    for (int i = 0; i <= resolution; i++) {
      double x = (i / resolution) * size.width;
      double t = i / resolution;
      double f = math.exp(minLog + t * (maxLog - minLog));
      double omega = 2.0 * math.pi * f / 48000.0;

      double totalDb = 0.0;

      // Precise Biquad Magnitude Response for FabFilter Pro-Q3 standard
      for (int b = 0; b < 8; b++) {
        if (!bandEnabled[b]) continue;
        double bandF = freqs[b].clamp(minFreq, maxFreq);
        double bandG = gains[b];
        double bandQ = math.max(0.1, qs[b]);
        double w0 = 2.0 * math.pi * bandF / 48000.0;
        double alpha = math.sin(w0) / (2.0 * bandQ);
        double A = math.pow(10.0, bandG / 40.0).toDouble();

        double b0 = 1.0, b1 = 0.0, b2 = 0.0;
        double a0 = 1.0, a1 = 0.0, a2 = 0.0;

        switch (bandTypes[b]) {
          case EqType.bell:
            b0 = 1.0 + alpha * A;
            b1 = -2.0 * math.cos(w0);
            b2 = 1.0 - alpha * A;
            a0 = 1.0 + alpha / A;
            a1 = -2.0 * math.cos(w0);
            a2 = 1.0 - alpha / A;
            break;
          case EqType.lowCut:
            b0 = (1.0 + math.cos(w0)) / 2.0;
            b1 = -(1.0 + math.cos(w0));
            b2 = (1.0 + math.cos(w0)) / 2.0;
            a0 = 1.0 + alpha;
            a1 = -2.0 * math.cos(w0);
            a2 = 1.0 - alpha;
            break;
          case EqType.highCut:
            b0 = (1.0 - math.cos(w0)) / 2.0;
            b1 = 1.0 - math.cos(w0);
            b2 = (1.0 - math.cos(w0)) / 2.0;
            a0 = 1.0 + alpha;
            a1 = -2.0 * math.cos(w0);
            a2 = 1.0 - alpha;
            break;
          case EqType.lowShelf:
            b0 = A * ((A + 1.0) - (A - 1.0) * math.cos(w0) + 2.0 * math.sqrt(A) * alpha);
            b1 = 2.0 * A * ((A - 1.0) - (A + 1.0) * math.cos(w0));
            b2 = A * ((A + 1.0) - (A - 1.0) * math.cos(w0) - 2.0 * math.sqrt(A) * alpha);
            a0 = (A + 1.0) + (A - 1.0) * math.cos(w0) + 2.0 * math.sqrt(A) * alpha;
            a1 = -2.0 * ((A - 1.0) + (A + 1.0) * math.cos(w0));
            a2 = (A + 1.0) + (A - 1.0) * math.cos(w0) - 2.0 * math.sqrt(A) * alpha;
            break;
          case EqType.highShelf:
            b0 = A * ((A + 1.0) + (A - 1.0) * math.cos(w0) + 2.0 * math.sqrt(A) * alpha);
            b1 = -2.0 * A * ((A - 1.0) + (A + 1.0) * math.cos(w0));
            b2 = A * ((A + 1.0) + (A - 1.0) * math.cos(w0) - 2.0 * math.sqrt(A) * alpha);
            a0 = (A + 1.0) - (A - 1.0) * math.cos(w0) + 2.0 * math.sqrt(A) * alpha;
            a1 = 2.0 * ((A - 1.0) - (A + 1.0) * math.cos(w0));
            a2 = (A + 1.0) - (A - 1.0) * math.cos(w0) - 2.0 * math.sqrt(A) * alpha;
            break;
          case EqType.notch:
            b0 = 1.0;
            b1 = -2.0 * math.cos(w0);
            b2 = 1.0;
            a0 = 1.0 + alpha;
            a1 = -2.0 * math.cos(w0);
            a2 = 1.0 - alpha;
            break;
        }

        double nb0 = b0 / a0;
        double nb1 = b1 / a0;
        double nb2 = b2 / a0;
        double na1 = a1 / a0;
        double na2 = a2 / a0;

        double cosW = math.cos(omega);
        double cos2W = math.cos(2.0 * omega);
        double sinW = math.sin(omega);
        double sin2W = math.sin(2.0 * omega);

        double numReal = nb0 + nb1 * cosW + nb2 * cos2W;
        double numImag = nb1 * sinW + nb2 * sin2W;
        double denReal = 1.0 + na1 * cosW + na2 * cos2W;
        double denImag = na1 * sinW + na2 * sin2W;

        double numSq = numReal * numReal + numImag * numImag;
        double denSq = denReal * denReal + denImag * denImag;

        if (denSq > 0.0) {
          double magSq = numSq / denSq;
          if (magSq > 0.0) {
            totalDb += 10.0 * (math.log(magSq) / math.ln10);
          }
        }
      }
      points.add(Offset(x, gainToY(totalDb.clamp(-30.0, 30.0))));
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
        ..color = isActive || isHovered
            ? AppColors.primaryNeon
            : AppColors.cardSurfaceSolid
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
          Offset(px - textPainter.width / 2, py - textPainter.height / 2 - 16),
        );
      }
    }
  }

  void _drawTooltip(
    Canvas canvas,
    double px,
    double py,
    int bandIndex,
    Size size,
  ) {
    final String text =
        '${freqs[bandIndex].toStringAsFixed(1)} Hz\n${gains[bandIndex].toStringAsFixed(1)} dB';

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
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 4),
    );

    // Box
    canvas.drawRRect(rect, Paint()..color = const Color(0xFF2D2D2D));
    canvas.drawRRect(
      rect,
      Paint()
        ..color = Colors.white.withValues(alpha: 0.1)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1,
    );

    textPainter.paint(canvas, Offset(tx + padding.left, ty + padding.top));
  }

  @override
  bool shouldRepaint(covariant _EqCurvePainter oldDelegate) => true;
}
