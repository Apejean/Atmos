import re

def main():
    path = "lib/features/exhibition/models/speaker_node.dart"
    with open(path, "r") as f:
        content = f.read()

    # Need to add isFixed to SpeakerNode
    if "bool isFixed" not in content:
        # Add property
        content = re.sub(
            r"class SpeakerNode \{", 
            "class SpeakerNode {\n  final bool isFixed;", 
            content
        )
        
        # Add to constructor
        content = re.sub(
            r"required this\.dispersionAngle,", 
            "required this.dispersionAngle,\n    this.isFixed = false,", 
            content
        )
        
        # Add to copyWith
        content = re.sub(
            r"double\? dispersionAngle,", 
            "double? dispersionAngle,\n    bool? isFixed,", 
            content
        )
        content = re.sub(
            r"dispersionAngle: dispersionAngle \?\? this\.dispersionAngle,", 
            "dispersionAngle: dispersionAngle ?? this.dispersionAngle,\n      isFixed: isFixed ?? this.isFixed,", 
            content
        )
        
        # Add to toMap
        content = re.sub(
            r"'dispersionAngle': dispersionAngle,", 
            "'dispersionAngle': dispersionAngle,\n      'isFixed': isFixed,", 
            content
        )
        
        # Add to fromMap
        content = re.sub(
            r"dispersionAngle: map\['dispersionAngle'\] \?\? 90\.0,", 
            "dispersionAngle: map['dispersionAngle'] ?? 90.0,\n      isFixed: map['isFixed'] ?? false,", 
            content
        )

    with open(path, "w") as f:
        f.write(content)

main()
