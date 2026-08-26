# Image Analysis & Implementation Plan

The image provided requires a completely different layout style than the typical Material Flutter Scaffold structure.

## Layout Breakdown

1. **Root Container**: Very dark blue/grey `#161A23` or similar. Padding all around.
2. **Top Header**:
   - `Row` with `MainAxisAlignment.spaceBetween`
   - Left: Logo (Vertical bars + "3D AUDIO SIMULATOR" in two lines).
   - Right: `Row` containing:
     - "SPL HEATMAP" text
     - A custom cyan toggle switch (ON/OFF)
     - A blue outlined button with an icon and text "EXPORT PDF REPORT"
     - A simple user profile icon button
3. **Main Content Body (Below Header)**:
   - Needs to be an `Expanded` `Row` to take remaining height.
   - **Left Canvas Area (Expanded)**:
     - The background is a grid `CustomPaint`.
     - The room is drawn as a wireframe rectangle with dimensions (e.g., 6.0m on top, 4.5m on side).
     - The grid is strictly 2D flat in the image, but maybe they want isometric since they previously asked for isometric? Wait, looking at the *new* image, the grid and the room are NOT isometric. It is a straight top-down 2D orthographic projection. (S01, S02, S03 etc. are placed on a flat grid). 
     - **Speakers**: Instead of standard Material Icons, the speakers are drawn as physical box outlines (isometric speaker box) with cyan sound waves radiating out. They have labels like "S03".
     - **Center**: A user head/shoulder icon.
     - **Floating Room Setup Dialog**: Draggable or positioned `Stack` element on the bottom left. Glassmorphism or solid dark background. Has "Width", "Depth", "Ceiling Height", "Ear Level" and Apply/Cancel buttons.
   - **Right Inspector Panel (Fixed Width ~320px)**:
     - "SPEAKER INSPECTOR" title.
     - "Selected Speaker: S03 (Front Right)" text.
     - **HEIGHT (Z)** block: Vertical cyan slider + isometric speaker height icon + input box.
     - **TILT ANGLE** block: Arc slider (CustomPaint) + speaker tilt icon + input box.
     - **PAN ANGLE** block: Horizontal slider + compass icon + input box.
     - Bottom buttons: "Mute", "Solo", "Remove Speaker" outline buttons.

## Adjustments required in `speaker_canvas_screen.dart`:
- We need to abandon standard `Scaffold` appbar and build a completely custom `Column` -> `Row(Header) + Expanded(Row(Canvas + Sidebar))`.
- Remove the Isometric Transform `Matrix4` if it conflicts with the new image, because the new image shows a perfectly flat top-down 2D view (just with a really cool neon UI). Wait, if I look closely at the image, it IS flat 2D. The user said: "이 이미지대로 만들라고 UI를".
- **Tabs?**: The image does *not* show tabs. But the user said: "메인화면에 생성한 룸이랑 연동되게 룸탭이 추가가 되고 ... 그리고 보내준 목업 이미지와 동일하게 나오면 되는거야". I should put the Tabs right below the header or just inside the header.
- Let's create a completely fresh layout for the `build` method.
