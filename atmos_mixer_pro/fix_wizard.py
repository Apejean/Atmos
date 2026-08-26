import re

with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'r') as f:
    content = f.read()

# 1. TickerProvider
content = content.replace("SingleTickerTickerProviderStateMixin", "SingleTickerProviderStateMixin")

# 2. RoomZoneModel -> RoomZone
content = content.replace("RoomZoneModel", "RoomZone")

# 3. unchecked_use_of_nullable_value - DropdownMenuItem value
# r is RoomZone, the value should be typed correctly, map((r) => DropdownMenuItem(value: r.id, child: Text(r.name)))
content = content.replace("rooms.map((r) => DropdownMenuItem(value: r.id, child: Text(r.name))).toList()", "rooms.map<DropdownMenuItem<String>>((r) => DropdownMenuItem(value: r.id, child: Text(r.name))).toList()")

# 4. withOpacity -> withValues(alpha: ...)
content = content.replace("AppColors.primaryNeon.withOpacity(0.5)", "AppColors.primaryNeon.withValues(alpha: 0.5)")
content = content.replace("Colors.redAccent.withOpacity(0.7)", "Colors.redAccent.withValues(alpha: 0.7)")

# 5. unused pct
content = re.sub(r'\s*double pct = x / size\.width;', '', content)


with open('lib/features/dashboard/widgets/room_calibration_wizard_modal.dart', 'w') as f:
    f.write(content)
