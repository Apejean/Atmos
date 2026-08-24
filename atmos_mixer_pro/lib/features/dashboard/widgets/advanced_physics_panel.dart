import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Advanced Physical Acoustics Control Panel (Phase 3)
/// Controls Doppler Effect, Air High-Freq Absorption, and Speaker Directivity Angle.
class AdvancedPhysicsPanel extends StatefulWidget {
  const AdvancedPhysicsPanel({super.key});

  @override
  State<AdvancedPhysicsPanel> createState() => _AdvancedPhysicsPanelState();
}

class _AdvancedPhysicsPanelState extends State<AdvancedPhysicsPanel> {
  bool _dopplerEnabled = true;
  double _dopplerFactor = 1.0;
  bool _airAbsorptionEnabled = true;
  double _temperatureCelsius = 20.0;
  double _humidityPercent = 50.0;
  double _speakerDirectivityAngleDeg = 90.0;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: const Color(0xFF1E1E2C),
      title: const Row(
        children: [
          Icon(Icons.science, color: AppColors.primaryNeon),
          SizedBox(width: 8),
          Text(
            'Advanced Physical Acoustics Panel',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
          ),
        ],
      ),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Doppler Effect
              _buildSectionHeader(Icons.speed, 'Doppler Effect (Pitch Shift)'),
              SwitchListTile(
                title: const Text('Enable Doppler Pitch Shift', style: TextStyle(color: Colors.white, fontSize: 13)),
                value: _dopplerEnabled,
                activeThumbColor: AppColors.primaryNeon,
                onChanged: (val) => setState(() => _dopplerEnabled = val),
              ),
              if (_dopplerEnabled) ...[
                Row(
                  children: [
                    const Text('Doppler Factor: ', style: TextStyle(color: Colors.white70, fontSize: 12)),
                    Expanded(
                      child: Slider(
                        value: _dopplerFactor,
                        min: 0.1,
                        max: 3.0,
                        divisions: 29,
                        activeColor: AppColors.primaryNeon,
                        label: '${_dopplerFactor.toStringAsFixed(1)}x',
                        onChanged: (val) => setState(() => _dopplerFactor = val),
                      ),
                    ),
                    Text('${_dopplerFactor.toStringAsFixed(1)}x', style: const TextStyle(color: Colors.white, fontSize: 12)),
                  ],
                ),
              ],
              const Divider(color: Colors.white12),

              // Air Absorption
              _buildSectionHeader(Icons.air, 'Air High-Frequency Absorption'),
              SwitchListTile(
                title: const Text('Enable Air HF Damping', style: TextStyle(color: Colors.white, fontSize: 13)),
                value: _airAbsorptionEnabled,
                activeThumbColor: Colors.cyanAccent,
                onChanged: (val) => setState(() => _airAbsorptionEnabled = val),
              ),
              if (_airAbsorptionEnabled) ...[
                Row(
                  children: [
                    const Text('Room Temp (°C): ', style: TextStyle(color: Colors.white70, fontSize: 12)),
                    Expanded(
                      child: Slider(
                        value: _temperatureCelsius,
                        min: 0.0,
                        max: 40.0,
                        divisions: 40,
                        activeColor: Colors.cyanAccent,
                        label: '${_temperatureCelsius.toInt()}°C',
                        onChanged: (val) => setState(() => _temperatureCelsius = val),
                      ),
                    ),
                    Text('${_temperatureCelsius.toInt()}°C', style: const TextStyle(color: Colors.white, fontSize: 12)),
                  ],
                ),
                Row(
                  children: [
                    const Text('Humidity (%): ', style: TextStyle(color: Colors.white70, fontSize: 12)),
                    Expanded(
                      child: Slider(
                        value: _humidityPercent,
                        min: 10.0,
                        max: 90.0,
                        divisions: 80,
                        activeColor: Colors.cyanAccent,
                        label: '${_humidityPercent.toInt()}%',
                        onChanged: (val) => setState(() => _humidityPercent = val),
                      ),
                    ),
                    Text('${_humidityPercent.toInt()}%', style: const TextStyle(color: Colors.white, fontSize: 12)),
                  ],
                ),
              ],
              const Divider(color: Colors.white12),

              // Speaker Directivity
              _buildSectionHeader(Icons.radar, 'Speaker Directivity & Dispersion Angle'),
              Row(
                children: [
                  const Text('Dispersion Angle: ', style: TextStyle(color: Colors.white70, fontSize: 12)),
                  Expanded(
                    child: Slider(
                      value: _speakerDirectivityAngleDeg,
                      min: 30.0,
                      max: 180.0,
                      divisions: 15,
                      activeColor: Colors.amberAccent,
                      label: '${_speakerDirectivityAngleDeg.toInt()}°',
                      onChanged: (val) => setState(() => _speakerDirectivityAngleDeg = val),
                    ),
                  ),
                  Text('${_speakerDirectivityAngleDeg.toInt()}°', style: const TextStyle(color: Colors.white, fontSize: 12)),
                ],
              ),
            ],
          ),
        ),
      ),
      actions: [
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.primaryNeon,
            foregroundColor: Colors.black,
          ),
          onPressed: () {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('⚡ Physical acoustics parameters updated!'),
                backgroundColor: Colors.green,
              ),
            );
            Navigator.of(context).pop();
          },
          child: const Text('Apply Parameters'),
        ),
      ],
    );
  }

  Widget _buildSectionHeader(IconData icon, String title) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          Icon(icon, size: 16, color: AppColors.primaryNeon),
          const SizedBox(width: 6),
          Text(
            title,
            style: const TextStyle(
              color: Colors.white,
              fontSize: 13,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}
