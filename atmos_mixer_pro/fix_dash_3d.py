import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("ch.channel == node.channel", "ch.id == (node.channel + 1)")
content = content.replace("OutputChannelModel(channel: node.channel)", "OutputChannelModel(id: node.channel + 1, name: 'Speaker ${node.channel + 1}')")
content = content.replace("gain: gainDb.clamp(-60.0, 12.0),", "gainDb: gainDb.clamp(-60.0, 12.0),")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
