class ChannelDropdownValueHelper {
  static String getMonoValue(int channel) => 'mono_$channel';
  static String getStereoValue(int channel) => 'stereo_$channel';
  static String getMultiValue(int channel) => 'multi_$channel';

  static bool isMono(String value) => value.startsWith('mono_');
  static bool isStereo(String value) => value.startsWith('stereo_');
  static bool isMulti(String value) => value.startsWith('multi_');

  static int? getChannel(String value) {
    final parts = value.split('_');
    if (parts.length >= 2) {
      return int.tryParse(parts[1]);
    }
    return null;
  }
}
