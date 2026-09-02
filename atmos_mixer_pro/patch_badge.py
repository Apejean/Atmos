import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Update Badge Canvas rendering
old_badge = """      // 6.4 Floating 3D Channel Badge Text Sprite
      const badgeCanvas = document.createElement('canvas');
      badgeCanvas.width = 128;
      badgeCanvas.height = 64;
      const ctx = badgeCanvas.getContext('2d');
      ctx.fillStyle = isSelected ? '#0284c7' : '#0f172a';
      ctx.beginPath();
      ctx.roundRect(4, 4, 120, 56, 12);
      ctx.fill();
      ctx.strokeStyle = isSelected ? '#38bdf8' : '#334155';
      ctx.lineWidth = 4;
      ctx.stroke();
      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 26px -apple-system, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(`CH ${(speakerData.channel !== undefined ? speakerData.channel + 1 : 1)}`, 64, 32);

      const badgeTex = new THREE.CanvasTexture(badgeCanvas);
      const spriteMat = new THREE.SpriteMaterial({ map: badgeTex, depthTest: false });
      const sprite = new THREE.Sprite(spriteMat);
      sprite.scale.set(0.4, 0.2, 1.0);
      sprite.position.set(0, sh / 2 + 0.16, 0);
      group.add(sprite);"""

new_badge = """      // 6.4 Floating 3D Channel Badge Text Sprite (Updated for CH and Out CH)
      const badgeCanvas = document.createElement('canvas');
      badgeCanvas.width = 160;
      badgeCanvas.height = 80;
      const ctx = badgeCanvas.getContext('2d');
      ctx.fillStyle = isSelected ? '#0284c7' : '#0f172a';
      ctx.beginPath();
      ctx.roundRect(4, 4, 152, 72, 12);
      ctx.fill();
      ctx.strokeStyle = isSelected ? '#38bdf8' : '#334155';
      ctx.lineWidth = 4;
      ctx.stroke();
      
      // SPK Index
      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 28px -apple-system, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(`SPK ${speakerData.index || 1}`, 80, 28);
      
      // Output Channel
      ctx.font = 'bold 16px -apple-system, sans-serif';
      ctx.fillStyle = isSelected ? '#bae6fd' : '#94a3b8';
      ctx.fillText(`Out CH: ${speakerData.channel !== undefined ? speakerData.channel + 1 : 1}`, 80, 56);

      const badgeTex = new THREE.CanvasTexture(badgeCanvas);
      const spriteMat = new THREE.SpriteMaterial({ map: badgeTex, depthTest: false });
      const sprite = new THREE.Sprite(spriteMat);
      sprite.scale.set(0.6, 0.3, 1.0);
      sprite.position.set(0, sh / 2 + 0.20, 0);
      group.add(sprite);
      
      // Distance/Delay HUD Sprite (Hidden by default, updated in animate)
      const hudCanvas = document.createElement('canvas');
      hudCanvas.width = 200;
      hudCanvas.height = 40;
      const hudCtx = hudCanvas.getContext('2d');
      const hudTex = new THREE.CanvasTexture(hudCanvas);
      const hudMat = new THREE.SpriteMaterial({ map: hudTex, depthTest: false, transparent: true });
      const hudSprite = new THREE.Sprite(hudMat);
      hudSprite.scale.set(0.8, 0.16, 1.0);
      // It will float near the speaker
      hudSprite.position.set(0, sh / 2 + 0.50, 0);
      hudSprite.userData.isHud = true;
      hudSprite.userData.hudCanvas = hudCanvas;
      hudSprite.userData.hudCtx = hudCtx;
      hudSprite.userData.hudTex = hudTex;
      group.add(hudSprite);
      
      // Hit Dot (Laser collision point)
      const dotGeo = new THREE.SphereGeometry(0.04, 16, 16);
      const dotMat = new THREE.MeshBasicMaterial({ color: 0xef4444, transparent: true, opacity: 0.8 });
      const hitDot = new THREE.Mesh(dotGeo, dotMat);
      hitDot.visible = false;
      hitDot.userData.isHitDot = true;
      group.add(hitDot);
"""
content = content.replace(old_badge, new_badge)

with open(path, 'w') as f:
    f.write(content)
print("Badge patched.")
