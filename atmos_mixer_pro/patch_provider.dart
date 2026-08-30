import 'dart:io';

void main() {
  final file = File('lib/features/exhibition/state/three_js_engine_provider.dart');
  String content = file.readAsStringSync();

  content = content.replaceFirst(
    '''  final _speakerTappedController = StreamController<String>.broadcast();
  Stream<String> get onSpeakerTapped => _speakerTappedController.stream;''',
    '''  final _speakerTappedController = StreamController<String>.broadcast();
  Stream<String> get onSpeakerTapped => _speakerTappedController.stream;

  final _speakerMovedController = StreamController<Map<String, dynamic>>.broadcast();
  Stream<Map<String, dynamic>> get onSpeakerMoved => _speakerMovedController.stream;'''
  );

  content = content.replaceFirst(
    '''          if (data["type"] == "SPEAKER_SELECTED" && data["speakerId"] != null) {
            final id = data["speakerId"] as String;
            _speakerTappedController.add(id);
          }''',
    '''          if (data["type"] == "SPEAKER_SELECTED" && data["speakerId"] != null) {
            final id = data["speakerId"] as String;
            _speakerTappedController.add(id);
          } else if ((data["type"] == "SPEAKER_MOVED" || data["type"] == "SPEAKER_DRAGGING") && data["speakerId"] != null) {
            _speakerMovedController.add({
              'id': data["speakerId"],
              'x': data["x"],
              'y': data["y"],
              'isFinal': data["type"] == "SPEAKER_MOVED",
            });
          }'''
  );
  
  content = content.replaceFirst(
    '''  void dispose() {
    isEngineReadyNotifier.dispose();
    _speakerTappedController.close();''',
    '''  void dispose() {
    isEngineReadyNotifier.dispose();
    _speakerTappedController.close();
    _speakerMovedController.close();'''
  );

  file.writeAsStringSync(content);
}
