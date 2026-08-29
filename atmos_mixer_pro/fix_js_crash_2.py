import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

find_js = """    final jsCall = "window.updateScene(${jsonEncode(payload)});";
    _webViewController!.runJavaScript(jsCall);"""

replace_js = """    final jsCall = "if (typeof window.updateScene === 'function') { window.updateScene(${jsonEncode(payload)}); }";
    _webViewController!.runJavaScript(jsCall);"""

content = content.replace(find_js, replace_js)

find_js2 = """_webViewController!.runJavaScript("window.setCameraView('$viewName');");"""
replace_js2 = """_webViewController!.runJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('$viewName'); }");"""

content = content.replace(find_js2, replace_js2)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
