import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# Replace RoomCalibrationWizardModal with OutputRoutingMatrixModal
content = content.replace("import 'package:atmos_mixer_pro/features/dashboard/widgets/room_calibration_wizard_modal.dart';", "import 'package:atmos_mixer_pro/features/dashboard/widgets/output_routing_matrix_modal.dart';")
content = content.replace("RoomCalibrationWizardModal()", "OutputRoutingMatrixModal()")
content = content.replace("'Room Calibration'", "'Output Routing'")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
