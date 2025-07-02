const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('Starting the custom build process for the Chrome Extension...');

const reactAppDir         = path.join(__dirname, '..');
const contentScriptSource = path.join(reactAppDir, 'src', 'scripts', 'content.js');
const pageOverrideSource  = path.join(reactAppDir, 'src', 'scripts', 'pageFetchOverride.js');
const buildDir            = path.join(reactAppDir, 'build');
const contentScriptDest   = path.join(buildDir, 'static', 'js');

// 1️⃣ Run React build
try {
  console.log('Running react-scripts build...');
  execSync('react-scripts build', { stdio: 'inherit' });
  console.log('React build completed successfully.');
} catch (err) {
  console.error('React build failed:', err);
  process.exit(1);
}

// 2️⃣ Copy content.js
try {
  if (!fs.existsSync(contentScriptDest)) {
    throw new Error(`Missing directory: ${contentScriptDest}`);
  }
  fs.copyFileSync(contentScriptSource, path.join(contentScriptDest, 'content.js'));
  console.log(`Copied content.js → ${contentScriptDest}`);
} catch (err) {
  console.error('Failed to copy content.js:', err);
  process.exit(1);
}

// 3️⃣ Copy pageFetchOverride.js
try {
  if (!fs.existsSync(buildDir)) {
    throw new Error(`Missing build directory: ${buildDir}`);
  }
  fs.copyFileSync(pageOverrideSource, path.join(buildDir, 'pageFetchOverride.js'));
  console.log(`Copied pageFetchOverride.js → ${buildDir}`);
} catch (err) {
  console.error('Failed to copy pageFetchOverride.js:', err);
  process.exit(1);
}

console.log('Extension build process finished successfully!');
