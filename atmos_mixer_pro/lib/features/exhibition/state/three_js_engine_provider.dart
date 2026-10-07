import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:webview_flutter/webview_flutter.dart';
import 'package:webview_windows/webview_windows.dart' as win;
import 'dart:async';

/// Windows(WebView2)에는 webview_flutter의 JS 채널이 없으므로, 문서가 만들어질 때
/// 같은 이름의 객체를 넣어 HTML(`window.SpeakerBridge.postMessage(JSON 문자열)`)을 바꾸지 않고 쓴다.
const _windowsSpeakerBridgeShim =
    "window.SpeakerBridge = { postMessage: function (m) { window.chrome.webview.postMessage(m); } };";

class ThreeJsEngineService {
  HttpServer? _server;
  String? _serverUrl;
  WebViewController? _webViewController; // macOS: webview_flutter
  win.WebviewController? _windowsController; // Windows: WebView2(webview_windows)
  final ValueNotifier<bool> isEngineReadyNotifier = ValueNotifier(false);

  /// 3D 뷰어를 띄울 수 없는 이유(Windows에서 WebView2 런타임이 없거나 시작에 실패). null이면 정상이거나 준비 중.
  final ValueNotifier<String?> unavailableReasonNotifier = ValueNotifier(null);

  final _speakerTappedController = StreamController<String>.broadcast();
  Stream<String> get onSpeakerTapped => _speakerTappedController.stream;

  final _speakerMovedController = StreamController<Map<String, dynamic>>.broadcast();
  Stream<Map<String, dynamic>> get onSpeakerMoved => _speakerMovedController.stream;

  WebViewController? get controller => _webViewController;
  win.WebviewController? get windowsController => _windowsController;

  /// 3D 화면(Dynamic3DRoom)이 지금 보이는지. Windows에서 보이지 않을 때 WebView2를 멈추는 데 쓴다.
  bool _viewVisible = false;

  /// 플랫폼과 무관하게 3D 웹뷰가 만들어졌는지.
  bool get hasView => _webViewController != null || _windowsController != null;
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
            if (path.endsWith(".glb")) { contentType = "model/gltf-binary"; }
            else if (path.endsWith(".gltf")) { contentType = "model/gltf+json"; }
          } else if (path.startsWith("/assets/")) {
            assetPath = path.substring(1);
            if (path.endsWith(".html")) { contentType = "text/html; charset=utf-8"; }
            else if (path.endsWith(".js")) { contentType = "application/javascript; charset=utf-8"; }
            else if (path.endsWith(".glb")) { contentType = "model/gltf-binary"; }
            else if (path.endsWith(".gltf")) { contentType = "model/gltf+json"; }
            else if (path.endsWith(".svg")) { contentType = "image/svg+xml"; }
            else if (path.endsWith(".png")) { contentType = "image/png"; }
          } else {
            assetPath = "assets/3d_simulator$path";
            if (path.endsWith(".svg")) { contentType = "image/svg+xml"; }
            else if (path.endsWith(".png")) { contentType = "image/png"; }
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

      if (Platform.isWindows) {
        await _initWindowsWebView();
      } else {
        _initWebViewController();
      }
    } catch (e) {
      debugPrint("Error starting 3D local server: $e");
    }
  }

  /// JS → Dart 메시지(JSON 문자열) 처리. macOS JS 채널과 Windows 웹 메시지가 같이 쓴다.
  void _handleBridgeMessage(String raw) {
    try {
      final data = jsonDecode(raw);
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
  }

  /// Windows: WebView2로 같은 HTML을 띄운다. 런타임이 없거나 시작에 실패하면 앱은 그대로 두고
  /// [unavailableReasonNotifier]에 이유를 남겨 3D 자리에 안내를 보인다.
  Future<void> _initWindowsWebView() async {
    if (_serverUrl == null) return;

    final version = await win.WebviewController.getWebViewVersion();
    if (version == null) {
      debugPrint("ThreeJsEngine: WebView2 runtime is not installed");
      unavailableReasonNotifier.value =
          "WebView2 런타임이 설치되어 있지 않아 3D 방을 표시할 수 없습니다.\nMicrosoft Edge WebView2 런타임을 설치한 뒤 앱을 다시 시작하세요.";
      return;
    }

    final webController = win.WebviewController();
    try {
      await webController.initialize();
      await webController.setBackgroundColor(const Color(0xFF0B0F14));
      await webController.setPopupWindowPolicy(win.WebviewPopupWindowPolicy.deny);
      await webController.addScriptToExecuteOnDocumentCreated(_windowsSpeakerBridgeShim);

      // JS가 문자열을 보내면 문자열로, 객체를 보내면 Map으로 온다.
      webController.webMessage.listen(
        (message) => _handleBridgeMessage(message is String ? message : jsonEncode(message)),
        onError: (Object e) => debugPrint("Error handling JS message: $e"),
      );
      webController.loadingState.listen((state) {
        if (state == win.LoadingState.navigationCompleted) {
          debugPrint("ThreeJsEngine: WebView2 navigation completed");
          // Wait a bit for JS to fully evaluate before marking ready
          Future.delayed(const Duration(milliseconds: 500), () {
            isEngineReadyNotifier.value = true;
            // 3D 화면이 아직 열리지 않았으면 멈춰 둔다(setViewVisible에서 다시 켠다).
            if (!_viewVisible) _setWindowsViewActive(false);
          });
        }
      });
      webController.onLoadError.listen((status) {
        debugPrint("ThreeJsEngine: WebView2 Error: ${status.name}");
      });

      await webController.loadUrl(_serverUrl!);
      _windowsController = webController;
    } catch (e) {
      debugPrint("ThreeJsEngine: WebView2 start failed: $e");
      unavailableReasonNotifier.value = "3D 방 뷰어(WebView2)를 시작하지 못했습니다.\n$e";
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
      onMessageReceived: (message) => _handleBridgeMessage(message.message),
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
    if (!isEngineReady) return;
    if (_windowsController != null) {
      _windowsController!.executeScript(js).catchError((Object e) {
        debugPrint("ThreeJsEngine: executeScript failed: $e");
      });
    } else if (_webViewController != null) {
      _webViewController!.runJavaScript(js);
    }
  }


  /// 3D 화면이 열리거나(true) 닫힐 때(false) 부른다.
  /// macOS 웹뷰는 창에서 빠지면 그리기를 멈추지만 Windows의 WebView2는 화면 밖에서도 매 프레임 그리므로,
  /// 보이지 않는 동안 멈춰 무인 운영 중 CPU·GPU를 쓰지 않게 한다. macOS에서는 아무것도 하지 않는다.
  void setViewVisible(bool visible) {
    _viewVisible = visible;
    if (isEngineReady) _setWindowsViewActive(visible);
  }

  void _setWindowsViewActive(bool active) {
    final c = _windowsController;
    if (c == null) return;
    (active ? c.resume() : c.suspend()).catchError((Object e) {
      debugPrint("ThreeJsEngine: WebView2 ${active ? 'resume' : 'suspend'} failed: $e");
    });
  }

  void setEarLevel(double level) {
    executeJavaScript("window.updateEarLevel($level);");
  }

  void dispose() {
    _server?.close(force: true);
    _windowsController?.dispose();
    _speakerTappedController.close();
    isEngineReadyNotifier.dispose();
    unavailableReasonNotifier.dispose();
  }
}

final threeJsEngineProvider = Provider<ThreeJsEngineService>((ref) {
  final service = ThreeJsEngineService();
  service.initialize(); // Auto initialize on first read
  return service;
});
