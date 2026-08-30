import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:webview_flutter/webview_flutter.dart';
import 'dart:async';

class ThreeJsEngineService {
  HttpServer? _server;
  String? _serverUrl;
  WebViewController? _webViewController;
  final ValueNotifier<bool> isEngineReadyNotifier = ValueNotifier(false);
  
  final _speakerTappedController = StreamController<String>.broadcast();
  Stream<String> get onSpeakerTapped => _speakerTappedController.stream;

  final _speakerMovedController = StreamController<Map<String, dynamic>>.broadcast();
  Stream<Map<String, dynamic>> get onSpeakerMoved => _speakerMovedController.stream;

  WebViewController? get controller => _webViewController;
  bool get isEngineReady => isEngineReadyNotifier.value;

  Future<void> initialize() async {
    if (_server != null) return; // Already initialized

    try {
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      _server = server;
      final port = server.port;
      _serverUrl = "http://127.0.0.1:$port/";

      server.listen((HttpRequest request) async {
        final path = request.uri.path;
        final response = request.response;
        
        try {
          String assetPath;
          String contentType = "application/octet-stream";

          if (path == "/" || path == "/index.html") {
            assetPath = "assets/3d_simulator/studio_engine.html";
            contentType = "text/html; charset=utf-8";
          } else if (path.startsWith("/js/")) {
            assetPath = "assets$path";
            contentType = "application/javascript; charset=utf-8";
          } else if (path.startsWith("/models/")) {
            assetPath = "assets$path";
            if (path.endsWith(".glb")) contentType = "model/gltf-binary";
            else if (path.endsWith(".gltf")) contentType = "model/gltf+json";
          } else if (path.startsWith("/assets/")) {
            assetPath = path.substring(1);
            if (path.endsWith(".html")) contentType = "text/html; charset=utf-8";
            else if (path.endsWith(".js")) contentType = "application/javascript; charset=utf-8";
            else if (path.endsWith(".glb")) contentType = "model/gltf-binary";
            else if (path.endsWith(".gltf")) contentType = "model/gltf+json";
            else if (path.endsWith(".svg")) contentType = "image/svg+xml";
            else if (path.endsWith(".png")) contentType = "image/png";
          } else {
            assetPath = "assets/3d_simulator$path";
            if (path.endsWith(".svg")) contentType = "image/svg+xml";
            else if (path.endsWith(".png")) contentType = "image/png";
          }

          final data = await rootBundle.load(assetPath);
          response
            ..statusCode = HttpStatus.ok
            ..headers.set("Content-Type", contentType)
            ..headers.set("Access-Control-Allow-Origin", "*")
            ..add(data.buffer.asUint8List());
        } catch (e) {
          response
            ..statusCode = HttpStatus.notFound
            ..write("Asset not found: $path");
        } finally {
          await response.close();
        }
      });

      _initWebViewController();
    } catch (e) {
      debugPrint("Error starting 3D local server: $e");
    }
  }

  void _initWebViewController() {
    if (_serverUrl == null) return;

    final webController = WebViewController();
    webController.setJavaScriptMode(JavaScriptMode.unrestricted);
    try {
      webController.setBackgroundColor(const Color(0xFF0B0F14));
    } catch (e) {
      debugPrint("macOS setBackgroundColor error ignored: $e");
    }
    
    webController.setOnConsoleMessage((message) {
      debugPrint("JS Console [${message.level.name}]: ${message.message}");
    });

    webController.addJavaScriptChannel(
      "SpeakerBridge",
      onMessageReceived: (message) {
        try {
          final data = jsonDecode(message.message);
          if (data["type"] == "SPEAKER_SELECTED" && data["speakerId"] != null) {
            final id = data["speakerId"] as String;
            _speakerTappedController.add(id);
          } else if ((data["type"] == "SPEAKER_MOVED" || data["type"] == "SPEAKER_DRAGGING") && data["speakerId"] != null) {
            _speakerMovedController.add({
              'id': data["speakerId"],
              'x': data["x"],
              'y': data["y"],
              'isFinal': data["type"] == "SPEAKER_MOVED",
            });
          }
        } catch (e) {
          debugPrint("Error handling JS message: $e");
        }
      },
    );
    
    webController.setNavigationDelegate(
      NavigationDelegate(
        onPageFinished: (url) {
          debugPrint("ThreeJsEngine: WebView onPageFinished: $url");
          // Wait a bit for JS to fully evaluate before marking ready
          Future.delayed(const Duration(milliseconds: 500), () {
            isEngineReadyNotifier.value = true;
          });
        },
        onWebResourceError: (error) {
          debugPrint("ThreeJsEngine: WebView Error: ${error.errorCode} - ${error.description}");
        },
      ),
    );
    
    webController.loadRequest(Uri.parse(_serverUrl!));
    _webViewController = webController;
    // We don't notify here because it's still loading. isEngineReadyNotifier handles the ready state.
  }

  void executeJavaScript(String js) {
    if (isEngineReady && _webViewController != null) {
      _webViewController!.runJavaScript(js);
    }
  }


  void setEarLevel(double level) {
    executeJavaScript("window.updateEarLevel($level);");
  }

  void dispose() {
    _server?.close(force: true);
    _speakerTappedController.close();
    isEngineReadyNotifier.dispose();
  }
}

final threeJsEngineProvider = Provider<ThreeJsEngineService>((ref) {
  final service = ThreeJsEngineService();
  service.initialize(); // Auto initialize on first read
  return service;
});
