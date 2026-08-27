import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

class Speaker3DBox extends StatelessWidget {
  final double angleX;
  final double angleY;
  final double angleZ;

  const Speaker3DBox({
    super.key,
    required this.angleX,
    required this.angleY,
    required this.angleZ,
  });

  @override
  Widget build(BuildContext context) {
    const double w = 160.0;
    const double h = 280.0;
    const double d = 120.0;

    return Transform(
      transform: Matrix4.identity()
        ..setEntry(3, 2, 0.001) // perspective
        ..rotateX(angleX)
        ..rotateY(angleY)
        ..rotateZ(angleZ),
      alignment: Alignment.center,
      child: Stack(
        alignment: Alignment.center,
        children: [
          // Back
          Transform(
            transform: Matrix4.translationValues(0, 0, -d / 2)..rotateY(math.pi),
            alignment: Alignment.center,
            child: SvgPicture.asset('assets/3d_simulator/speaker_tex_back.svg', width: w, height: h),
          ),
          // Left
          Transform(
            transform: Matrix4.translationValues(-w / 2, 0, 0)..rotateY(-math.pi / 2),
            alignment: Alignment.center,
            child: SvgPicture.asset('assets/3d_simulator/speaker_tex_side.svg', width: d, height: h),
          ),
          // Right
          Transform(
            transform: Matrix4.translationValues(w / 2, 0, 0)..rotateY(math.pi / 2),
            alignment: Alignment.center,
            child: SvgPicture.asset('assets/3d_simulator/speaker_tex_side.svg', width: d, height: h),
          ),
          // Top
          Transform(
            transform: Matrix4.translationValues(0, -h / 2, 0)..rotateX(-math.pi / 2),
            alignment: Alignment.center,
            child: SvgPicture.asset('assets/3d_simulator/speaker_tex_top.svg', width: w, height: d),
          ),
          // Front
          Transform(
            transform: Matrix4.translationValues(0, 0, d / 2),
            alignment: Alignment.center,
            child: SvgPicture.asset('assets/3d_simulator/speaker_tex_front.svg', width: w, height: h),
          ),
        ],
      ),
    );
  }
}
