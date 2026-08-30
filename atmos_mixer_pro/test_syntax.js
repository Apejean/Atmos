const fs = require('fs');
const acorn = require('acorn');

const code = fs.readFileSync('assets/3d_simulator/studio_engine.html', 'utf8');

// Extract everything inside <script> tags
const scriptRegex = /<script>([\s\S]*?)<\/script>/g;
let match;
while ((match = scriptRegex.exec(code)) !== null) {
  const scriptContent = match[1];
  try {
    acorn.parse(scriptContent, { ecmaVersion: 2020 });
    console.log("Syntax is valid!");
  } catch (err) {
    console.error("Syntax Error: ", err.message);
  }
}
