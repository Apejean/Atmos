# 동적 3D 룸 & 청취자 시뮬레이터 개발 가이드 (@Front 에이전트 용)

## 핵심 아키텍처 (Decoupled Rendering)
사용자가 `ROOM SETUP`에서 방 크기(가로, 세로, 높이)를 변경할 때, 중앙의 청취자(머리)는 크기가 왜곡되지 않고 방의 크기만 실시간으로 변경되어야 합니다.
이를 위해 **3D 마네킹 헤드(GLB)**와 **방의 격자(Wireframe)** 렌더링을 완전히 분리하고, **하나의 회전 상태(Angle)**로 두 렌더링을 동기화(Sync)합니다.

## 1. 메인 시뮬레이터 화면 (Main Canvas)
*   **배경 (Background)**: 어두운 화면 위에 코드로 그려진 3D 룸 와이어프레임과 4x4 바닥 격자가 깔려 있습니다.
*   **바닥 (Floor)**: 격자 위로 스윗스팟을 나타내는 DBAP 히트맵이 물감 번지듯 렌더링 됩니다.
*   **정중앙 (Center)**: **3D 마네킹 헤드(`listener_head.glb`)**가 위치하며, 사용자가 화면을 스와이프하면 방과 머리, 히트맵이 일체형으로 360도 회전합니다. (배치: W/2, D/2, 1.2m)
*   **객체 (Objects)**: 2D 텍스처를 3D로 변환한 스피커 큐브들이 공간 곳곳에 떠 있거나 바닥에 놓여 있습니다.
*   **우측 하단 (Bottom Right)**: 새로운 스피커를 방 한가운데 생성하는 `[Add Speaker]` 플로팅 버튼이 있습니다.
*   **좌측 상단 헤더**: 뒤로가기 화살표 (`icon_back.svg`) 버튼과 상단 타이틀 "Exhibition Canvas"가 존재합니다.

(※ 기술 구현 메모: 헤드는 `model_viewer_plus`로 구현하고 부모의 제스처 상태를 `camera-orbit`으로 넘겨 동기화 회전 처리합니다.)

## 2. 동적 룸 와이어프레임 & 격자 (CustomPaint)
*   방의 바닥 4x4 격자와 기둥 테두리는 모델 파일이 아닌 **Flutter의 `CustomPaint`**를 사용하여 직접 그립니다.
*   **동적 크기**: `ROOM SETUP` 상태에서 관리하는 방의 너비(W), 깊이(D), 높이(H) 데이터를 기반으로 3D 좌표(X, Y, Z)를 계산합니다.
*   **3D -> 2D 투영**: 부모로부터 받은 회전 상태(pan/tilt 각도)를 `Matrix4`에 적용(`Matrix4.rotationX` * `Matrix4.rotationY`)하고, 각 3D 점(Vertex)을 2D 화면 좌표로 투영(Projection)하여 `canvas.drawLine`으로 와이어프레임을 그립니다.

## 3. 회전 동기화 (Syncing Rotation)
*   화면 전체를 덮는 `GestureDetector`를 `Stack` 최상단에 배치합니다.
*   사용자가 드래그(PanUpdate)할 때 발생하는 Delta 값을 `orbitAngleX`와 `orbitAngleY` 상태(State)에 누적합니다.
*   **동기화**:
    1.  이 각도를 `model_viewer_plus`의 `camera-orbit` 파라미터로 넘깁니다. (모델 회전)
    2.  이 각도를 `CustomPaint`의 `Matrix4` 투영 계산식에 넘깁니다. (방 격자 회전)
*   이렇게 하면 3D 모델과 코드로 그린 방이 완벽하게 한 세트처럼 빙글빙글 돌아가게 됩니다.

## Z-Ordering (Stack 배치)
```dart
Stack(
  children: [
    // 1. 방의 뒷면 기둥과 바닥 격자를 그리는 CustomPaint (Background)
    DynamicRoomPainter(..., angleX, angleY, drawBackground: true),
    
    // 2. 3D 헤드 모델
    ModelViewer(src: '...', cameraOrbit: '${angleX}deg ${angleY}deg'),
    
    // 3. 방의 앞면 기둥을 그리는 CustomPaint (Foreground) - 입체감을 위해 모델 앞쪽 선을 기름
    DynamicRoomPainter(..., angleX, angleY, drawForeground: true),
    
    // 4. 회전 제어용 제스처 감지기
    GestureDetector(onPanUpdate: ...),
  ]
)
```

## 4. 스피커 관리 및 배치 조작
*   **간편한 생성 (Add Speaker)**: 캔버스 우측 하단의 `[Add Speaker]` 버튼(플로팅 동작)을 누르면, 룸의 정중앙에 새로운 스피커 노드가 즉시 배치됩니다.
*   **채널 자동 할당**: 스피커를 생성할 때, 현재 사용 중이지 않은 가장 빠른 채널 번호를 스캔하여 자동으로 할당합니다. (1번부터 시작하며, 오디오 인터페이스나 하드웨어에서 인식된 실제 채널 수와 동일하게 매칭됩니다.)
*   **배치 조작 (Drag & Drop)**: 생성된 스피커는 Blueprint(도면) 위에서 드래그 앤 드롭으로 자유롭게 이동이 가능해야 하며, 3차원 공간(X, Y, Z축)의 모든 좌표 이동을 지원합니다.

## 5. 룸 어쿠스틱 및 글로벌 리버브 (Room Acoustics)
룸 전체의 음향 특성을 시뮬레이션하기 위한 글로벌 설정 값은 다음과 같이 제어됩니다.
*   **글로벌 리버브 제어**: 글로벌 환경 변수로 **'리버브 믹스(Reverb Mix)'**와 **'잔향 감쇠 시간(Reverb Decay)'**을 조절하는 슬라이더를 두어 전체 공간의 울림 정도를 통제합니다.
*   **공간 분리 및 차음 (Transmission Loss)**: 방 벽면의 **차음 성능(dB)**을 설정하여, 구역 안팎으로 소리가 투과될 때 발생하는 음압 감쇠 및 반사 시뮬레이션의 기초 데이터를 제공합니다.
