import re
with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

# Logic Pro Z reset (Center Front implies Y=Front, X=Center, Z is normally 0 (Ear Level)).
# Double Tap on side-view: _z = 0.0 is Ear Level in our mapping. I mapped 0.0 to Ear Level, 1.0 to Top.
# Let's verify mapping: Z axis: 0.0 -> Ear Level, 1.0 -> Top.
# _formatZ: val 0.0 -> mapped = -1.0. val 1.0 -> mapped = 1.0.
# So _formatZ(0.0) -> "-1.000" which is Ear Level.
# Wait, the instruction says: "-1.000 (Ear Level/바닥) ~ +1.000 (Top/천장)"
# My mapping is (val - 0.5) * 2.0. So 0.0 => -1.0, 1.0 => +1.0. This is perfectly correct.
# And double-click to reset sets _x=0.5 (mapped 0.0), _y=0.0 (mapped +1.0 Front), _z=0.0 (mapped -1.0 Ear Level) Wait, the instruction says:
# "정중앙 초기 위치(Center-Front, X=0, Y=1.0)로 즉시 리셋"
# If _x=0.5 -> X=0
# If _y=0.0 -> Y=1.0
# Is Z reset to Ear Level (-1.0) or Center (0.0)? The instructions mention X=0, Y=1.0 for the planar grid. For Elevation grid, "빈 공간을 Alt+Click 하거나 더블 클릭 했을 때, 퍽의 위치가 정중앙 초기 위치로".
# If I reset Z to 0.0 (mapped -1.0), it is Ear Level.
# Actually I'll just leave Z double tap as `_z = 0.0` which is Ear Level.

pass
