import re

with open("lib/features/dashboard/widgets/room_calibration_wizard_modal.dart", "r") as f:
    content = f.read()

# Add a close button to the title row
old_title = "title: const Text('Room Auto Calibration', style: TextStyle(color: Colors.white)),"
new_title = """title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Text('Room Auto Calibration', style: TextStyle(color: Colors.white)),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white54),
            onPressed: () => Navigator.of(context).pop(),
            splashRadius: 20,
          ),
        ],
      ),"""

content = content.replace(old_title, new_title)

# Also add a Cancel button alongside the Stepper controls if it's step 0
# Actually, the Stepper handles 'Back' but maybe 'Cancel' is better on step 0
old_controls = """                  if (_currentStep < 3)
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(backgroundColor: AppColors.primaryNeon),
                      onPressed: details.onStepContinue,
                      child: Text(_currentStep == 0 ? '측정 시작' : '다음'),
                    )
                  else
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(backgroundColor: AppColors.success),
                      onPressed: details.onStepContinue,
                      child: const Text('💾 시스템 적용 (Apply)'),
                    ),
                  if (_currentStep > 0) ...[
                    const SizedBox(width: 8),
                    TextButton(
                      onPressed: details.onStepCancel,
                      child: const Text('이전', style: TextStyle(color: Colors.white54)),
                    ),
                  ],"""

new_controls = """                  if (_currentStep < 3)
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(backgroundColor: AppColors.primaryNeon),
                      onPressed: details.onStepContinue,
                      child: Text(_currentStep == 0 ? '측정 시작' : '다음'),
                    )
                  else
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(backgroundColor: AppColors.success),
                      onPressed: details.onStepContinue,
                      child: const Text('💾 시스템 적용 (Apply)'),
                    ),
                  if (_currentStep > 0) ...[
                    const SizedBox(width: 8),
                    TextButton(
                      onPressed: details.onStepCancel,
                      child: const Text('이전', style: TextStyle(color: Colors.white54)),
                    ),
                  ] else ...[
                    const SizedBox(width: 8),
                    TextButton(
                      onPressed: () => Navigator.of(context).pop(),
                      child: const Text('취소', style: TextStyle(color: Colors.white54)),
                    ),
                  ],"""
content = content.replace(old_controls, new_controls)

with open("lib/features/dashboard/widgets/room_calibration_wizard_modal.dart", "w") as f:
    f.write(content)
