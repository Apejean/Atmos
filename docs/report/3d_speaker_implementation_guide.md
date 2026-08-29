# 3D 스피커 UI 구현 가이드 (@Front 에이전트 참고용)

## 목표
Flutter의 `Matrix4` 3D 변환(Transform)을 사용하여, 4장의 평면 2D SVG 텍스처를 조합해 실시간으로 X, Y, Z축 회전이 가능한 진짜 3D 스피커 박스를 구현합니다.

## 사용될 에셋 (Image/3D simulator/ 경로)
1. `speaker_tex_front.svg` (160 x 280) - 정면 (흰색 우퍼/트위터)
2. `speaker_tex_top.svg` (160 x 120) - 윗면 (전면 방향 화살표)
3. `speaker_tex_back.svg` (160 x 280) - 뒷면 (단자 없음, 깔끔한 배경)
4. `speaker_tex_side.svg` (120 x 280) - 좌/우 측면 공용

## 3D 박스 조립 스펙 (Transform)
*   **스피커 박스 크기**: Width(W): 160, Height(H): 280, Depth(D): 120
*   `Stack` 위젯 내부에 각 면(Face) 위젯을 배치하고 `Transform` 위젯으로 3D 공간에 조립합니다.
*   조립 로직 기준 (기준점은 큐브의 정중앙 또는 바닥 중앙으로 설정하여 회전 시 축이 틀어지지 않도록 주의):
    *   **Front**: `Matrix4.translationValues(0, 0, D/2)`
    *   **Back**: `Matrix4.translationValues(0, 0, -D/2) * Matrix4.rotationY(pi)`
    *   **Top**: `Matrix4.translationValues(0, -H/2, 0) * Matrix4.rotationX(-pi/2)`
    *   **Left Side**: `Matrix4.translationValues(-W/2, 0, 0) * Matrix4.rotationY(-pi/2)`
    *   **Right Side**: `Matrix4.translationValues(W/2, 0, 0) * Matrix4.rotationY(pi/2)`

## 상호작용
*   부모 위젯에서 전달받는 X, Y, Z 각도(회전 상태값)를 이 조립된 3D 큐브 그룹 전체에 적용(`Transform(transform: Matrix4.rotationX(angleX) * Matrix4.rotationY(angleY) * Matrix4.rotationZ(angleZ)`)하여, 끊김 없이 부드럽게 렌더링되도록 구현하세요.
