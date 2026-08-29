with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

bad = """              interactionPrompt: InteractionPrompt.none,
              innerModelViewerHtml: '<model-viewer scale="${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}"></model-viewer>',
            ),"""
            
good = """              interactionPrompt: InteractionPrompt.none,
              scale: '${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}',
            ),"""

content = content.replace(bad, good)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
