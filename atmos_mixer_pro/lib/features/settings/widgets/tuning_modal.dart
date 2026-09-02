import 'dart:math' as math;
import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;
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
  final bool phaseInvert;
  final double gainDb;
  final List<bool> bandEnabled;
  final List<EqType> bandTypes;
  final List<int> bandSlopes;
  final List<double> freqs;
  final List<double> gains;
  final List<double> qs;
  final bool isStereoLinked;

  ChannelTuningState({
    required this.delay,
    required this.phaseInvert,
    required this.gainDb,
    required this.bandEnabled,
    required this.bandTypes,
    List<int>? bandSlopes,
    required this.freqs,
    required this.gains,
    required this.qs,
    required this.isStereoLinked,
  }) : bandSlopes = bandSlopes ?? List.filled(8, 12);

  factory ChannelTuningState.initial() {
    return ChannelTuningState(
      delay: 0.0,
      phaseInvert: false,
      gainDb: 0.0,
      bandEnabled: List.filled(8, false),
      bandTypes: List.filled(8, EqType.bell),
      bandSlopes: List.filled(8, 12),
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
    bool? phaseInvert,
    double? gainDb,
    List<bool>? bandEnabled,
    List<EqType>? bandTypes,
    List<int>? bandSlopes,
    List<double>? freqs,
    List<double>? gains,
    List<double>? qs,
    bool? isStereoLinked,
  }) {
    return ChannelTuningState(
      delay: delay ?? this.delay,
      phaseInvert: phaseInvert ?? this.phaseInvert,
      gainDb: gainDb ?? this.gainDb,
      bandEnabled: bandEnabled ?? this.bandEnabled,
      bandTypes: bandTypes ?? this.bandTypes,
      bandSlopes: bandSlopes ?? this.bandSlopes,
      freqs: freqs ?? this.freqs,
      gains: gains ?? this.gains,
      qs: qs ?? this.qs,
      isStereoLinked: isStereoLinked ?? this.isStereoLinked,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'delay': delay,
      'phaseInvert': phaseInvert,
      'gainDb': gainDb,
      'bandEnabled': bandEnabled,
      'bandTypes': bandTypes.map((e) => e.index).toList(),
      'bandSlopes': bandSlopes,
      'freqs': freqs,
      'gains': gains,
      'qs': qs,
      'isStereoLinked': isStereoLinked,
    };
  }

  factory ChannelTuningState.fromJson(Map<String, dynamic> json) {
    return ChannelTuningState(
      delay: (json['delay'] as num?)?.toDouble() ?? 0.0,
      phaseInvert: (json['phaseInvert'] as bool?) ?? false,
      gainDb: (json['gainDb'] as num?)?.toDouble() ?? 0.0,
      bandEnabled: json['bandEnabled'] != null
          ? (json['bandEnabled'] as List).cast<bool>()
          : List.filled(8, false),
      bandTypes: json['bandTypes'] != null
          ? (json['bandTypes'] as List)
              .map((e) => EqType.values[e as int])
              .toList()
          : List.filled(8, EqType.bell),
      bandSlopes: json['bandSlopes'] != null
          ? (json['bandSlopes'] as List).map((e) => (e as num).toInt()).toList()
          : List.filled(8, 12),
      freqs: json['freqs'] != null
          ? (json['freqs'] as List).map((e) => (e as num).toDouble()).toList()
          : List.generate(
              8,
              (i) => math.min(100 * math.pow(2, i), 20000.0).toDouble(),
            ),
      gains: json['gains'] != null
          ? (json['gains'] as List).map((e) => (e as num).toDouble()).toList()
          : List.filled(8, 0.0),
      qs: json['qs'] != null
          ? (json['qs'] as List).map((e) => (e as num).toDouble()).toList()
          : List.filled(8, 0.707),
      isStereoLinked: (json['isStereoLinked'] as bool?) ?? true,
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
          phaseInvert: tuning.phaseInvert,
          gainDb: tuning.gainDb,
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
          phaseInvert: setting.phaseInvert,
          gainDb: setting.gainDb,
          bandEnabled: bandEnabled,
          bandTypes: bandTypes,
          bandSlopes: List.filled(8, 12),
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

class _TuningModalState extends ConsumerState<TuningModal>
    with SingleTickerProviderStateMixin {
  int _selectedChannel = 1;
  bool _isStereoLinked = true;
  bool _phaseInvert = false;
  final TextEditingController _delayController = TextEditingController(
    text: '0.0',
  );
  final TextEditingController _gainController = TextEditingController(
    text: '0.0',
  );

  int _activeBandIndex = 0;
  int _hoverIndex = -1;
  bool _isDragging = false;
  Offset _dragStartPos = Offset.zero;
  double _dragStartGain = 0.0;

  // 8 bands state
  final List<bool> _bandEnabled = List.filled(8, false);
  final List<EqType> _bandTypes = List.filled(8, EqType.bell);
  final List<int> _bandSlopes = List.filled(8, 12);
  final List<TextEditingController> _freqControllers = [];
  final List<TextEditingController> _gainControllers = [];
  final List<TextEditingController> _qControllers = [];

  Timer? _throttleTimer;

  static const double minFreq = 20.0;
  static const double maxFreq = 20000.0;
  static const double minGain = -30.0;
  static const double maxGain = 30.0;

  // Variable Y-Axis Display Range & Auto Scale
  bool _isAutoScale = true;
  double _currentMaxDb = 12.0;
  double _targetMaxDb = 12.0;
  late AnimationController _scaleController;
  late Animation<double> _scaleAnimation;

  @override
  void initState() {
    super.initState();
    _scaleController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 200),
    );
    _scaleAnimation =
        Tween<double>(begin: 12.0, end: 12.0).animate(_scaleController);
    _scaleController.addListener(() {
      setState(() {
        _currentMaxDb = _scaleAnimation.value;
      });
    });

    for (int i = 0; i < 8; i++) {
      _freqControllers.add(TextEditingController());
      _gainControllers.add(TextEditingController());
      _qControllers.add(TextEditingController());
    }

    // We delay the load so we don't modify provider state during init.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadStateForChannel(_selectedChannel);
      _updateAutoScale(force: true);
    });
  }

  void _updateAutoScale({bool force = false}) {
    if (!_isAutoScale && !force) return;

    double maxAbsGain = 0.0;
    for (int i = 0; i < 8; i++) {
      final double g = (double.tryParse(_gainControllers[i].text) ?? 0.0).abs();
      if (_bandEnabled[i] || i == _activeBandIndex) {
        if (g > maxAbsGain) maxAbsGain = g;
      }
    }

    double newTarget;
    if (maxAbsGain > 12.0) {
      newTarget = 30.0;
    } else if (maxAbsGain > 6.0) {
      newTarget = 12.0;
    } else if (maxAbsGain > 3.0) {
      newTarget = 6.0;
    } else {
      newTarget = 3.0;
    }

    if ((newTarget - _targetMaxDb).abs() > 0.01 || force) {
      _targetMaxDb = newTarget;
      final beginVal = _currentMaxDb;
      _scaleAnimation = Tween<double>(
        begin: beginVal,
        end: newTarget,
      ).animate(CurvedAnimation(
        parent: _scaleController,
        curve: Curves.easeInOutCubic,
      ));
      _scaleController.forward(from: 0.0);
    }
  }

  void _setManualScale(double scale) {
    setState(() {
      _isAutoScale = false;
      _targetMaxDb = scale;
      final beginVal = _currentMaxDb;
      _scaleAnimation = Tween<double>(
        begin: beginVal,
        end: scale,
      ).animate(CurvedAnimation(
        parent: _scaleController,
        curve: Curves.easeInOutCubic,
      ));
      _scaleController.forward(from: 0.0);
    });
  }

  void _toggleAutoScale(bool auto) {
    setState(() {
      _isAutoScale = auto;
      if (auto) {
        _updateAutoScale(force: true);
      }
    });
  }

  void _loadStateForChannel(int channel) {
    final tuning = ref.read(tuningStateProvider.notifier).getTuning(channel);
    _delayController.text = tuning.delay.toStringAsFixed(1);
    _gainController.text = tuning.gainDb.toStringAsFixed(1);
    _phaseInvert = tuning.phaseInvert;
    _isStereoLinked = tuning.isStereoLinked;
    _bandEnabled.setAll(0, tuning.bandEnabled);
    _bandTypes.setAll(0, tuning.bandTypes);
    _bandSlopes.setAll(
      0,
      tuning.bandSlopes.length == 8
          ? tuning.bandSlopes
          : List.filled(8, 12),
    );

    for (int i = 0; i < 8; i++) {
      _freqControllers[i].text = tuning.freqs[i].toStringAsFixed(1);
      _gainControllers[i].text = tuning.gains[i].toStringAsFixed(1);
      _qControllers[i].text = tuning.qs[i].toStringAsFixed(3);
    }
    _updateAutoScale(force: true);
    setState(() {});
  }

  void _saveCurrentState() {
    final tuning = ChannelTuningState(
      delay: double.tryParse(_delayController.text) ?? 0.0,
      phaseInvert: _phaseInvert,
      gainDb: double.tryParse(_gainController.text) ?? 0.0,
      bandEnabled: List.from(_bandEnabled),
      bandTypes: List.from(_bandTypes),
      bandSlopes: List.from(_bandSlopes),
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
    _scaleController.dispose();
    _throttleTimer?.cancel();
    _delayController.dispose();
    _gainController.dispose();
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
      final double gain = double.tryParse(_gainController.text) ?? 0.0;
      final bool phaseInvert = _phaseInvert;

      final List<EqBand> bands = [];
      for (int i = 0; i < 8; i++) {
        final double freq = double.tryParse(_freqControllers[i].text) ?? 1000.0;
        final double g = double.tryParse(_gainControllers[i].text) ?? 0.0;
        final double q = double.tryParse(_qControllers[i].text) ?? 0.707;
        bands.add(
          EqBand(
            enabled: _bandEnabled[i],
            freq: freq,
            gain: g,
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
          phaseInvert: phaseInvert,
          gainDb: gain,
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
            phaseInvert: phaseInvert,
            gainDb: gain,
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
    gain = gain.clamp(-_currentMaxDb, _currentMaxDb);
    return (height / 2.0) - (gain / _currentMaxDb) * (height / 2.0);
  }

  int _findClosestBandIndex(Offset localPosition, Size size) {
    double bestDist = double.infinity;
    int bestIndex = -1;

    for (int i = 0; i < 8; i++) {
      if (!_bandEnabled[i] && i != _activeBandIndex) continue;
      double freq = double.tryParse(_freqControllers[i].text) ?? 1000.0;
      double gain = double.tryParse(_gainControllers[i].text) ?? 0.0;

      double px = _freqToX(freq, size.width);
      double py = _gainToY(gain, size.height);

      double dist = math.sqrt(
        math.pow(px - localPosition.dx, 2) + math.pow(py - localPosition.dy, 2),
      );
      if (dist < 32.0 && dist < bestDist) {
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

  static const List<Color> _bandColors = [
    Color(0xFFE05656), // 1: Red
    Color(0xFFF39C12), // 2: Orange
    Color(0xFFF1C40F), // 3: Yellow
    Color(0xFF2ECC71), // 4: Green
    Color(0xFF1ABC9C), // 5: Cyan/Teal
    Color(0xFF3498DB), // 6: Sky Blue
    Color(0xFF9B59B6), // 7: Purple
    Color(0xFFE84393), // 8: Magenta/Pink
  ];

  static const Color _dialogBg = Color(0xFF1E2229);
  static const Color _graphBg = Color(0xFF111620);
  static const Color _graphBorder = Color(0xFF1E2C3D);
  static const Color _inputBg = Color(0xFF15181E);
  static const Color _inputBorder = Color(0xFF2F3542);
  static const Color _activeCyan = Color(0xFF00D2FF);

  // --- UI Components ---

  Widget _buildStepperBox({
    required TextEditingController controller,
    required VoidCallback onStepUp,
    required VoidCallback onStepDown,
    required ValueChanged<String> onChanged,
    double width = 80,
    double height = 36,
    bool isSigned = false,
  }) {
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        color: _inputBg,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: _inputBorder, width: 1),
      ),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: Colors.white,
                fontSize: 13,
                fontWeight: FontWeight.w500,
              ),
              keyboardType: TextInputType.numberWithOptions(
                decimal: true,
                signed: isSigned,
              ),
              decoration: const InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(horizontal: 4, vertical: 8),
                border: InputBorder.none,
              ),
              inputFormatters: [
                FilteringTextInputFormatter.allow(
                  isSigned ? RegExp(r'^-?\d*\.?\d*') : RegExp(r'^\d*\.?\d*'),
                ),
              ],
              onChanged: onChanged,
            ),
          ),
          Container(
            width: 18,
            decoration: const BoxDecoration(
              border: Border(left: BorderSide(color: _inputBorder, width: 1)),
            ),
            child: Column(
              children: [
                Expanded(
                  child: InkWell(
                    onTap: onStepUp,
                    child: const Center(
                      child: Icon(Icons.keyboard_arrow_up, size: 12, color: Colors.white70),
                    ),
                  ),
                ),
                Container(height: 1, color: _inputBorder),
                Expanded(
                  child: InkWell(
                    onTap: onStepDown,
                    child: const Center(
                      child: Icon(Icons.keyboard_arrow_down, size: 12, color: Colors.white70),
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

  Widget _buildBandToggle(int index) {
    final bool isSelected = _activeBandIndex == index;
    final bool isEnabled = _bandEnabled[index];

    return GestureDetector(
      onTap: () {
        setState(() {
          _activeBandIndex = index;
        });
      },
      onSecondaryTap: () {
        setState(() {
          _bandEnabled[index] = !isEnabled;
          _activeBandIndex = index;
          _saveCurrentState();
          _sendThrottledUpdate();
        });
      },
      onDoubleTap: () {
        setState(() {
          _bandEnabled[index] = !isEnabled;
          _activeBandIndex = index;
          _saveCurrentState();
          _sendThrottledUpdate();
        });
      },
      child: Tooltip(
        message: 'Band ${index + 1} (${isEnabled ? "Active" : "Bypassed"})\n클릭: 선택 / 더블클릭: 활성화 토글',
        child: Container(
          width: 26,
          height: 26,
          margin: const EdgeInsets.symmetric(horizontal: 2.5),
          alignment: Alignment.center,
          decoration: BoxDecoration(
            // 1. 선택된 밴드: 선명한 시안(Cyan) 포인트 컬러 채우기
            //    선택 안 된 밴드: 차분한 다크 차콜 배경
            color: isSelected
                ? const Color(0xFF00E5FF)
                : (isEnabled ? const Color(0xFF22242B) : const Color(0xFF16181E)),
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: isSelected ? const Color(0xFF00E5FF) : Colors.white12,
              width: 1.2,
            ),
            boxShadow: isSelected
                ? [
                    BoxShadow(
                      color: const Color(0xFF00E5FF).withValues(alpha: 0.35),
                      blurRadius: 6,
                      spreadRadius: 1,
                    )
                  ]
                : null,
          ),
          child: Text(
            '${index + 1}',
            textAlign: TextAlign.center,
            style: TextStyle(
              // 선택 시 검정 글씨, 평소에는 은은한 화이트 글씨
              color: isSelected
                  ? Colors.black
                  : (isEnabled ? Colors.white70 : Colors.white30),
              fontSize: 11,
              fontWeight: isSelected ? FontWeight.bold : FontWeight.w600,
              height: 1.0,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildActiveBandControls() {
    final int idx = _activeBandIndex;
    final Color bandColor = _bandColors[idx];
    final bool isEnabled = _bandEnabled[idx];

    final double currentFreq =
        double.tryParse(_freqControllers[idx].text) ?? 1000.0;
    final double currentGain =
        double.tryParse(_gainControllers[idx].text) ?? 0.0;
    final double currentQ =
        double.tryParse(_qControllers[idx].text) ?? 0.707;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      decoration: BoxDecoration(
        color: const Color(0xFF1E1E24),
        borderRadius: BorderRadius.circular(16),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.35),
            blurRadius: 10,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          // 1. Left Section: Band Power + Filter Type + Slope
          SizedBox(
            width: 170,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    // Power Toggle Button
                    InkWell(
                      borderRadius: BorderRadius.circular(14),
                      onTap: () {
                        setState(() {
                          _bandEnabled[idx] = !_bandEnabled[idx];
                          _saveCurrentState();
                          _updateAutoScale();
                          _sendThrottledUpdate();
                        });
                      },
                      child: Container(
                        width: 26,
                        height: 26,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: isEnabled
                              ? bandColor.withValues(alpha: 0.25)
                              : const Color(0xFF282E3A),
                          border: Border.all(
                            color: isEnabled ? bandColor : const Color(0xFF4A5568),
                            width: 1.5,
                          ),
                        ),
                        child: Icon(
                          Icons.power_settings_new,
                          size: 15,
                          color: isEnabled ? bandColor : Colors.white38,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    // Band Label Pill
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                      decoration: BoxDecoration(
                        color: bandColor.withValues(alpha: 0.15),
                        borderRadius: BorderRadius.circular(10),
                        border: Border.all(color: bandColor.withValues(alpha: 0.5), width: 1),
                      ),
                      child: Text(
                        'Band ${idx + 1}',
                        style: TextStyle(
                          color: bandColor,
                          fontSize: 11,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                // Filter Type Dropdown
                Container(
                  height: 30,
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  decoration: BoxDecoration(
                    color: const Color(0xFF15181E),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: const Color(0xFF374151), width: 1),
                  ),
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<EqType>(
                      value: _bandTypes[idx],
                      dropdownColor: const Color(0xFF1E2229),
                      isDense: true,
                      isExpanded: true,
                      icon: const Icon(Icons.arrow_drop_down, color: Colors.white70, size: 18),
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w500,
                      ),
                      items: const [
                        DropdownMenuItem(
                          value: EqType.bell,
                          child: Text('Bell'),
                        ),
                        DropdownMenuItem(
                          value: EqType.lowCut,
                          child: Text('Low Cut'),
                        ),
                        DropdownMenuItem(
                          value: EqType.highCut,
                          child: Text('High Cut'),
                        ),
                        DropdownMenuItem(
                          value: EqType.lowShelf,
                          child: Text('Low Shelf'),
                        ),
                        DropdownMenuItem(
                          value: EqType.highShelf,
                          child: Text('High Shelf'),
                        ),
                        DropdownMenuItem(
                          value: EqType.notch,
                          child: Text('Notch'),
                        ),
                      ],
                      onChanged: (val) {
                        if (val != null) {
                          setState(() {
                            _bandTypes[idx] = val;
                            _saveCurrentState();
                            _updateAutoScale();
                            _sendThrottledUpdate();
                          });
                        }
                      },
                    ),
                  ),
                ),
                const SizedBox(height: 6),
                // Slope Dropdown / Indicator
                if (_bandTypes[idx] == EqType.lowCut || _bandTypes[idx] == EqType.highCut)
                  Container(
                    height: 24,
                    padding: const EdgeInsets.symmetric(horizontal: 6),
                    decoration: BoxDecoration(
                      color: const Color(0xFF15181E),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(color: const Color(0xFF374151), width: 1),
                    ),
                    child: DropdownButtonHideUnderline(
                      child: DropdownButton<int>(
                        value: _bandSlopes[idx],
                        dropdownColor: const Color(0xFF1E2229),
                        isDense: true,
                        icon: const Icon(Icons.arrow_drop_down, size: 16, color: _activeCyan),
                        style: const TextStyle(
                          color: _activeCyan,
                          fontSize: 11,
                          fontWeight: FontWeight.bold,
                        ),
                        items: const [
                          DropdownMenuItem(
                            value: 12,
                            child: Text('12 dB/oct'),
                          ),
                          DropdownMenuItem(
                            value: 18,
                            child: Text('18 dB/oct'),
                          ),
                          DropdownMenuItem(
                            value: 24,
                            child: Text('24 dB/oct'),
                          ),
                        ],
                        onChanged: (val) {
                          if (val != null) {
                            setState(() {
                              _bandSlopes[idx] = val;
                              _saveCurrentState();
                              _sendThrottledUpdate();
                            });
                          }
                        },
                      ),
                    ),
                  )
                else
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 4, horizontal: 2),
                    child: Text(
                      'Slope: N/A',
                      style: TextStyle(
                        color: Colors.white24,
                        fontSize: 10,
                      ),
                    ),
                  ),
              ],
            ),
          ),

          // Vertical Separator
          Container(
            width: 1,
            height: 90,
            margin: const EdgeInsets.symmetric(horizontal: 14),
            color: Colors.white12,
          ),

          // 2. Center Section: 3 Pro-Q 3 Style Rotary Knobs (FREQ, GAIN, Q)
          Expanded(
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                // FREQ Knob
                _AbletonKnob(
                  title: 'FREQ',
                  minLabel: '10 Hz',
                  maxLabel: '30 kHz',
                  value: currentFreq.clamp(10.0, 30000.0),
                  min: 10.0,
                  max: 30000.0,
                  isLogarithmic: true,
                  activeColor: bandColor,
                  onChanged: (val) {
                    setState(() {
                      _freqControllers[idx].text = val.toStringAsFixed(1);
                      _saveCurrentState();
                      _sendThrottledUpdate();
                    });
                  },
                  onEditingComplete: () {
                    _saveCurrentState();
                    _updateAutoScale();
                    _sendThrottledUpdate();
                  },
                ),
                // GAIN Knob
                _AbletonKnob(
                  title: 'GAIN',
                  minLabel: '-30',
                  maxLabel: '+30',
                  value: currentGain.clamp(-30.0, 30.0),
                  min: -30.0,
                  max: 30.0,
                  isBipolar: true,
                  activeColor: bandColor,
                  isEnabled: isEnabled,
                  onChanged: (val) {
                    setState(() {
                      _gainControllers[idx].text = val.toStringAsFixed(1);
                      _saveCurrentState();
                      _updateAutoScale();
                      _sendThrottledUpdate();
                    });
                  },
                  onEditingComplete: () {
                    _saveCurrentState();
                    _updateAutoScale();
                    _sendThrottledUpdate();
                  },
                ),
                // Q Knob
                _AbletonKnob(
                  title: 'Q',
                  minLabel: '0.025',
                  maxLabel: '40',
                  value: currentQ.clamp(0.025, 40.0),
                  min: 0.025,
                  max: 40.0,
                  isLogarithmic: true,
                  activeColor: bandColor,
                  onChanged: (val) {
                    setState(() {
                      _qControllers[idx].text = val.toStringAsFixed(3);
                      _saveCurrentState();
                      _sendThrottledUpdate();
                    });
                  },
                  onEditingComplete: () {
                    _saveCurrentState();
                    _sendThrottledUpdate();
                  },
                ),
              ],
            ),
          ),

          // Vertical Separator
          Container(
            width: 1,
            height: 90,
            margin: const EdgeInsets.symmetric(horizontal: 14),
            color: Colors.white12,
          ),

          // 3. Right Section: Quick Band Actions (Invert, Reset, Delete)
          SizedBox(
            width: 70,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                IconButton(
                  tooltip: 'Invert Gain (±)',
                  icon: const Icon(Icons.swap_vert, color: Colors.white70, size: 20),
                  onPressed: () {
                    setState(() {
                      double g = double.tryParse(_gainControllers[idx].text) ?? 0.0;
                      _gainControllers[idx].text = (-g).toStringAsFixed(1);
                      _saveCurrentState();
                      _updateAutoScale();
                      _sendThrottledUpdate();
                    });
                  },
                ),
                IconButton(
                  tooltip: 'Reset Band Defaults',
                  icon: const Icon(Icons.refresh, color: Colors.white70, size: 19),
                  onPressed: () {
                    setState(() {
                      _gainControllers[idx].text = '0.0';
                      _qControllers[idx].text = '0.707';
                      _bandTypes[idx] = EqType.bell;
                      _saveCurrentState();
                      _updateAutoScale();
                      _sendThrottledUpdate();
                    });
                  },
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
      height: 330,
      width: double.infinity,
      decoration: BoxDecoration(
        color: _graphBg,
        border: Border.all(color: _graphBorder, width: 1.2),
        borderRadius: BorderRadius.circular(8),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(8),
        child: LayoutBuilder(
          builder: (context, constraints) {
            return Listener(
              onPointerDown: (event) {
                if (event.buttons == kSecondaryMouseButton) {
                  int bandIdx = _findClosestBandIndex(
                    event.localPosition,
                    Size(constraints.maxWidth, constraints.maxHeight),
                  );
                  if (bandIdx != -1) {
                    _showBandContextMenu(context, event.position, bandIdx);
                  } else if (_activeBandIndex != -1) {
                    _showBandContextMenu(context, event.position, _activeBandIndex);
                  }
                  return;
                }

                if (event.buttons == kPrimaryMouseButton) {
                  int bestIndex = _findClosestBandIndex(
                    event.localPosition,
                    Size(constraints.maxWidth, constraints.maxHeight),
                  );
                  if (bestIndex != -1) {
                    setState(() {
                      _activeBandIndex = bestIndex;
                      _bandEnabled[bestIndex] = true;
                      _isDragging = true;
                      _dragStartPos = event.localPosition;
                      _dragStartGain = double.tryParse(_gainControllers[bestIndex].text) ?? 0.0;
                    });
                    double newFreq = _xToFreq(event.localPosition.dx, constraints.maxWidth).clamp(minFreq, maxFreq);
                    setState(() {
                      _freqControllers[_activeBandIndex].text = newFreq.toStringAsFixed(1);
                    });
                    _saveCurrentState();
                    _updateAutoScale();
                    _sendThrottledUpdate();
                  }
                }
              },
              onPointerMove: (event) {
                _handleHover(event, constraints);
                if (_isDragging && _activeBandIndex != -1) {
                  double newFreq = _xToFreq(event.localPosition.dx, constraints.maxWidth).clamp(minFreq, maxFreq);
                  double deltaY = event.localPosition.dy - _dragStartPos.dy;
                  double deltaGain = -(deltaY / constraints.maxHeight) * (2.0 * _currentMaxDb);
                  double newGain = (_dragStartGain + deltaGain).clamp(minGain, maxGain);

                  setState(() {
                    _freqControllers[_activeBandIndex].text = newFreq.toStringAsFixed(1);
                    _gainControllers[_activeBandIndex].text = newGain.toStringAsFixed(1);
                  });
                  _dragStartPos = event.localPosition;
                  _dragStartGain = newGain;

                  _saveCurrentState();
                  _updateAutoScale();
                  _sendThrottledUpdate();
                }
              },
              onPointerUp: (event) {
                if (_isDragging) {
                  setState(() => _isDragging = false);
                  _saveCurrentState();
                  _updateAutoScale();
                  _applyTuning(silent: true);
                }
              },
              onPointerCancel: (event) {
                if (_isDragging) {
                  setState(() => _isDragging = false);
                  _saveCurrentState();
                  _updateAutoScale();
                  _applyTuning(silent: true);
                }
              },
              onPointerSignal: (pointerSignal) {
                if (pointerSignal is PointerScrollEvent) {
                  int targetBand = _hoverIndex != -1 ? _hoverIndex : _activeBandIndex;
                  if (targetBand == -1 || !_bandEnabled[targetBand]) {
                    targetBand = _findClosestBandIndex(
                      pointerSignal.localPosition,
                      Size(constraints.maxWidth, constraints.maxHeight),
                    );
                  }

                  if (targetBand != -1 && _bandEnabled[targetBand]) {
                    double currentQ = double.tryParse(_qControllers[targetBand].text) ?? 0.707;
                    // Scroll up -> increase Q (narrower filter), Scroll down -> decrease Q (wider filter)
                    double delta = pointerSignal.scrollDelta.dy < 0 ? 0.05 : -0.05;
                    double newQ = (currentQ + delta).clamp(0.025, 40.0);
                    setState(() {
                      _activeBandIndex = targetBand;
                      _qControllers[targetBand].text = newQ.toStringAsFixed(3);
                    });
                    _sendThrottledUpdate();
                  }
                }
              },
              onPointerPanZoomUpdate: (event) {
                int targetBand = _hoverIndex != -1 ? _hoverIndex : _activeBandIndex;
                if (targetBand == -1 || !_bandEnabled[targetBand]) {
                  targetBand = _findClosestBandIndex(
                    event.localPosition,
                    Size(constraints.maxWidth, constraints.maxHeight),
                  );
                }

                if (targetBand != -1 && _bandEnabled[targetBand]) {
                  double currentQ = double.tryParse(_qControllers[targetBand].text) ?? 0.707;
                  double delta = 0.0;
                  if (event.scale != 1.0) {
                    delta = (event.scale - 1.0) * 0.2;
                  } else if (event.panDelta.dy != 0) {
                    delta = event.panDelta.dy < 0 ? 0.02 : -0.02;
                  }
                  if (delta.abs() > 0.0001) {
                    double newQ = (currentQ + delta).clamp(0.025, 40.0);
                    setState(() {
                      _activeBandIndex = targetBand;
                      _qControllers[targetBand].text = newQ.toStringAsFixed(3);
                    });
                    _sendThrottledUpdate();
                  }
                }
              },
              child: MouseRegion(
                onHover: (event) => _handleHover(event, constraints),
                onExit: (_) => setState(() => _hoverIndex = -1),
                child: Stack(
                  children: [
                    // Dynamic Background Grid with animated display scale
                    CustomPaint(
                      painter: _EqGridPainter(
                        minFreq: minFreq,
                        maxFreq: maxFreq,
                        renderedMaxDb: _currentMaxDb,
                      ),
                      size: Size(
                        constraints.maxWidth,
                        constraints.maxHeight,
                      ),
                    ),
                    // Dynamic Curve, Rainbow Master Stroke, Fills, Nodes and Tooltips
                    CustomPaint(
                      painter: _EqCurvePainter(
                        activeBandIndex: _activeBandIndex,
                        hoverIndex: _hoverIndex,
                        isDragging: _isDragging,
                        bandEnabled: _bandEnabled,
                        bandTypes: _bandTypes,
                        bandSlopes: _bandSlopes,
                        freqs: freqs,
                        gains: gains,
                        qs: qs,
                        minFreq: minFreq,
                        maxFreq: maxFreq,
                        renderedMaxDb: _currentMaxDb,
                        bandColors: _bandColors,
                      ),
                      size: Size(constraints.maxWidth, constraints.maxHeight),
                    ),
                    // Display Range Badge at bottom right
                    Positioned(
                      right: 10,
                      bottom: 10,
                      child: _buildDisplayRangeBadge(),
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

  Widget _buildDisplayRangeBadge() {
    String label = _isAutoScale
        ? 'Auto (${_targetMaxDb.toInt()} dB)'
        : '${_targetMaxDb.toInt()} dB';

    return Container(
      height: 24,
      decoration: BoxDecoration(
        color: const Color(0xDD1E2229),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white24, width: 1),
      ),
      child: PopupMenuButton<dynamic>(
        tooltip: 'Display Range',
        color: const Color(0xFF1E2229),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        offset: const Offset(0, -180),
        onSelected: (val) {
          if (val == 'auto') {
            _toggleAutoScale(!_isAutoScale);
          } else if (val is double) {
            _setManualScale(val);
          }
        },
        itemBuilder: (context) => [
          PopupMenuItem(
            value: 'auto',
            child: Row(
              children: [
                Icon(_isAutoScale ? Icons.check : null,
                    size: 16, color: _activeCyan),
                const SizedBox(width: 8),
                const Text('Auto Scale',
                    style: TextStyle(color: Colors.white, fontSize: 13)),
              ],
            ),
          ),
          const PopupMenuDivider(),
          ...[3.0, 6.0, 12.0, 30.0].map((db) => PopupMenuItem(
                value: db,
                child: Row(
                  children: [
                    Icon(
                        !_isAutoScale && _targetMaxDb == db
                            ? Icons.check
                            : null,
                        size: 16,
                        color: _activeCyan),
                    const SizedBox(width: 8),
                    Text('±${db.toInt()} dB',
                        style: const TextStyle(
                            color: Colors.white, fontSize: 13)),
                  ],
                ),
              )),
        ],
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                label,
                style: const TextStyle(
                  color: Colors.white70,
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(width: 4),
              const Icon(Icons.arrow_drop_down,
                  color: Colors.white70, size: 14),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final hwChannelsAsync = ref.watch(hardwareChannelsProvider);
    final hwChannels = hwChannelsAsync.value ?? [];
    final engineState = ref.watch(engineStateProvider);
    final int maxChannels = hwChannels.isNotEmpty
        ? hwChannels.length
        : (engineState.outputChannelCount > 0
            ? engineState.outputChannelCount
            : 2);

    int safeSelectedChannel = _selectedChannel;
    if (safeSelectedChannel > maxChannels) safeSelectedChannel = 1;

    return Dialog(
      backgroundColor: _dialogBg,
      insetPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: const BorderSide(color: Color(0xFF2E3846), width: 1),
      ),
      child: Container(
        width: 980,
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
        child: SingleChildScrollView(
          child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Window Title Header
            const Center(
              child: Text(
                '출력 채널 Mixer (Delay, Phase & EQ Eight)',
                style: TextStyle(
                  color: Colors.white,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const SizedBox(height: 14),

            // Top Row: Channel, Link, Phase, Gain, Delay Control Row
            Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                // 1. Channel Selector
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      '채널',
                      style: TextStyle(
                        color: Colors.white70,
                        fontSize: 12,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 6),
                    Container(
                      width: 170,
                      height: 36,
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      decoration: BoxDecoration(
                        color: _inputBg,
                        borderRadius: BorderRadius.circular(6),
                        border: Border.all(color: _inputBorder, width: 1),
                      ),
                      child: DropdownButtonHideUnderline(
                        child: DropdownButton<int>(
                          value: safeSelectedChannel,
                          dropdownColor: const Color(0xFF1E2229),
                          isDense: true,
                          isExpanded: true,
                          icon: const Icon(Icons.arrow_drop_down, color: Colors.white70),
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 13,
                            fontWeight: FontWeight.w500,
                          ),
                          items: List.generate(maxChannels, (index) {
                            final ch = index + 1;
                            final side = ch % 2 != 0 ? "(L)" : "(R)";
                            final hwName =
                                index < hwChannels.length &&
                                    hwChannels[index].isNotEmpty
                                ? ' • ${hwChannels[index]}'
                                : '';
                            return DropdownMenuItem<int>(
                              value: ch,
                              child: Text(
                                'Channel $ch $side$hwName',
                                overflow: TextOverflow.ellipsis,
                              ),
                            );
                          }).toList(),
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
                  ],
                ),
                const SizedBox(width: 14),

                // 2. Link L/R Checkbox
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: InkWell(
                    onTap: () {
                      setState(() {
                        _isStereoLinked = !_isStereoLinked;
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      });
                    },
                    borderRadius: BorderRadius.circular(4),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          width: 18,
                          height: 18,
                          decoration: BoxDecoration(
                            color: _isStereoLinked ? _activeCyan : _inputBg,
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(
                              color: _isStereoLinked ? _activeCyan : _inputBorder,
                              width: 1,
                            ),
                          ),
                          child: _isStereoLinked
                              ? const Icon(Icons.check, size: 13, color: Colors.black)
                              : null,
                        ),
                        const SizedBox(width: 6),
                        const Text(
                          'Link L/R',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 13,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 18),

                // 3. Phase Invert Toggle Pill
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: InkWell(
                    onTap: () {
                      setState(() {
                        _phaseInvert = !_phaseInvert;
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      });
                    },
                    borderRadius: BorderRadius.circular(12),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        AnimatedContainer(
                          duration: const Duration(milliseconds: 180),
                          width: 38,
                          height: 20,
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(10),
                            color: _phaseInvert ? _activeCyan : const Color(0xFF2A313D),
                            border: Border.all(
                              color: _phaseInvert ? _activeCyan : const Color(0xFF3F4B5A),
                              width: 1,
                            ),
                          ),
                          alignment: _phaseInvert ? Alignment.centerRight : Alignment.centerLeft,
                          child: Container(
                            width: 14,
                            height: 14,
                            decoration: BoxDecoration(
                              shape: BoxShape.circle,
                              color: _phaseInvert ? Colors.black : Colors.white70,
                            ),
                          ),
                        ),
                        const SizedBox(width: 6),
                        const Text(
                          'Ø Phase (180°)',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 13,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const Spacer(),

                // 4. Gain (dB)
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Gain (dB)',
                      style: TextStyle(
                        color: Colors.white70,
                        fontSize: 12,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 6),
                    _buildStepperBox(
                      controller: _gainController,
                      width: 80,
                      height: 36,
                      isSigned: true,
                      onStepUp: () {
                        double current = double.tryParse(_gainController.text) ?? 0.0;
                        double next = (current + 0.5).clamp(-24.0, 24.0);
                        _gainController.text = next.toStringAsFixed(1);
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                      onStepDown: () {
                        double current = double.tryParse(_gainController.text) ?? 0.0;
                        double next = (current - 0.5).clamp(-24.0, 24.0);
                        _gainController.text = next.toStringAsFixed(1);
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                      onChanged: (v) {
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                    ),
                  ],
                ),
                const SizedBox(width: 12),

                // 5. Delay (ms)
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Delay (ms)',
                      style: TextStyle(
                        color: Colors.white70,
                        fontSize: 12,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 6),
                    _buildStepperBox(
                      controller: _delayController,
                      width: 80,
                      height: 36,
                      isSigned: false,
                      onStepUp: () {
                        double current = double.tryParse(_delayController.text) ?? 0.0;
                        double next = (current + 1.0).clamp(0.0, 500.0);
                        _delayController.text = next.toStringAsFixed(1);
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                      onStepDown: () {
                        double current = double.tryParse(_delayController.text) ?? 0.0;
                        double next = (current - 1.0).clamp(0.0, 500.0);
                        _delayController.text = next.toStringAsFixed(1);
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                      onChanged: (v) {
                        _saveCurrentState();
                        _sendThrottledUpdate();
                      },
                    ),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 14),

            // EQ Curve Display
            _buildInteractiveEqCurve(),

            const SizedBox(height: 14),

            // 8 Bands Selection Pills
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: List.generate(8, (i) => _buildBandToggle(i)),
            ),

            const SizedBox(height: 14),

            // Active Band Parameter Controls (4 Cards)
            _buildActiveBandControls(),

            const SizedBox(height: 18),

            // Bottom Buttons
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text(
                    '닫기',
                    style: TextStyle(
                      color: _activeCyan,
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                ElevatedButton(
                  style: ElevatedButton.styleFrom(
                    backgroundColor: _activeCyan,
                    foregroundColor: Colors.black,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 24,
                      vertical: 12,
                    ),
                    elevation: 0,
                  ),
                  onPressed: () => _applyTuning(silent: false),
                  child: const Text(
                    '적용 (Apply)',
                    style: TextStyle(
                      fontWeight: FontWeight.bold,
                      fontSize: 14,
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      ),
    );
  }

  void _showBandContextMenu(BuildContext context, Offset globalPosition, int bandIndex) {
    showMenu(
      context: context,
      position: RelativeRect.fromLTRB(
        globalPosition.dx,
        globalPosition.dy,
        globalPosition.dx,
        globalPosition.dy,
      ),
      color: const Color(0xFF1E2229),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      items: <PopupMenuEntry<dynamic>>[
        PopupMenuItem<dynamic>(
          value: 'toggle',
          child: Text(
            _bandEnabled[bandIndex] ? 'Disable Band' : 'Enable Band',
            style: const TextStyle(color: Colors.white),
          ),
        ),
        const PopupMenuDivider(),
        ...EqType.values.map((type) => PopupMenuItem<dynamic>(
              value: type,
              child: Row(
                children: [
                  Icon(
                    _bandTypes[bandIndex] == type ? Icons.check : null,
                    size: 16,
                    color: _activeCyan,
                  ),
                  const SizedBox(width: 8),
                  Text(type.name, style: const TextStyle(color: Colors.white)),
                ],
              ),
            )),
      ],
    ).then((value) {
      if (value != null) {
        setState(() {
          if (value == 'toggle') {
            _bandEnabled[bandIndex] = !_bandEnabled[bandIndex];
          } else if (value is EqType) {
            _bandTypes[bandIndex] = value;
          }
          _saveCurrentState();
          _sendThrottledUpdate();
        });
      }
    });
  }
}

// --------------------------------------------------------------------------
// Painters
// --------------------------------------------------------------------------

class _EqGridPainter extends CustomPainter {
  final double minFreq;
  final double maxFreq;
  final double renderedMaxDb;

  _EqGridPainter({
    required this.minFreq,
    required this.maxFreq,
    required this.renderedMaxDb,
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
      g = g.clamp(-renderedMaxDb, renderedMaxDb);
      return (size.height / 2.0) - (g / renderedMaxDb) * (size.height / 2.0);
    }

    final gridPaint = Paint()
      ..color = const Color(0xFF1E2633)
      ..strokeWidth = 1;
    final majorGridPaint = Paint()
      ..color = const Color(0xFF283548)
      ..strokeWidth = 1;
    final zeroLinePaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.4)
      ..strokeWidth = 1.5;

    // Determine grid steps based on current scale mode
    List<double> gainSteps;
    if (renderedMaxDb <= 4.5) {
      gainSteps = [3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -3.0];
    } else if (renderedMaxDb <= 9.0) {
      gainSteps = [6.0, 4.0, 2.0, 0.0, -2.0, -4.0, -6.0];
    } else if (renderedMaxDb <= 18.0) {
      gainSteps = [12.0, 8.0, 4.0, 0.0, -4.0, -8.0, -12.0];
    } else {
      gainSteps = [30.0, 20.0, 10.0, 0.0, -10.0, -20.0, -30.0];
    }

    for (double g in gainSteps) {
      double y = gainToY(g);
      canvas.drawLine(
        Offset(0, y),
        Offset(size.width, y),
        g == 0.0 ? zeroLinePaint : gridPaint,
      );

      // Y-Axis label on the left
      String gText = g == 0.0 ? '0' : (g > 0 ? '+${g.toInt()}' : '${g.toInt()}');
      final textPainter = TextPainter(
        text: TextSpan(
          text: gText,
          style: TextStyle(
            color: g == 0.0 ? const Color(0xFF8EA8C3) : const Color(0xFF55687E),
            fontSize: 9.5,
            fontWeight: FontWeight.w600,
          ),
        ),
        textDirection: TextDirection.ltr,
      );
      textPainter.layout();

      double labelY = (y - textPainter.height / 2).clamp(4.0, size.height - textPainter.height - 4.0);
      textPainter.paint(
        canvas,
        Offset(8, labelY),
      );
    }

    // Vertical lines (Freq)
    final freqs = [
      20.0,
      50.0,
      100.0,
      200.0,
      500.0,
      1000.0,
      2000.0,
      5000.0,
      10000.0,
      20000.0,
    ];
    for (double f in freqs) {
      double x = freqToX(f);
      bool isMajor = f == 100.0 || f == 1000.0 || f == 10000.0;
      canvas.drawLine(
        Offset(x, 0),
        Offset(x, size.height),
        isMajor ? majorGridPaint : gridPaint,
      );

      // Skip '20' label to avoid overlapping with bottom-left gain label
      if (f == 20.0) continue;

      String fText = f >= 1000.0
          ? '${(f / 1000).toStringAsFixed(f % 1000 == 0 ? 0 : 1)}k'
          : '${f.toInt()}';
      final textPainter = TextPainter(
        text: TextSpan(
          text: fText,
          style: TextStyle(
            color: isMajor ? const Color(0xFF8EA8C3) : const Color(0xFF55687E),
            fontSize: 9.5,
            fontWeight: FontWeight.w600,
          ),
        ),
        textDirection: TextDirection.ltr,
      );
      textPainter.layout();

      double labelX = (x - textPainter.width / 2).clamp(4.0, size.width - textPainter.width - 6.0);
      textPainter.paint(
        canvas,
        Offset(labelX, size.height - textPainter.height - 4),
      );
    }
  }

  @override
  bool shouldRepaint(covariant _EqGridPainter oldDelegate) =>
      oldDelegate.renderedMaxDb != renderedMaxDb;
}

class _EqCurvePainter extends CustomPainter {
  final int activeBandIndex;
  final int hoverIndex;
  final bool isDragging;
  final List<bool> bandEnabled;
  final List<EqType> bandTypes;
  final List<int> bandSlopes;
  final List<double> freqs;
  final List<double> gains;
  final List<double> qs;
  final double minFreq;
  final double maxFreq;
  final double renderedMaxDb;
  final List<Color> bandColors;

  _EqCurvePainter({
    required this.activeBandIndex,
    required this.hoverIndex,
    required this.isDragging,
    required this.bandEnabled,
    required this.bandTypes,
    required this.bandSlopes,
    required this.freqs,
    required this.gains,
    required this.qs,
    required this.minFreq,
    required this.maxFreq,
    required this.renderedMaxDb,
    required this.bandColors,
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
      g = g.clamp(-renderedMaxDb, renderedMaxDb);
      return (size.height / 2.0) - (g / renderedMaxDb) * (size.height / 2.0);
    }

    // Function to calculate single band response in dB
    double calculateSingleBandDb(int b, double f, double omega) {
      double bandF = freqs[b].clamp(minFreq, maxFreq);
      double bandG = gains[b];
      double bandQ = math.max(0.025, qs[b]);
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
          b2 = A * ((A + 1.0) - (A - 1.0) * math.cos(w0) - 2.0 * math.sqrt(A) * alpha);
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
          double magDb = 10.0 * (math.log(magSq) / math.ln10);
          if (bandTypes[b] == EqType.lowCut || bandTypes[b] == EqType.highCut) {
            double slopeFactor = (bandSlopes.length > b ? bandSlopes[b] : 12) / 12.0;
            return bandG + slopeFactor * magDb;
          }
          return magDb;
        }
      }
      return 0.0;
    }

    int resolution = 400;
    List<Offset> compositePoints = [];

    for (int i = 0; i <= resolution; i++) {
      double x = (i / resolution) * size.width;
      double t = i / resolution;
      double f = math.exp(minLog + t * (maxLog - minLog));
      double omega = 2.0 * math.pi * f / 48000.0;

      double totalDb = 0.0;

      for (int b = 0; b < 8; b++) {
        if (!bandEnabled[b]) continue;
        double singleDb = calculateSingleBandDb(b, f, omega);
        totalDb += singleDb;
      }
      compositePoints.add(Offset(x, gainToY(totalDb.clamp(-renderedMaxDb, renderedMaxDb))));
    }

    // 1. Draw Master Shaded Filter Fill (Bounded precisely by the Composite Curve and 0 dB Baseline)
    if (compositePoints.isNotEmpty) {
      Path fillPath = Path();
      fillPath.moveTo(0, gainToY(0.0));
      for (var p in compositePoints) {
        fillPath.lineTo(p.dx, p.dy);
      }
      fillPath.lineTo(size.width, gainToY(0.0));
      fillPath.close();

      // Soft spectral horizontal gradient under the entire EQ curve
      final masterFillPaint = Paint()
        ..shader = ui.Gradient.linear(
          const Offset(0, 0),
          Offset(size.width, 0),
          bandColors.map((c) => c.withValues(alpha: 0.14)).toList(),
          List.generate(8, (i) => i / 7.0),
        )
        ..style = PaintingStyle.fill;
      canvas.drawPath(fillPath, masterFillPaint);

      // Active Band Focus Glow (Clipped strictly inside the white curve area)
      if (activeBandIndex != -1 && bandEnabled[activeBandIndex]) {
        double activeX = freqToX(freqs[activeBandIndex]);
        double bandQ = math.max(0.025, qs[activeBandIndex]);
        double bandWidthPx = (size.width / 2.5) / math.sqrt(bandQ);

        canvas.save();
        canvas.clipPath(fillPath);
        final activeGlowPaint = Paint()
          ..shader = ui.Gradient.radial(
            Offset(
              activeX,
              gainToY(gains[activeBandIndex]),
            ),
            bandWidthPx.clamp(60.0, size.width),
            [
              bandColors[activeBandIndex].withValues(alpha: 0.35),
              bandColors[activeBandIndex].withValues(alpha: 0.0),
            ],
          );
        canvas.drawRect(Rect.fromLTWH(0, 0, size.width, size.height), activeGlowPaint);
        canvas.restore();
      }
    }

    // 2. Draw Composite EQ Curve (Pure Solid White Stroke)
    if (compositePoints.isNotEmpty) {
      Path curvePath = Path();
      curvePath.moveTo(compositePoints.first.dx, compositePoints.first.dy);
      for (int i = 1; i < compositePoints.length; i++) {
        curvePath.lineTo(compositePoints[i].dx, compositePoints[i].dy);
      }

      final curvePaint = Paint()
        ..color = Colors.white
        ..strokeWidth = 2.4
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round
        ..isAntiAlias = true;
      canvas.drawPath(curvePath, curvePaint);
    }

    // 3. Draw 8 Band Nodes with unique colors and parameter boxes
    for (int b = 0; b < 8; b++) {
      if (!bandEnabled[b]) continue;

      bool isActive = b == activeBandIndex;
      bool isHovered = b == hoverIndex;
      double px = freqToX(freqs[b]);
      double py = gainToY(gains[b]);
      Color nodeColor = bandColors[b];

      // Clean outer white ring for active node
      if (isActive) {
        canvas.drawCircle(
          Offset(px, py),
          12.0,
          Paint()
            ..color = Colors.white
            ..strokeWidth = 2.0
            ..style = PaintingStyle.stroke,
        );
      }

      // Main Node Circle
      canvas.drawCircle(
        Offset(px, py),
        isActive ? 10.0 : 8.5,
        Paint()
          ..color = nodeColor
          ..style = PaintingStyle.fill,
      );

      // Node Number inside circle
      final numPainter = TextPainter(
        text: TextSpan(
          text: '${b + 1}',
          style: const TextStyle(
            color: Colors.white,
            fontSize: 10,
            fontWeight: FontWeight.bold,
            height: 1.0,
          ),
        ),
        textDirection: TextDirection.ltr,
      );
      numPainter.layout();
      numPainter.paint(
        canvas,
        Offset(px - numPainter.width / 2, py - numPainter.height / 2),
      );

      // Parameter readout box above/below active or hovered node
      if (isActive || isHovered) {
        String topText = freqs[b] >= 1000.0
            ? '${(freqs[b] / 1000).toStringAsFixed(1)} kHz'
            : '${freqs[b].toStringAsFixed(1)} Hz';
        String bottomText = '${gains[b] >= 0 ? '+' : ''}${gains[b].toStringAsFixed(1)} dB';

        final boxTextPainter = TextPainter(
          text: TextSpan(
            children: [
              TextSpan(
                text: '$topText\n',
                style: TextStyle(
                  color: nodeColor,
                  fontSize: 10,
                  fontWeight: FontWeight.bold,
                  height: 1.2,
                ),
              ),
              TextSpan(
                text: bottomText,
                style: TextStyle(
                  color: nodeColor,
                  fontSize: 10,
                  fontWeight: FontWeight.bold,
                  height: 1.2,
                ),
              ),
            ],
          ),
          textAlign: TextAlign.center,
          textDirection: TextDirection.ltr,
        );
        boxTextPainter.layout();

        double padH = 6.0;
        double padV = 4.0;
        double bWidth = boxTextPainter.width + padH * 2;
        double bHeight = boxTextPainter.height + padV * 2;

        double bx = (px - bWidth / 2).clamp(4.0, size.width - bWidth - 4.0);
        double by = py - bHeight - 16.0;
        if (by < 4.0) by = py + 16.0;

        final badgeRect = RRect.fromRectAndRadius(
          Rect.fromLTWH(bx, by, bWidth, bHeight),
          const Radius.circular(4),
        );

        canvas.drawRRect(
          badgeRect,
          Paint()..color = const Color(0xEE141922),
        );
        canvas.drawRRect(
          badgeRect,
          Paint()
            ..color = nodeColor.withValues(alpha: 0.8)
            ..strokeWidth = 1
            ..style = PaintingStyle.stroke,
        );

        boxTextPainter.paint(canvas, Offset(bx + padH, by + padV));
      }
    }
  }

  @override
  bool shouldRepaint(covariant _EqCurvePainter oldDelegate) => true;
}

// ---------------------------------------------------------------------------
// Ableton Live EQ Eight Style Rotary Knob Widget
// ---------------------------------------------------------------------------
class _AbletonKnob extends StatefulWidget {
  final String title;
  final String minLabel;
  final String maxLabel;
  final double value;
  final double min;
  final double max;
  final bool isLogarithmic;
  final bool isBipolar;
  final Color activeColor;
  final bool isEnabled;
  final ValueChanged<double> onChanged;
  final VoidCallback? onEditingComplete;

  const _AbletonKnob({
    required this.title,
    required this.minLabel,
    required this.maxLabel,
    required this.value,
    required this.min,
    required this.max,
    this.isLogarithmic = false,
    this.isBipolar = false,
    required this.activeColor,
    this.isEnabled = true,
    required this.onChanged,
    this.onEditingComplete,
  });

  @override
  State<_AbletonKnob> createState() => _AbletonKnobState();
}

class _AbletonKnobState extends State<_AbletonKnob> {
  bool _isHovered = false;
  bool _isEditing = false;
  late TextEditingController _textController;
  late FocusNode _focusNode;

  @override
  void initState() {
    super.initState();
    _textController = TextEditingController(text: _formatEditValue(widget.value));
    _focusNode = FocusNode();
    _focusNode.addListener(() {
      if (!_focusNode.hasFocus && _isEditing) {
        _commitText();
      }
    });
  }

  @override
  void didUpdateWidget(_AbletonKnob oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_isEditing && oldWidget.value != widget.value) {
      _textController.text = _formatEditValue(widget.value);
    }
  }

  @override
  void dispose() {
    _textController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  String _formatEditValue(double v) {
    final String t = widget.title.toUpperCase();
    if (t == 'FREQ') {
      return v >= 1000.0 ? (v / 1000.0).toStringAsFixed(1) : v.toStringAsFixed(0);
    } else if (t == 'GAIN') {
      return (v >= 0 ? '+' : '') + v.toStringAsFixed(1);
    } else {
      return v.toStringAsFixed(3);
    }
  }

  String _formatDisplayValue(double v) {
    final String t = widget.title.toUpperCase();
    if (t == 'FREQ') {
      if (v >= 1000.0) {
        if (v % 1000 == 0) {
          final int intVal = v.toInt();
          return '${intVal.toString().replaceAllMapped(RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))'), (m) => '${m[1]},')} Hz';
        } else {
          return '${(v / 1000.0).toStringAsFixed(1)} kHz';
        }
      } else {
        return '${v.toStringAsFixed(0)} Hz';
      }
    } else if (t == 'GAIN') {
      if (!widget.isEnabled) return '0.0 dB';
      return '${v >= 0 ? '+' : ''}${v.toStringAsFixed(1)} dB';
    } else {
      return v.toStringAsFixed(3);
    }
  }

  void _commitText() {
    setState(() {
      _isEditing = false;
    });
    String raw = _textController.text.trim();
    bool isKilo = raw.toLowerCase().endsWith('k') || raw.toLowerCase().contains('khz');
    String clean = raw.replaceAll(RegExp(r'[^\d.-]'), '');
    double? parsed = double.tryParse(clean);
    if (parsed != null) {
      if (widget.title.toUpperCase() == 'FREQ') {
        if (isKilo || (parsed > 0 && parsed <= 30.0)) {
          parsed *= 1000.0;
        }
      }
      double clamped = parsed.clamp(widget.min, widget.max);
      widget.onChanged(clamped);
      widget.onEditingComplete?.call();
    }
  }

  double _valueToNorm(double val) {
    if (widget.isLogarithmic) {
      final double logMin = math.log(math.max(1e-6, widget.min));
      final double logMax = math.log(widget.max);
      final double logVal = math.log(val.clamp(widget.min, widget.max));
      return ((logVal - logMin) / (logMax - logMin)).clamp(0.0, 1.0);
    } else {
      return ((val - widget.min) / (widget.max - widget.min)).clamp(0.0, 1.0);
    }
  }

  double _normToValue(double norm) {
    final double clamped = norm.clamp(0.0, 1.0);
    if (widget.isLogarithmic) {
      final double logMin = math.log(math.max(1e-6, widget.min));
      final double logMax = math.log(widget.max);
      return math.exp(logMin + clamped * (logMax - logMin)).clamp(widget.min, widget.max);
    } else {
      return (widget.min + clamped * (widget.max - widget.min)).clamp(widget.min, widget.max);
    }
  }

  void _handleDrag(DragUpdateDetails details) {
    final double currentNorm = _valueToNorm(widget.value);
    final bool isShift = HardwareKeyboard.instance.isShiftPressed;
    final double sensitivity = isShift ? 0.0006 : 0.006;
    final double delta = -details.delta.dy * sensitivity;
    final double newNorm = (currentNorm + delta).clamp(0.0, 1.0);
    final double newVal = _normToValue(newNorm);
    widget.onChanged(newVal);
  }

  void _handleScroll(PointerScrollEvent event) {
    final double currentNorm = _valueToNorm(widget.value);
    final bool isShift = HardwareKeyboard.instance.isShiftPressed;
    final double step = isShift ? 0.003 : 0.025;
    final double delta = event.scrollDelta.dy < 0 ? step : -step;
    final double newNorm = (currentNorm + delta).clamp(0.0, 1.0);
    final double newVal = _normToValue(newNorm);
    widget.onChanged(newVal);
    widget.onEditingComplete?.call();
  }

  @override
  Widget build(BuildContext context) {
    final double norm = _valueToNorm(widget.value);

    return Opacity(
      opacity: widget.isEnabled ? 1.0 : 0.45,
      child: MouseRegion(
        onEnter: (_) => setState(() => _isHovered = true),
        onExit: (_) => setState(() => _isHovered = false),
        child: Listener(
          onPointerSignal: (event) {
            if (widget.isEnabled && event is PointerScrollEvent) {
              _handleScroll(event);
            }
          },
          child: GestureDetector(
            onVerticalDragUpdate: widget.isEnabled ? _handleDrag : null,
            onVerticalDragEnd: (_) => widget.onEditingComplete?.call(),
            onDoubleTap: widget.isEnabled
                ? () {
                    setState(() {
                      _isEditing = true;
                      _textController.text = _formatEditValue(widget.value);
                      _textController.selection = TextSelection(
                        baseOffset: 0,
                        extentOffset: _textController.text.length,
                      );
                    });
                    Future.delayed(const Duration(milliseconds: 50), () {
                      if (mounted) _focusNode.requestFocus();
                    });
                  }
                : null,
            child: SizedBox(
              width: 84.0, // 각 노브 영역의 고정 너비 (옆 노브와 절대 겹치지 않음)
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  // 1. 로터리 노브 본체 (44x44) & Hover Tooltip
                  Stack(
                    alignment: Alignment.center,
                    clipBehavior: Clip.none,
                    children: [
                      SizedBox(
                        width: 44,
                        height: 44,
                        child: CustomPaint(
                          painter: _AbletonKnobPainter(
                            norm: norm,
                            isBipolar: widget.isBipolar,
                            isQKnob: widget.title.toUpperCase() == 'Q',
                            activeColor: widget.activeColor,
                            isHovered: _isHovered && widget.isEnabled,
                          ),
                        ),
                      ),
                      if (_isHovered && !_isEditing && widget.isEnabled)
                        Positioned(
                          top: -22,
                          child: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                            decoration: BoxDecoration(
                              color: const Color(0xFF151921),
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: widget.activeColor.withValues(alpha: 0.8),
                                width: 1,
                              ),
                              boxShadow: [
                                BoxShadow(
                                  color: Colors.black.withValues(alpha: 0.6),
                                  blurRadius: 4,
                                  offset: const Offset(0, 2),
                                ),
                              ],
                            ),
                            child: Text(
                              _formatDisplayValue(widget.value),
                              style: TextStyle(
                                color: widget.activeColor,
                                fontSize: 10,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                  const SizedBox(height: 4),

                  // 2. 노브 바로 아래 좌/우 최소/최대값 (격리된 노브 컬럼 내 여유 있는 너비)
                  SizedBox(
                    width: 72.0,
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          widget.minLabel,
                          style: const TextStyle(fontSize: 9, color: Colors.white38),
                        ),
                        Text(
                          widget.maxLabel,
                          style: const TextStyle(fontSize: 9, color: Colors.white38),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 4),

                  // 3. 현재 실시간 값 뱃지 (더블클릭 시 키보드 입력 가능)
                  _isEditing
                      ? SizedBox(
                          height: 20,
                          width: 68,
                          child: TextField(
                            controller: _textController,
                            focusNode: _focusNode,
                            textAlign: TextAlign.center,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 10,
                              fontWeight: FontWeight.bold,
                            ),
                            decoration: InputDecoration(
                              isDense: true,
                              contentPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                              filled: true,
                              fillColor: const Color(0xFF10141C),
                              border: OutlineInputBorder(
                                borderRadius: BorderRadius.circular(4),
                                borderSide: BorderSide(color: widget.activeColor, width: 1.2),
                              ),
                              focusedBorder: OutlineInputBorder(
                                borderRadius: BorderRadius.circular(4),
                                borderSide: BorderSide(color: widget.activeColor, width: 1.5),
                              ),
                            ),
                            onSubmitted: (_) => _commitText(),
                          ),
                        )
                      : GestureDetector(
                          onDoubleTap: widget.isEnabled
                              ? () {
                                  setState(() {
                                    _isEditing = true;
                                    _textController.text = _formatEditValue(widget.value);
                                    _textController.selection = TextSelection(
                                      baseOffset: 0,
                                      extentOffset: _textController.text.length,
                                    );
                                  });
                                  Future.delayed(const Duration(milliseconds: 50), () {
                                    if (mounted) _focusNode.requestFocus();
                                  });
                                }
                              : null,
                          child: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                            decoration: BoxDecoration(
                              color: Colors.black45,
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: _isHovered && widget.isEnabled
                                    ? widget.activeColor.withValues(alpha: 0.7)
                                    : Colors.white12,
                                width: 1,
                              ),
                            ),
                            child: Text(
                              _formatDisplayValue(widget.value),
                              style: const TextStyle(
                                fontSize: 10,
                                fontWeight: FontWeight.bold,
                                color: Colors.white,
                              ),
                            ),
                          ),
                        ),
                  const SizedBox(height: 2),

                  // 4. 최하단 메인 라벨
                  Text(
                    widget.title,
                    style: const TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.bold,
                      color: Colors.white,
                      letterSpacing: 0.5,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _AbletonKnobPainter extends CustomPainter {
  final double norm;
  final bool isBipolar;
  final bool isQKnob;
  final Color activeColor;
  final bool isHovered;

  _AbletonKnobPainter({
    required this.norm,
    required this.isBipolar,
    this.isQKnob = false,
    required this.activeColor,
    required this.isHovered,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final outerRadius = size.width / 2 - 2.0;
    final knobRadius = outerRadius - 4.5;

    // Track arc: 0.75 * pi (135 deg / 7:30) to 2.25 * pi (45 deg / 4:30), total span 1.5 * pi (270 deg)
    const double startAngle = 0.75 * math.pi;
    const double totalSweep = 1.5 * math.pi;

    // Background track arc
    final trackPaint = Paint()
      ..color = const Color(0xFF28313F)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.5
      ..strokeCap = StrokeCap.round;

    final arcRect = Rect.fromCircle(center: center, radius: outerRadius);
    canvas.drawArc(arcRect, startAngle, totalSweep, false, trackPaint);

    // Active track arc
    final activePaint = Paint()
      ..color = activeColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.5
      ..strokeCap = StrokeCap.round;

    if (!isBipolar) {
      final sweep = (norm * totalSweep).clamp(0.001, totalSweep);
      canvas.drawArc(arcRect, startAngle, sweep, false, activePaint);
    } else {
      // 0 dB is at 12 o'clock (1.5 * pi)
      const double centerAngle = 1.5 * math.pi;
      final sweep = (norm - 0.5) * totalSweep;
      if (sweep.abs() > 0.001) {
        canvas.drawArc(arcRect, centerAngle, sweep, false, activePaint);
      }
    }

    // Tick marker at 12 o'clock for Bipolar (0 dB) or Q (1.0)
    if (isBipolar || isQKnob) {
      final tickTop = center + const Offset(0, -1) * outerRadius;
      final tickBottom = center + const Offset(0, -1) * (outerRadius - 3.0);
      canvas.drawLine(
        tickTop,
        tickBottom,
        Paint()
          ..color = Colors.white54
          ..strokeWidth = 1.5,
      );
    }

    // Dial circle body
    final dialFill = Paint()
      ..shader = RadialGradient(
        colors: isHovered
            ? [const Color(0xFF333B4A), const Color(0xFF1E242E)]
            : [const Color(0xFF282F3B), const Color(0xFF181D25)],
        center: const Alignment(-0.2, -0.2),
      ).createShader(Rect.fromCircle(center: center, radius: knobRadius));

    canvas.drawCircle(center, knobRadius, dialFill);

    final dialBorder = Paint()
      ..color = isHovered ? const Color(0xFF55647C) : const Color(0xFF3A4555)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2;
    canvas.drawCircle(center, knobRadius, dialBorder);

    // Pointer line
    final currentAngle = startAngle + norm * totalSweep;
    final innerP = center + Offset(math.cos(currentAngle), math.sin(currentAngle)) * (knobRadius * 0.25);
    final outerP = center + Offset(math.cos(currentAngle), math.sin(currentAngle)) * (knobRadius - 2.0);

    final pointerPaint = Paint()
      ..color = Colors.white
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0
      ..strokeCap = StrokeCap.round;

    canvas.drawLine(innerP, outerP, pointerPaint);
  }

  @override
  bool shouldRepaint(covariant _AbletonKnobPainter oldDelegate) {
    return oldDelegate.norm != norm ||
        oldDelegate.isBipolar != isBipolar ||
        oldDelegate.isQKnob != isQKnob ||
        oldDelegate.activeColor != activeColor ||
        oldDelegate.isHovered != isHovered;
  }
}
