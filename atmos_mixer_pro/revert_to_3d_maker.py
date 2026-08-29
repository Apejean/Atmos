import subprocess

# We need to get back Maker's 3D code since that was actually closer to what the Architect and CEO just agreed upon!
# I will use git checkout to restore the specific files from Maker's commit (48f0bd87e9dcfda43f694259878e4a9ae9549b7c) 
# and then adapt it to fit the newly requested UI logic.

subprocess.run(["git", "checkout", "48f0bd87e9dcfda43f694259878e4a9ae9549b7c", "--", "lib/features/exhibition/screens/speaker_canvas_screen.dart"])
subprocess.run(["git", "checkout", "48f0bd87e9dcfda43f694259878e4a9ae9549b7c", "--", "lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart"])
subprocess.run(["git", "checkout", "48f0bd87e9dcfda43f694259878e4a9ae9549b7c", "--", "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"])
subprocess.run(["git", "checkout", "48f0bd87e9dcfda43f694259878e4a9ae9549b7c", "--", "lib/features/exhibition/models/speaker_node.dart"])

