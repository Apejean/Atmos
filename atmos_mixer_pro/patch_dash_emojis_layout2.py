import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

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
