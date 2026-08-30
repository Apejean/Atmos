import re

def main():
    path = "lib/features/exhibition/state/three_js_engine_provider.dart"
    with open(path, "r") as f:
        content = f.read()

    old_catch = """        } catch (e) {
          response
            ..statusCode = HttpStatus.internalServerError
            ..write("Error loading asset: $e");
        } finally {"""
    
    new_catch = """        } catch (e) {
          response
            ..statusCode = HttpStatus.internalServerError
            ..write("Error loading asset");
        } finally {"""

    content = content.replace(old_catch, new_catch)

    with open(path, "w") as f:
        f.write(content)

main()
