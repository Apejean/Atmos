import re

with open('lib/features/dashboard/state/output_routing_state.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'package:flutter_riverpod/flutter_riverpod.dart';", 
"""import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;""")

content = content.replace("""  OutputChannelModel copyWith({""",
"""  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'isMuted': isMuted,
      'isSoloed': isSoloed,
      'isPhaseInverted': isPhaseInverted,
      'delayMs': delayMs,
      'gainDb': gainDb,
    };
  }

  OutputChannelModel copyWith({""")

content = content.replace("""  void updateChannel(OutputChannelModel updated) {
    state = state.map((ch) => ch.id == updated.id ? updated : ch).toList();
  }""",
"""  void updateChannel(OutputChannelModel updated) {
    state = state.map((ch) => ch.id == updated.id ? updated : ch).toList();
    _syncToBackend();
  }

  void _syncToBackend() {
    final payload = {
      'channels': state.map((e) => e.toJson()).toList(),
    };
    final jsonString = jsonEncode(payload);
    rust_api.apiUpdateOutputRouting(jsonPayload: jsonString).catchError((e) {
      print('Failed to sync output routing: $e');
    });
  }""")

with open('lib/features/dashboard/state/output_routing_state.dart', 'w') as f:
    f.write(content)
