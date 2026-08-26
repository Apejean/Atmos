import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """        return Scaffold(
          backgroundColor: bgColor,
          body: Column("""
replacement = """        return Scaffold(
          backgroundColor: bgColor,
          floatingActionButton: FloatingActionButton.extended(
            backgroundColor: neonCyan,
            icon: const Icon(Icons.add, color: Colors.black),
            label: const Text('Add Speaker', style: TextStyle(color: Colors.black, fontWeight: FontWeight.bold)),
            onPressed: () {
              ref.read(speakerLayoutProvider.notifier).addSpeaker(
                SpeakerNode(
                  id: DateTime.now().millisecondsSinceEpoch.toString(),
                  x: 0,
                  y: 0,
                  heightZ: 3.0, // default ceiling
                  channel: 0,
                ),
              );
            },
          ),
          body: Column("""
content = content.replace(target, replacement)

# Fix InteractiveViewer boundary margin and scaling
target_iv = """                          // Interactive Viewer for Canvas
                          GestureDetector(
                            onTap: () {
                              _canvasFocusNode.requestFocus();
                              setState(() => _inspectorSpeakerId = null);
                            },
                            child: Container(
                              color: bgColor,
                              child: InteractiveViewer(
                                transformationController: _transformationController,
                                minScale: 0.1,
                                maxScale: 10.0,
                                constrained: false,
                                child: Stack("""
replacement_iv = """                          // Interactive Viewer for Canvas
                          GestureDetector(
                            onTap: () {
                              _canvasFocusNode.requestFocus();
                              setState(() => _inspectorSpeakerId = null);
                            },
                            child: Container(
                              color: bgColor,
                              child: InteractiveViewer(
                                transformationController: _transformationController,
                                minScale: 0.1,
                                maxScale: 10.0,
                                constrained: false,
                                boundaryMargin: const EdgeInsets.all(2000), // Fix zoom out bug
                                child: Stack("""
content = content.replace(target_iv, replacement_iv)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
