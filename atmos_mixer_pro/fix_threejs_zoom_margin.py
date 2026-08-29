import re

with open('assets/3d_simulator/studio_engine.html', 'r') as f:
    content = f.read()

find_zoom = """
    // --- 8. Set Camera View Presets ---
    window.setCameraView = function(viewName) {
      const maxDim = Math.max(currentRoom.width, currentRoom.depth);
      const dist = maxDim * 1.35;
"""

# The user wants "적당하게 맞춰서 양 옆과 위 아래 어느정도의 여백을 두고 보여주는게 좋을거같아"
# And "더블 클릭했을때 너무 줌이 멀리 되서 다시 줌인을 해야하는데 이걸 딱 핏하게 화면에 전체적으로 다 보이게 하면 좋을거같아"
# If `dist = maxDim * 1.35;` is too far, we need to decrease the multiplier. Wait, the user said "너무 줌이 멀리 되서" (it zooms out too far).
# So we need to DECREASE the distance!
# Let's change it to `maxDim * 0.9` or `maxDim * 0.85`.
# Wait, let's also check the double-click event.

replace_zoom = """
    // --- 8. Set Camera View Presets ---
    window.setCameraView = function(viewName) {
      const maxDim = Math.max(currentRoom.width, currentRoom.depth);
      // Decrease the multiplier from 1.35 so that it is "딱 핏하게" (snugly fit) with some margin.
      // Since FOV is usually around 45-60, distance = maxDim / (2 * tan(fov/2))
      // For FOV 50, tan(25) ~ 0.46, so distance ~ maxDim / 0.92 ~ maxDim * 1.08.
      // Let's use 1.0.
      const dist = maxDim * 1.0;
"""
content = content.replace(find_zoom, replace_zoom)

# Let's also check if there is a separate double click zoom out logic
find_double_click = """    window.addEventListener('dblclick', () => {
      window.setCameraView('Auto');
    });"""

replace_double_click = """    window.addEventListener('dblclick', () => {
      // Dispatch an event or just do it locally?
      // Wait, we should probably read the current view from somewhere, or default to Auto.
      window.setCameraView(window.currentViewName || 'Auto');
    });"""
content = content.replace(find_double_click, replace_double_click)

# Let's add window.currentViewName saving
find_save_view = """      switch(viewName) {"""
replace_save_view = """      window.currentViewName = viewName;
      switch(viewName) {"""
content = content.replace(find_save_view, replace_save_view)

with open('assets/3d_simulator/studio_engine.html', 'w') as f:
    f.write(content)
