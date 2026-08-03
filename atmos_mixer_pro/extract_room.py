import os

source_file = 'lib/features/exhibition/screens/speaker_canvas_screen.dart'
dest_file = 'lib/features/exhibition/widgets/room_zone_widget.dart'

with open(source_file, 'r') as f:
    lines = f.readlines()

new_lines = []
room_widget_lines = []
door_painter_lines = []

in_room_widget = False
in_door_painter = False

for line in lines:
    if line.startswith('class _DraggableRoomWidget extends ConsumerStatefulWidget {'):
        in_room_widget = True
        
    if line.startswith('class _CadDoorPainter extends CustomPainter {'):
        in_door_painter = True
        
    if in_room_widget:
        room_widget_lines.append(line)
        if line.startswith('}'): # end of _DraggableRoomWidgetState? Wait, there are two classes.
            # We can't simply check for `}` because classes have many methods.
            pass
    elif in_door_painter:
        door_painter_lines.append(line)
    else:
        new_lines.append(line)

# Wait, simple line parsing is dangerous for nested braces.
