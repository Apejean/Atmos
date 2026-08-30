import re

def main():
    path = "lib/features/exhibition/models/speaker_node.dart"
    with open(path, "r") as f:
        content = f.read()

    # ensure fromMap and toMap and constructor are correct since regex might have failed partially
    if "this.isFixed = false," not in content:
        content = re.sub(
            r"required this\.dispersionAngle,", 
            "required this.dispersionAngle,\n    this.isFixed = false,", 
            content
        )
    if "'isFixed': isFixed" not in content:
        content = re.sub(
            r"'dispersionAngle': dispersionAngle,", 
            "'dispersionAngle': dispersionAngle,\n      'isFixed': isFixed,", 
            content
        )
    if "isFixed: map\['isFixed'\] \?\? false" not in content:
        content = re.sub(
            r"dispersionAngle: \(map\['dispersionAngle'\] \?\? 90\.0\)\.toDouble\(\),", 
            "dispersionAngle: (map['dispersionAngle'] ?? 90.0).toDouble(),\n      isFixed: map['isFixed'] ?? false,", 
            content
        )

    with open(path, "w") as f:
        f.write(content)

main()
