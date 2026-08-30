import 'dart:io';

void main() {
  final file = File('lib/features/exhibition/state/three_js_engine_provider.dart');
  String content = file.readAsStringSync();

  content = content.replaceFirst(
    '''  void executeJavaScript(String js) {
    if (_isEngineReady && _webViewController != null) {''',
    '''  void executeJavaScript(String js) {
    if (isEngineReady && _webViewController != null) {'''
  );

  content = content.replaceFirst(
    '''  @override
  void dispose() {''',
    '''  void dispose() {'''
  );

  file.writeAsStringSync(content);
}
