const playwright = require('playwright');
(async () => {
  const browser = await playwright.chromium.launch();
  const page = await browser.newPage();
  await page.setContent(`
    <script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/3.3.0/model-viewer.min.js"></script>
    <model-viewer id="mv" src="https://modelviewer.dev/shared-assets/models/Astronaut.glb"></model-viewer>
  `);
  await page.waitForLoadState('networkidle');
  const result = await page.evaluate(() => {
    const mv = document.querySelector('model-viewer');
    const symbols = Object.getOwnPropertySymbols(mv).map(s => s.toString());
    return symbols;
  });
  console.log(result);
  await browser.close();
})();
