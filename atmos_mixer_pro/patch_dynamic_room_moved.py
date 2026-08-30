import re

def main():
    path = "lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart"
    with open(path, "r") as f:
        content = f.read()

    # replace exception logic with firstOrNull
    content = content.replace("final node = currentNodes.firstWhere((n) => n.id == id, orElse: () => throw Exception('Node not found'));", "final node = currentNodes.where((n) => n.id == id).firstOrNull;")

    with open(path, "w") as f:
        f.write(content)

main()
