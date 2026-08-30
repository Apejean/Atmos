import re

def main():
    path = "lib/features/exhibition/models/speaker_node.dart"
    with open(path, "r") as f:
        content = f.read()

    # Need to add isFixed to the actual constructor definition
    if "this.isFixed = false," not in content:
        content = content.replace(
            "required this.channel,",
            "required this.channel,\n    this.isFixed = false,"
        )

    # Need to add isFixed to toJson
    if "'isFixed': isFixed," not in content:
        content = content.replace(
            "'channel': channel,",
            "'channel': channel,\n      'isFixed': isFixed,"
        )
    
    # Need to add isFixed to fromJson
    if "isFixed: map['isFixed'] ?? false," not in content:
        content = content.replace(
            "channel: map['channel'],",
            "channel: map['channel'],\n      isFixed: map['isFixed'] ?? false,"
        )
        
    with open(path, "w") as f:
        f.write(content)

main()
