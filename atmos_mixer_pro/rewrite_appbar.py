import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# We need to extract the entire AppBar block from `appBar: AppBar(` to its closing `),` inside the `Scaffold`.
# Since it's huge and nested, we will use a Python parenthesis matching function.

def find_matching_brace(text, start_index):
    count = 0
    for i in range(start_index, len(text)):
        if text[i] == '(':
            count += 1
        elif text[i] == ')':
            count -= 1
            if count == 0:
                return i
    return -1

appbar_start = content.find("appBar: AppBar(")
appbar_end = find_matching_brace(content, appbar_start + len("appBar: AppBar") - 1)

new_appbar = """appBar: AppBar(
                title: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Text('Exhibition Canvas', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                    const SizedBox(height: 8),
                    SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: Row(
                        children: [
                          ElevatedButton.icon(
                            onPressed: () {
                              setState(() => _showHeatmap = !_showHeatmap);
                            },
                            icon: Icon(_showHeatmap ? Icons.visibility : Icons.visibility_off, size: 16),
                            label: Text(_showHeatmap ? 'HIDE SPL HEATMAP' : 'SHOW SPL HEATMAP'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF23252A),
                              foregroundColor: _showHeatmap ? AppColors.primaryNeon : Colors.white54,
                            ),
                          ),
                          const SizedBox(width: 16),
                          ElevatedButton.icon(
                            onPressed: () {},
                            icon: const Icon(Icons.picture_as_pdf, size: 16),
                            label: const Text('EXPORT PDF REPORT'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF23252A),
                              foregroundColor: AppColors.primaryNeon,
                            ),
                          ),
                          const SizedBox(width: 16),
                          ElevatedButton.icon(
                            onPressed: () {
                              showDialog(
                                context: context,
                                builder: (context) => const RoomSetupModal(),
                              );
                            },
                            icon: const Icon(Icons.square_foot, size: 16),
                            label: const Text('ROOM SETUP'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF23252A),
                              foregroundColor: Colors.white,
                            ),
                          ),
                          const SizedBox(width: 16),
                          Container(
                            height: 34,
                            padding: const EdgeInsets.symmetric(horizontal: 6),
                            decoration: BoxDecoration(
                              color: Colors.grey.shade900,
                              borderRadius: BorderRadius.circular(17),
                              border: Border.all(
                                color: _isBinauralEnabled ? AppColors.primaryNeon : Colors.white24,
                                width: 1.5,
                              ),
                            ),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  _isBinauralEnabled ? Icons.headphones : Icons.speaker_group,
                                  size: 16,
                                  color: _isBinauralEnabled ? AppColors.primaryNeon : Colors.white70,
                                ),
                                const SizedBox(width: 6),
                                Text(
                                  _isBinauralEnabled ? 'Virtual (Binaural)' : 'Physical (Direct Out)',
                                  style: TextStyle(
                                    fontSize: 12,
                                    fontWeight: FontWeight.bold,
                                    color: _isBinauralEnabled ? AppColors.primaryNeon : Colors.white,
                                  ),
                                ),
                                const SizedBox(width: 4),
                                Switch(
                                  value: _isBinauralEnabled,
                                  activeColor: AppColors.primaryNeon,
                                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                                  onChanged: (val) {
                                    setState(() => _isBinauralEnabled = val);
                                  },
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                toolbarHeight: 100,
                bottom: PreferredSize(
                  preferredSize: const Size.fromHeight(48),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: TabBar(
                      isScrollable: true,
                      indicatorColor: AppColors.primaryNeon,
                      labelColor: AppColors.primaryNeon,
                      unselectedLabelColor: Colors.white54,
                      tabs: uiRooms.isEmpty 
                        ? [const Tab(text: 'Default Room')]
                        : uiRooms.map((r) => Tab(text: r.name)).toList(),
                    ),
                  ),
                ),
              )"""

content = content[:appbar_start] + new_appbar + content[appbar_end + 1:]
with open(path, "w") as f:
    f.write(content)
