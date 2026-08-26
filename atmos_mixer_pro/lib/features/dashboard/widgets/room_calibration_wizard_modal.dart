import 'package:flutter/material.dart';
import 'dart:math' as math;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

class RoomCalibrationWizardModal extends ConsumerStatefulWidget {
  const RoomCalibrationWizardModal({super.key});

  @override
  ConsumerState<RoomCalibrationWizardModal> createState() => _RoomCalibrationWizardModalState();
}

class _RoomCalibrationWizardModalState extends ConsumerState<RoomCalibrationWizardModal> with SingleTickerProviderStateMixin {
  int _currentStep = 0;
  
  // Step 1 State
  String? _selectedRoomId;
  int _selectedMicChannelChannel = 1;
  
  // Step 2 State
  double _measureProgress = 0.0;
  late AnimationController _radarController;
  
  // Step 3 State
  String _targetCurve = 'Flat (Studio Reference)';

  @override
  void initState() {
    super.initState();
    _radarController = AnimationController(vsync: this, duration: const Duration(seconds: 2));
  }

  @override
  void dispose() {
    _radarController.dispose();
    super.dispose();
  }

  void _nextStep() {
    if (_currentStep == 1) {
      // Start fake measurement
      _radarController.repeat();
      _fakeMeasurement();
    }
    setState(() {
      _currentStep++;
    });
  }

  void _prevStep() {
    if (_currentStep == 1) {
      _radarController.stop();
    }
    setState(() {
      _currentStep--;
    });
  }

  Future<void> _fakeMeasurement() async {
    _measureProgress = 0.0;
    while (_measureProgress < 1.0 && _currentStep == 1) {
      await Future.delayed(const Duration(milliseconds: 100));
      if (!mounted) return;
      setState(() {
        _measureProgress += 0.02;
        if (_measureProgress > 1.0) _measureProgress = 1.0;
      });
    }
    if (_currentStep == 1 && mounted) {
      _radarController.stop();
      _nextStep();
    }
  }

