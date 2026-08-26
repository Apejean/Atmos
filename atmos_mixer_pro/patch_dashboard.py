import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'package:atmos_mixer_pro/features/dashboard/widgets/output_routing_matrix_modal.dart';",
"""import 'package:atmos_mixer_pro/features/dashboard/widgets/output_calibration_modal.dart';""")

content = content.replace("OutputRoutingMatrixModal()", "OutputCalibrationModal()")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
