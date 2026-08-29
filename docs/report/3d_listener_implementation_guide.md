# 3D 청취자(Listener) 모델 적용 가이드 (@Front 에이전트 및 사용자 참고용)

## 목표
단순한 2D 이미지가 아닌, 고해상도의 3D 마네킹 헤드 모델(GLB/GLTF 형식)을 Flutter 앱에 띄워 360도로 자유롭게 회전시키는 3D 시각화 환경을 구축합니다.

## 필요한 3D 모델
사용자가 원하는 "고급스러운 유광 화이트 마네킹 헤드" 모델을 에셋으로 추가해야 합니다.
*   **권장 포맷**: `.glb` (단일 파일로 패키징되어 Flutter에서 다루기 가장 좋음)
*   **권장 소스**: [Sketchfab](https://sketchfab.com/) 또는 무료 3D 에셋 사이트에서 "Mannequin Head", "Bust", "Base Mesh Head" 등으로 검색하여 다운로드. (재질은 매끄러운 플라스틱/도자기 느낌 추천)
*   **저장 위치**: 모델을 구하면 `/Users/Allweno/Projects/GitHub/atmos/assets/models/listener_head.glb` 경로에 저장하세요.

## Flutter 구현 방법 (@Front 에이전트용 지시)
기존의 2D 이미지나 `Matrix4` 조합 대신, 진짜 3D 모델을 렌더링하는 전용 패키지를 도입해야 합니다.

1.  **패키지 추가**: `pubspec.yaml`에 `model_viewer_plus` 패키지를 추가합니다. (이 패키지가 GLB 렌더링에 가장 안정적입니다.)
    ```yaml
    dependencies:
      flutter:
        sdk: flutter
      model_viewer_plus: ^1.7.0 # (버전은 최신 안정 버전으로)
    ```
    *또한, `assets/models/` 경로를 `pubspec.yaml`의 assets 항목에 등록해야 합니다.*

2.  **3D 뷰어 위젯 구현**:
    ```dart
    ModelViewer(
      src: 'assets/models/listener_head.glb', // 로컬 에셋 경로
      alt: "A 3D model of a listener's head",
      ar: false, // AR 기능 끄기
      autoRotate: false, // 사용자 컨트롤 또는 시뮬레이션 데이터에 맡김
      cameraControls: true, // 터치로 회전 가능 여부 (시뮬레이터 성격에 맞게 조정)
      disableZoom: true, // 줌인/줌아웃 방지
      // backgroundColor: Colors.transparent, // 필요시 배경 투명화
    )
    ```

3.  **동적 회전 제어 (옵션)**:
    만약 터치가 아니라 X, Y, Z 데이터 값에 의해 코드로 강제 회전시켜야 한다면, `model_viewer_plus`의 `camera-orbit` 속성을 상태(State) 값과 연동하여 동적으로 업데이트하도록 구현해야 합니다.

## 다음 단계 (To-Do)
*   **사용자**: 위 조건에 맞는 3D 모델 파일(`.glb`)을 구해서 프로젝트의 `assets/models/` 폴더에 넣어주세요.
*   **@Front 에이전트**: 이 문서의 지시사항을 바탕으로 `model_viewer_plus` 패키지를 도입하고 UI에 모델을 띄우는 작업을 진행하세요.