  Widget _buildStep1(List<RoomZone> rooms) {
    if (_selectedRoomId == null && rooms.isNotEmpty) {
      _selectedRoomId = rooms.first.id;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Target Room Zone', style: TextStyle(color: Colors.white70)),
        const SizedBox(height: 8),
        DropdownButton<String>(
          isExpanded: true,
          value: _selectedRoomId,
          dropdownColor: AppColors.cardSurfaceSolid,
          style: const TextStyle(color: Colors.white),
          items: rooms.map<DropdownMenuItem<String>>((r) => DropdownMenuItem(value: r.id, child: Text(r.label))).toList(),
          onChanged: (v) => setState(() => _selectedRoomId = v),
        ),
        const SizedBox(height: 24),
        const Text('측정 마이크 입력 채널 (Input Channel)', style: TextStyle(color: Colors.white70)),
        const SizedBox(height: 8),
        DropdownButton<String>(
          isExpanded: true,
          value: _selectedMicChannelChannel,
          dropdownColor: AppColors.cardSurfaceSolid,
          style: const TextStyle(color: Colors.white),
          items: List.generate(32, (index) => index + 1).map((ch) => DropdownMenuItem(value: ch, child: Text('Input Ch $ch'))).toList(),
          onChanged: (v) => setState(() => _selectedMicChannelChannel = v!),
        ),
      ],
    );
  }

  Widget _buildStep2() {
    return Column(
      children: [
        const Text('Measuring Room Acoustics...', style: TextStyle(color: Colors.white, fontSize: 18)),
        const SizedBox(height: 32),
        SizedBox(
          height: 200,
          child: Stack(
            alignment: Alignment.center,
            children: [
              RotationTransition(
                turns: _radarController,
                child: Container(
                  width: 150,
                  height: 150,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    gradient: SweepGradient(
                      colors: [Colors.transparent, AppColors.primaryNeon.withValues(alpha: 0.5)],
                    ),
                  ),
                ),
              ),
              const Icon(Icons.mic, size: 48, color: Colors.white),
            ],
          ),
        ),
        const SizedBox(height: 32),
        LinearProgressIndicator(
          value: _measureProgress,
          backgroundColor: Colors.white12,
          valueColor: const AlwaysStoppedAnimation<Color>(AppColors.primaryNeon),
        ),
        const SizedBox(height: 8),
        Text('${(_measureProgress * 100).toInt()}%', style: const TextStyle(color: AppColors.primaryNeon)),
      ],
    );
  }

  Widget _buildStep3() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Target Curve Preset', style: TextStyle(color: Colors.white70)),
        const SizedBox(height: 8),
        DropdownButton<String>(
          isExpanded: true,
          value: _targetCurve,
          dropdownColor: AppColors.cardSurfaceSolid,
          style: const TextStyle(color: Colors.white),
          items: ['Flat (Studio Reference)', 'X-Curve (Cinema)', 'Bass Boost (Club)'].map((p) => DropdownMenuItem(value: p, child: Text(p))).toList(),
          onChanged: (v) => setState(() => _targetCurve = v!),
        ),
        const SizedBox(height: 24),
        const Text('Frequency Response Viewer (Mock)', style: TextStyle(color: Colors.white70)),
        const SizedBox(height: 16),
        Container(
          height: 200,
          width: double.infinity,
          decoration: BoxDecoration(
            color: Colors.black45,
            border: Border.all(color: Colors.white24),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Stack(
            children: [
              // Mock grid
              CustomPaint(painter: MockGridPainter()),
              // Mock lines
              CustomPaint(painter: MockFRPainter()),
              const Positioned(
                top: 8,
                right: 8,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('--- Before', style: TextStyle(color: Colors.redAccent, fontSize: 10)),
                    Text('--- Target', style: TextStyle(color: Colors.white70, fontSize: 10)),
                    Text('--- After (Predicted)', style: TextStyle(color: Colors.greenAccent, fontSize: 10)),
                  ],
                ),
              )
            ],
          ),
        )
      ],
    );
  }

  Widget _buildStep4() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Calibration Results (Mock)', style: TextStyle(color: Colors.white, fontSize: 16)),
        const SizedBox(height: 16),
        Container(
          height: 250,
          decoration: BoxDecoration(
            color: Colors.black45,
            borderRadius: BorderRadius.circular(8),
          ),
          child: SingleChildScrollView(
            child: DataTable(
              headingTextStyle: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              dataTextStyle: const TextStyle(color: Colors.white70),
              columns: const [
                DataColumn(label: Text('Channel')),
                DataColumn(label: Text('Delay (ms)')),
                DataColumn(label: Text('Gain (dB)')),
              ],
              rows: List.generate(8, (i) => DataRow(cells: [
                DataCell(Text('Ch ${i+1}')),
                DataCell(Text((1.5 + i * 0.2).toStringAsFixed(2))),
                DataCell(Text((0.0 - i * 0.5).toStringAsFixed(2))),
              ])),
            ),
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final rooms = ref.watch(roomZoneProvider);

    return AlertDialog(
      backgroundColor: AppColors.background,
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Text('Room Auto Calibration', style: TextStyle(color: Colors.white)),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white54),
            onPressed: () => Navigator.of(context).pop(),
            splashRadius: 20,
          ),
        ],
      ),
      content: SizedBox(
        width: 600,
        height: 400,
        child: Stepper(
          currentStep: _currentStep,
          type: StepperType.horizontal,
          onStepCancel: _currentStep > 0 ? _prevStep : null,
          onStepContinue: _currentStep < 3 ? _nextStep : () {
            Navigator.of(context).pop();
            ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Calibration applied successfully!')));
          },
          controlsBuilder: (context, details) {
            if (_currentStep == 1) return const SizedBox.shrink(); // No controls during measuring
            return Padding(
              padding: const EdgeInsets.only(top: 16.0),
              child: Row(
                children: [
                  if (_currentStep < 3)
                    ElevatedButton(
                      onPressed: details.onStepContinue,
                      style: ElevatedButton.styleFrom(backgroundColor: AppColors.primaryNeon),
                      child: const Text('Next', style: TextStyle(color: Colors.black)),
                    )
                  else
                    ElevatedButton(
                      onPressed: details.onStepContinue,
                      style: ElevatedButton.styleFrom(backgroundColor: Colors.greenAccent),
                      child: const Text('Apply', style: TextStyle(color: Colors.black)),
                    ),
                  const SizedBox(width: 16),
                  if (_currentStep > 0 && _currentStep != 1)
                    TextButton(
                      onPressed: details.onStepCancel,
                      child: const Text('Back', style: TextStyle(color: Colors.white70)),
                    ),
                ],
              ),
            );
          },
          steps: [
            Step(
              title: const Text('Setup', style: TextStyle(color: Colors.white)),
              content: _buildStep1(rooms),
              isActive: _currentStep >= 0,
              state: _currentStep > 0 ? StepState.complete : StepState.indexed,
            ),
            Step(
              title: const Text('Measure', style: TextStyle(color: Colors.white)),
              content: _buildStep2(),
              isActive: _currentStep >= 1,
              state: _currentStep > 1 ? StepState.complete : StepState.indexed,
            ),
            Step(
              title: const Text('Target', style: TextStyle(color: Colors.white)),
              content: _buildStep3(),
              isActive: _currentStep >= 2,
              state: _currentStep > 2 ? StepState.complete : StepState.indexed,
            ),
            Step(
              title: const Text('Result', style: TextStyle(color: Colors.white)),
              content: _buildStep4(),
              isActive: _currentStep >= 3,
            ),
          ],
        ),
      ),
    );
  }
}

class MockGridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = Colors.white10..strokeWidth = 1;
    for (double i = 0; i < size.width; i += 50) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);
    }
    for (double i = 0; i < size.height; i += 50) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class MockFRPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final beforePaint = Paint()..color = Colors.redAccent.withValues(alpha: 0.7)..strokeWidth = 2..style = PaintingStyle.stroke;
    final targetPaint = Paint()..color = Colors.white54..strokeWidth = 2..style = PaintingStyle.stroke;
    final afterPaint = Paint()..color = Colors.greenAccent..strokeWidth = 2..style = PaintingStyle.stroke;

    final beforePath = Path();
    final targetPath = Path();
    final afterPath = Path();

    beforePath.moveTo(0, size.height * 0.5);
    targetPath.moveTo(0, size.height * 0.4);
    afterPath.moveTo(0, size.height * 0.4);

    for (double x = 10; x <= size.width; x += 10) {
      // target: flat line
      targetPath.lineTo(x, size.height * 0.4);
      // before: bumpy
      double beforeY = size.height * 0.4 + math.sin(x * 0.05) * 30 + math.cos(x * 0.1) * 15;
      beforePath.lineTo(x, beforeY);
      // after: smoother
      double afterY = size.height * 0.4 + math.sin(x * 0.05) * 5 + math.cos(x * 0.1) * 2;
      afterPath.lineTo(x, afterY);
    }
    
    canvas.drawPath(beforePath, beforePaint);
    canvas.drawPath(targetPath, targetPaint);
    canvas.drawPath(afterPath, afterPaint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
