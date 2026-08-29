const playwright = require('playwright');
(async () => {
  const browser = await playwright.chromium.launch();
  const page = await browser.newPage();
  await page.setContent(`
    <script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/3.3.0/model-viewer.min.js"></script>
    <model-viewer id="mv" src="https://modelviewer.dev/shared-assets/models/Astronaut.glb" scale="2 2 2"></model-viewer>
  `);
  await page.waitForLoadState('networkidle');
  const result = await page.evaluate(async () => {
    const mv = document.querySelector('model-viewer');
    // wait for scene-graph-ready
    await new Promise(r => mv.addEventListener('scene-graph-ready', r));
    const symbols = Object.getOwnPropertySymbols(mv).map(s => s.toString());
    const sceneSymbol = Object.getOwnPropertySymbols(mv).find(s => s.description === 'scene');
    if (sceneSymbol) {
       const scene = mv[sceneSymbol];
       let meshCount = 0;
       scene.traverse(node => { if(node.isMesh) meshCount++; });
       return { found: true, meshCount };
    }
    return { found: false, symbols };
  });
  console.log(result);
  await browser.close();
})();
