with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add _showRoomSetupDialog
new_method = """  Future<void> _showRoomSetupDialog() async {
    final bp = ref.read(blueprintProvider);
    double widthM = bp.canvasWidthMeters;
    double lengthM = bp.canvasHeightMeters;
    double heightM = bp.roomHeightMeters;
    double listenM = bp.listeningHeightMeters;
    
    await showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          backgroundColor: AppColors.cardSurface,
          title: const Text('📏 Room Setup', style: TextStyle(color: Colors.white)),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                initialValue: widthM.toStringAsFixed(1),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: '방 가로 너비 (Width, m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
                onChanged: (val) => widthM = double.tryParse(val) ?? widthM,
              ),
              const SizedBox(height: 8),
              TextFormField(
                initialValue: lengthM.toStringAsFixed(1),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: '방 세로 길이 (Length, m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
                onChanged: (val) => lengthM = double.tryParse(val) ?? lengthM,
              ),
              const SizedBox(height: 8),
              TextFormField(
                initialValue: heightM.toStringAsFixed(1),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: '천장 높이 (Height, m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
                onChanged: (val) => heightM = double.tryParse(val) ?? heightM,
              ),
              const SizedBox(height: 8),
              TextFormField(
                initialValue: listenM.toStringAsFixed(1),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: '관객 귀 높이 (Listening Height, m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
                onChanged: (val) => listenM = double.tryParse(val) ?? listenM,
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('취소', style: TextStyle(color: Colors.white70)),
            ),
            ElevatedButton(
              style: ElevatedButton.styleFrom(backgroundColor: AppColors.primaryNeon, foregroundColor: Colors.black),
              onPressed: () {
                ref.read(blueprintProvider.notifier).setCanvasDimensions(
                  widthM.clamp(1.0, 1000.0), 
                  lengthM.clamp(1.0, 1000.0),
                  roomHM: heightM.clamp(1.0, 100.0),
                  listenHM: listenM.clamp(0.0, 20.0),
                );
                Navigator.pop(context);
              },
              child: const Text('저장'),
            ),
          ],
        );
      },
    );
  }

  Widget build(BuildContext context) {"""

content = content.replace("  Widget build(BuildContext context) {\n    final blueprint = ref.watch(blueprintProvider);", 
new_method + "\n    final blueprint = ref.watch(blueprintProvider);")

content = content.replace(
"""        floatingActionButton: PopupMenuButton<String>(""",
"""        floatingActionButton: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            FloatingActionButton.extended(
              heroTag: 'room_setup_fab',
              onPressed: _showRoomSetupDialog,
              backgroundColor: Colors.white10,
              foregroundColor: Colors.white,
              icon: const Icon(Icons.square_foot),
              label: const Text('📏 Room Setup'),
            ),
            const SizedBox(height: 16),
            PopupMenuButton<String>("""
)

content = content.replace(
"""          child: FloatingActionButton(
            onPressed: null,
            backgroundColor: AppColors.primaryNeon,
            child: const Icon(Icons.add, color: Colors.black),
          ),
        ),
        bottomNavigationBar: Container(""",
"""          child: FloatingActionButton(
            heroTag: 'add_menu_fab',
            onPressed: null,
            backgroundColor: AppColors.primaryNeon,
            child: const Icon(Icons.add, color: Colors.black),
          ),
        ),
          ],
        ),
        bottomNavigationBar: Container("""
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
