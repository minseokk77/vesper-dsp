const { execSync } = require('child_process');
const fs = require('fs');
const privateKey = fs.readFileSync('src-tauri/keys/updater', 'utf-8');
process.env.TAURI_SIGNING_PRIVATE_KEY = privateKey;
process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '베스퍼dsp';
try {
  execSync('pnpm tauri build', { stdio: 'inherit' });
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
