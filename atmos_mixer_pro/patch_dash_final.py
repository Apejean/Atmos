import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# Emojis replacements
content = content.replace("🎛 Atmos Mixer Pro", "Atmos Mixer Pro")
content = content.replace("🦆 스마트 더킹 작동중", "스마트 더킹 작동중")
content = content.replace("➕ 룸 추가", "Add Room")
content = content.replace("⚙️ Mixer", "Mixer")
content = content.replace("🎯 Apply 3D Calibration", "Apply 3D Calibration")
content = content.replace("✅ 3D Calibration 적용 완료! Output Routing에서 결과를 확인하세요.", "3D Calibration 적용 완료! Output Routing에서 결과를 확인하세요.")
content = content.replace("🎛 Output Routing", "Output Routing")

target = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 12,
          children: ["""

replacement = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Text(
                'Atmos Mixer Pro',
                style: TextStyle(
                  color: AppColors.textPrimary,
                  fontSize: 18,
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(width: 16),
              Consumer(
                builder: (context, ref, child) {
                  final isMasterMuted = ref.watch(
                    engineStateProvider.select((state) => state.masterMuteActive),
                  );
                  if (!isMasterMuted) return const SizedBox.shrink();
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
                    decoration: BoxDecoration(
                      color: Colors.orange.shade800,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.warning_amber_rounded, color: Colors.white, size: 20),
                        const SizedBox(width: 8),
                        Text(
                          'MASTER MUTE ACTIVE',
                          style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, letterSpacing: 1.2),
                        ),
                      ],
                    ),
                  );
                },
              ),
              const SizedBox(width: 16),
              const ResamplerStatusBadgeWidget(
                fileSampleRate: 44100,
                deviceSampleRate: 48000,
                forceActive: true,
              ),
              const SizedBox(width: 8),
              Consumer(builder: (context, ref, _) {
                return MasterLimiterMeterWidget(
                  initialGainReductionDb: ref.watch(engineStateProvider).shortTermLufs,
                  enableSimulationToggle: true,
                );
              }),
            ],
          ),
          const SizedBox(height: 16),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: ["""

if target in content:
    content = content.replace(target, replacement)
    
# Remove the old Title, Consumer(Mute), and Row(Resampler) from the old wrap block.
# We will just use regex to remove them from the `children: [` part until `Wrap(` for the inner buttons.

remove_target = """          const Text(
            'Atmos Mixer Pro',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 18,
              fontWeight: FontWeight.bold,
            ),
          ),
          Consumer(
            builder: (context, ref, child) {
              final isMasterMuted = ref.watch(
                engineStateProvider.select((state) => state.masterMuteActive),
              );
              if (!isMasterMuted) return const SizedBox.shrink();
              return Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 6,
                ),
                decoration: BoxDecoration(
                  color: Colors.orange.shade800,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.warning_amber_rounded,
                      color: Colors.white,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'MASTER MUTE ACTIVE',
                      style: TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        letterSpacing: 1.2,
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const ResamplerStatusBadgeWidget(
                fileSampleRate: 44100,
                deviceSampleRate: 48000,
                forceActive: true,
              ),
              const SizedBox(width: 8),
              MasterLimiterMeterWidget(
                initialGainReductionDb: ref.watch(engineStateProvider).shortTermLufs,
                enableSimulationToggle: true,
              ),
            ],
          ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: ["""

if remove_target in content:
    content = content.replace(remove_target, "")
    
# Finally replace the end part
end_target = """              ),
            ],
          ),
          Consumer(
            builder: (context, ref, child) {
              final config = ref.watch(configProvider);
              final engineState = ref.watch(engineStateProvider);

              return Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                children: ["""

end_replacement = """              ),
              const SizedBox(width: 24),
              Consumer(
                builder: (context, ref, child) {
                  final config = ref.watch(configProvider);
                  final engineState = ref.watch(engineStateProvider);

                  return Row(
                    mainAxisSize: MainAxisSize.min,
                    children: ["""

if end_target in content:
    content = content.replace(end_target, end_replacement)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
