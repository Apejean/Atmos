import re

with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'r') as f:
    content = f.read()

import_stmt = "import 'dart:math' as math;\n"
content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\n" + import_stmt)

old_paint = """    for (double x = 0; x < size.width; x += 10) {
      // Fake wave
      import 'dart:math'; // Oh wait, I can't import inside method. I'll just use a simple zigzag or sine wave without math.
    }
    // Let's rewrite this part.
  }"""

new_paint = """    for (double x = 10; x <= size.width; x += 10) {
      double pct = x / size.width;
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
  }"""

content = content.replace(old_paint, new_paint)

# And fix DataRow cells
old_datarow = """                DataColumn(label: Text('Ch ${i+1}')),
                DataColumn(label: Text('${(1.5 + i * 0.2).toStringAsFixed(2)}')),
                DataColumn(label: Text('${(0.0 - i * 0.5).toStringAsFixed(2)}')),
              ] as List<DataCell>)),"""
new_datarow = """                DataCell(Text('Ch ${i+1}')),
                DataCell(Text((1.5 + i * 0.2).toStringAsFixed(2))),
                DataCell(Text((0.0 - i * 0.5).toStringAsFixed(2))),
              ])),"""
content = content.replace(old_datarow, new_datarow)

with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'w') as f:
    f.write(content)
