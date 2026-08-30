import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # The dart side expects data to be parsed from JSON.
    # In JS:
    # window.SpeakerBridge.postMessage(JSON.stringify({ type: 'SPEAKER_DRAGGING', speakerId: ..., x: ..., y: ... }));

    old_pointer_move_msg = """        // Flutter state update throttled/debounced (Only send SPEAKER_DRAGGING for light UI updates)
        if (!throttleDragTimer && window.flutter_inappwebview) {
          throttleDragTimer = setTimeout(() => {
            const payload = JSON.stringify({
              id: draggedSpeakerNode.userData.speakerId,
              x: draggedSpeakerNode.position.x,
              y: -draggedSpeakerNode.position.z
            });
            window.flutter_inappwebview.callHandler('SPEAKER_DRAGGING', payload);
            throttleDragTimer = null;
          }, 100); // 10fps throttle for dragging to prevent freeze
        }"""
        
    new_pointer_move_msg = """        // Flutter state update throttled/debounced (Only send SPEAKER_DRAGGING for light UI updates)
        if (!throttleDragTimer && window.SpeakerBridge) {
          throttleDragTimer = setTimeout(() => {
            const payload = JSON.stringify({
              type: 'SPEAKER_DRAGGING',
              speakerId: draggedSpeakerNode.userData.speakerId,
              x: draggedSpeakerNode.position.x,
              y: -draggedSpeakerNode.position.z
            });
            window.SpeakerBridge.postMessage(payload);
            throttleDragTimer = null;
          }, 100); // 10fps throttle for dragging to prevent freeze
        }"""

    content = content.replace(old_pointer_move_msg, new_pointer_move_msg)

    old_pointer_up_msg = """    function handlePointerRelease() {
      if (isDragging && draggedSpeakerNode) {
        // Send final authoritative state to Flutter on pointer up
        const payload = JSON.stringify({
           id: draggedSpeakerNode.userData.speakerId,
           x: draggedSpeakerNode.position.x,
           y: -draggedSpeakerNode.position.z
        });
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_MOVED', payload);
        }
      }"""
      
    new_pointer_up_msg = """    function handlePointerRelease() {
      if (isDragging && draggedSpeakerNode) {
        // Send final authoritative state to Flutter on pointer up
        const payload = JSON.stringify({
           type: 'SPEAKER_MOVED',
           speakerId: draggedSpeakerNode.userData.speakerId,
           x: draggedSpeakerNode.position.x,
           y: -draggedSpeakerNode.position.z
        });
        if (window.SpeakerBridge) {
           window.SpeakerBridge.postMessage(payload);
        }
      }"""

    content = content.replace(old_pointer_up_msg, new_pointer_up_msg)
    
    # Also for tapped:
    old_pointer_down_msg = """        if (window.onSpeakerTapped) window.onSpeakerTapped(clickedId);
        // Post message for flutter
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_TAPPED', clickedId);
        } else {
           console.log('SPEAKER_TAPPED:', clickedId);
        }"""
        
    new_pointer_down_msg = """        if (window.onSpeakerTapped) window.onSpeakerTapped(clickedId);
        // Post message for flutter
        if (window.SpeakerBridge) {
           window.SpeakerBridge.postMessage(JSON.stringify({
              type: 'SPEAKER_SELECTED',
              speakerId: clickedId
           }));
        } else {
           console.log('SPEAKER_SELECTED:', clickedId);
        }"""

    content = content.replace(old_pointer_down_msg, new_pointer_down_msg)

    with open(path, "w") as f:
        f.write(content)

main()
