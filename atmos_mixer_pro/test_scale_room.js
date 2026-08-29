// Javascript to inject
const mv = document.querySelector('model-viewer');
mv.addEventListener('scene-graph-ready', function(e) {
  const sceneSymbol = Object.getOwnPropertySymbols(mv).find(s => s.description === 'scene');
  if(sceneSymbol) {
    const scene = mv[sceneSymbol];
    scene.traverse((node) => {
      // Find the room mesh (vertices < 1000)
      if (node.isMesh && node.geometry && node.geometry.attributes.position.count < 1000) {
         node.scale.set(scaleX, scaleY, scaleZ);
         node.updateMatrix();
         node.updateMatrixWorld(true);
      }
    });
  }
});
