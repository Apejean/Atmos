import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# I need to add [ Room Calibration ] button near the Wrap of buttons in _buildHeader
old_code = """          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: [
              ElevatedButton("""

new_code = """          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: [
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                onPressed: () {
                  showDialog(
                    context: context,
                    builder: (context) => const RoomCalibrationWizardModal(),
                  );
                },
                child: const Text('Room Calibration', style: TextStyle(color: Colors.black)),
              ),
              ElevatedButton("""

if old_code in content:
    content = content.replace(old_code, new_code)
    
    # add import
    import_stmt = "import 'package:atmos_mixer_pro/features/dashboard/widgets/room_calibration_wizard_modal.dart';\n"
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\n" + import_stmt)
    
    with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
        f.write(content)
    print("Dashboard patched successfully")
else:
    print("Failed to find old code in dashboard_screen.dart")

