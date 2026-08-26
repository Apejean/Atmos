import re

with open('lib/features/dashboard/state/output_routing_state.dart', 'r') as f:
    content = f.read()

# Add EqBand to OutputChannelModel
if "List<Map<String, dynamic>> eqBands;" not in content:
    content = content.replace("final double gainDb;",
"""final double gainDb;
  final List<Map<String, dynamic>> eqBands;""")

    content = content.replace("this.gainDb = 0.0,",
"""this.gainDb = 0.0,
    this.eqBands = const [],""")

    content = content.replace("'gainDb': gainDb,",
"""'gainDb': gainDb,
      'eqBands': eqBands,""")

    content = content.replace("double? gainDb,",
"""double? gainDb,
    List<Map<String, dynamic>>? eqBands,""")

    content = content.replace("gainDb: gainDb ?? this.gainDb,",
"""gainDb: gainDb ?? this.gainDb,
      eqBands: eqBands ?? this.eqBands,""")

with open('lib/features/dashboard/state/output_routing_state.dart', 'w') as f:
    f.write(content)
