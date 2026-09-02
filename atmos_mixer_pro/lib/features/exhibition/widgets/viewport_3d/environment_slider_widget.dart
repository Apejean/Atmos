import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';

class EnvironmentSliderWidget extends ConsumerWidget {
  const EnvironmentSliderWidget({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final env = ref.watch(environmentStateProvider);

    return PopupMenuButton<String>(
      offset: const Offset(0, 40),
      color: const Color(0xFF1B232E),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.3)),
      ),
      tooltip: 'Acoustic Environment',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: Colors.black.withValues(alpha: 0.4),
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: Colors.orangeAccent.withValues(alpha: 0.3)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.thermostat_rounded, size: 14, color: Colors.orangeAccent),
            const SizedBox(width: 4),
            Text(
              '${env.temperature.toStringAsFixed(1)}°C',
              style: const TextStyle(
                color: Colors.orangeAccent,
                fontSize: 11,
                fontWeight: FontWeight.bold,
              ),
            ),
          ],
        ),
      ),
      itemBuilder: (context) => [
        PopupMenuItem<String>(
          enabled: false,
          padding: EdgeInsets.zero,
          child: const _EnvironmentPopupContent(),
        ),
      ],
    );
  }
}

class _EnvironmentPopupContent extends ConsumerStatefulWidget {
  const _EnvironmentPopupContent();

  @override
  ConsumerState<_EnvironmentPopupContent> createState() => _EnvironmentPopupContentState();
}

class _EnvironmentPopupContentState extends ConsumerState<_EnvironmentPopupContent> {
  bool _isEditingTemp = false;
  bool _isEditingHum = false;
  late TextEditingController _tempCtrl;
  late TextEditingController _humCtrl;

  @override
  void initState() {
    super.initState();
    final env = ref.read(environmentStateProvider);
    _tempCtrl = TextEditingController(text: env.temperature.toStringAsFixed(1));
    _humCtrl = TextEditingController(text: env.humidity.toStringAsFixed(1));
  }

  @override
  void dispose() {
    _tempCtrl.dispose();
    _humCtrl.dispose();
    super.dispose();
  }

  void _submitTemp() {
    final val = double.tryParse(_tempCtrl.text);
    if (val != null) {
      ref.read(environmentStateProvider.notifier).setTemperature(val.clamp(-10.0, 40.0));
    }
    setState(() {
      _isEditingTemp = false;
    });
  }

  void _submitHum() {
    final val = double.tryParse(_humCtrl.text);
    if (val != null) {
      ref.read(environmentStateProvider.notifier).setHumidity(val.clamp(0.0, 100.0));
    }
    setState(() {
      _isEditingHum = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final env = ref.watch(environmentStateProvider);

    return Container(
      width: 240,
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                'Temperature (°C)',
                style: TextStyle(color: Colors.white70, fontSize: 12),
              ),
              _isEditingTemp
                  ? SizedBox(
                      width: 50,
                      height: 20,
                      child: TextField(
                        controller: _tempCtrl,
                        style: const TextStyle(color: Colors.orangeAccent, fontSize: 12),
                        textAlign: TextAlign.right,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(
                          isDense: true,
                          contentPadding: EdgeInsets.zero,
                          border: InputBorder.none,
                        ),
                        onSubmitted: (_) => _submitTemp(),
                        onTapOutside: (_) => _submitTemp(),
                      ),
                    )
                  : GestureDetector(
                      onDoubleTap: () {
                        _tempCtrl.text = env.temperature.toStringAsFixed(1);
                        setState(() {
                          _isEditingTemp = true;
                        });
                      },
                      child: Text(
                        '${env.temperature.toStringAsFixed(1)} °C',
                        style: const TextStyle(color: Colors.orangeAccent, fontSize: 12, fontWeight: FontWeight.bold),
                      ),
                    ),
            ],
          ),
          SliderTheme(
            data: SliderTheme.of(context).copyWith(
              activeTrackColor: Colors.orangeAccent,
              thumbColor: Colors.orangeAccent,
              trackHeight: 2.0,
            ),
            child: Slider(
              value: env.temperature,
              min: -10.0,
              max: 40.0,
              onChanged: (val) {
                ref.read(environmentStateProvider.notifier).setTemperature(val);
              },
            ),
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                'Humidity (%)',
                style: TextStyle(color: Colors.white70, fontSize: 12),
              ),
              _isEditingHum
                  ? SizedBox(
                      width: 50,
                      height: 20,
                      child: TextField(
                        controller: _humCtrl,
                        style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 12),
                        textAlign: TextAlign.right,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(
                          isDense: true,
                          contentPadding: EdgeInsets.zero,
                          border: InputBorder.none,
                        ),
                        onSubmitted: (_) => _submitHum(),
                        onTapOutside: (_) => _submitHum(),
                      ),
                    )
                  : GestureDetector(
                      onDoubleTap: () {
                        _humCtrl.text = env.humidity.toStringAsFixed(1);
                        setState(() {
                          _isEditingHum = true;
                        });
                      },
                      child: Text(
                        '${env.humidity.toStringAsFixed(1)} %',
                        style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 12, fontWeight: FontWeight.bold),
                      ),
                    ),
            ],
          ),
          SliderTheme(
            data: SliderTheme.of(context).copyWith(
              activeTrackColor: Colors.lightBlueAccent,
              thumbColor: Colors.lightBlueAccent,
              trackHeight: 2.0,
            ),
            child: Slider(
              value: env.humidity,
              min: 0.0,
              max: 100.0,
              onChanged: (val) {
                ref.read(environmentStateProvider.notifier).setHumidity(val);
              },
            ),
          ),
          const SizedBox(height: 8),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            decoration: BoxDecoration(
              color: Colors.black.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: Colors.greenAccent.withValues(alpha: 0.2)),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('Speed of Sound', style: TextStyle(color: Colors.white70, fontSize: 11)),
                Text(
                  '${env.speedOfSound.toStringAsFixed(2)} m/s',
                  style: const TextStyle(
                    color: Colors.greenAccent,
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
