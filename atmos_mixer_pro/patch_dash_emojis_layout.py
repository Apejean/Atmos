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

# Layout replacement
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
          children: [
          const Text(
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
else:
    print("Could not find target to replace.")
    
# We must close the new Row and Column. The old structure was:
#           Wrap(
#             ...
#             children: [ ...buttons... ]
#           ),
#           Consumer(...) // ducking
#         ], // end of SingleChildScrollView child Wrap
#       ),
#     );

target_end = """              IconButton(
                icon: const Icon(Icons.grid_on, color: Colors.white),
                tooltip: 'Speaker Layout',
                onPressed: () {
                  Navigator.of(context).push(
                    MaterialPageRoute(
                      builder: (context) =>
                          const atmos_exhibition.SpeakerCanvasScreen(),
                    ),
                  );
                },
              ),
            ],
          ),
          Consumer(
            builder: (context, ref, child) {
              final config = ref.watch(configProvider);
              final engineState = ref.watch(engineStateProvider);

              return Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                children: [
                  if (engineState.duckingActive)
                    Container(
                      margin: const EdgeInsets.only(right: 4),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: AppColors.accentOrange.withValues(alpha: 0.2),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(color: AppColors.accentOrange),
                      ),
                      child: const Text(
                        '스마트 더킹 작동중',
                        style: TextStyle(
                          color: AppColors.accentOrange,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  Text(
                    config?.deviceName ?? '기본 오디오 출력',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.refresh, color: Colors.white),
                    tooltip: '스캔',
                    onPressed: () async {
                      try {
                        final deviceInfos = await rust_api.apiGetOutputDevices();
                        GlobalDeviceCache.devices = deviceInfos.map((d) => d.name).toList();
                        for (final info in deviceInfos) {
                          GlobalDeviceCache.channels[info.name] = info.channelNames;
                        }
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('오디오 장치 목록을 새로고침했습니다.')),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('스캔 실패: $e')),
                          );
                        }
                      }
                    },
                  ),
                ],
              );
            },
          ),
        ],
      ),
      ),
    );
  }"""

replacement_end = """              IconButton(
                icon: const Icon(Icons.grid_on, color: Colors.white),
                tooltip: 'Speaker Layout',
                onPressed: () {
                  Navigator.of(context).push(
                    MaterialPageRoute(
                      builder: (context) =>
                          const atmos_exhibition.SpeakerCanvasScreen(),
                    ),
                  );
                },
              ),
              const SizedBox(width: 24),
              Consumer(
                builder: (context, ref, child) {
                  final config = ref.watch(configProvider);
                  final engineState = ref.watch(engineStateProvider);

                  return Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (engineState.duckingActive)
                        Container(
                          margin: const EdgeInsets.only(right: 4),
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                          decoration: BoxDecoration(
                            color: AppColors.accentOrange.withValues(alpha: 0.2),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(color: AppColors.accentOrange),
                          ),
                          child: const Text(
                            '스마트 더킹 작동중',
                            style: TextStyle(
                              color: AppColors.accentOrange,
                              fontSize: 12,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ),
                      Text(
                        config?.deviceName ?? '기본 오디오 출력',
                        style: const TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.refresh, color: Colors.white),
                        tooltip: '스캔',
                        onPressed: () async {
                          try {
                            final deviceInfos = await rust_api.apiGetOutputDevices();
                            GlobalDeviceCache.devices = deviceInfos.map((d) => d.name).toList();
                            for (final info in deviceInfos) {
                              GlobalDeviceCache.channels[info.name] = info.channelNames;
                            }
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('오디오 장치 목록을 새로고침했습니다.')),
                              );
                            }
                          } catch (e) {
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('스캔 실패: $e')),
                              );
                            }
                          }
                        },
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
          ),
        ],
      ),
    );
  }"""

if target_end in content:
    content = content.replace(target_end, replacement_end)
else:
    print("Could not find target_end to replace.")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
