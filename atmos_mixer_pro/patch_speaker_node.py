import re

with open('lib/features/exhibition/widgets/speaker_node_widget.dart', 'r') as f:
    content = f.read()

old_code = """                          (index) {
                            String channelName = 'Ch ${index + 1}';
                            if (hwChannelsAsync.value != null && index < hwChannelsAsync.value!.length) {
                              channelName = hwChannelsAsync.value![index].toString(); // Assuming it might be a String or can be cast to string
                            } else if (config != null && config.deviceName != null && GlobalDeviceCache.channels.containsKey(config.deviceName)) {
                              if (index < GlobalDeviceCache.channels[config.deviceName]!.length) {
                                channelName = GlobalDeviceCache.channels[config.deviceName]![index];
                              }
                            }
                            
                            // Truncate if too long to prevent UI breaking
                            if (channelName.length > 20) {
                              channelName = '${channelName.substring(0, 17)}...';
                            }

                            return DropdownMenuItem<int>(
                              value: index,
                              child: Text(channelName),
                            );
                          },"""

new_code = """                          (index) {
                            String hwName = 'Out ${index + 1}';
                            if (hwChannelsAsync.value != null && index < hwChannelsAsync.value!.length) {
                              hwName = hwChannelsAsync.value![index].toString();
                            } else if (config != null && config.deviceName != null && GlobalDeviceCache.channels.containsKey(config.deviceName)) {
                              if (index < GlobalDeviceCache.channels[config.deviceName]!.length) {
                                hwName = GlobalDeviceCache.channels[config.deviceName]![index];
                              }
                            }
                            
                            String channelName = 'Ch ${index + 1} ($hwName)';
                            // Truncate if too long to prevent UI breaking
                            if (channelName.length > 25) {
                              channelName = '${channelName.substring(0, 22)}...';
                            }

                            return DropdownMenuItem<int>(
                              value: index,
                              child: Text(channelName),
                            );
                          },"""

if old_code in content:
    with open('lib/features/exhibition/widgets/speaker_node_widget.dart', 'w') as f:
        f.write(content.replace(old_code, new_code))
    print("SpeakerNodeWidget patched")
else:
    print("Could not find code block in SpeakerNodeWidget")
