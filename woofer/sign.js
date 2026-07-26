import { execSync } from 'child_process';

console.log("Starting auto-signer...");
try {
  execSync('npx tauri signer sign --private-key-path src-tauri\\\\keys\\\\updater "src-tauri\\\\target\\\\release\\\\bundle\\\\nsis\\\\Vesper Woofer_1.3.2_x64-setup.exe"', {
    env: { ...process.env, TAURI_KEY_PASSWORD: 'minjun7641' },
    stdio: 'inherit'
  });
  console.log("Sign complete!");
} catch (e) {
  console.error("Sign failed:", e);
}
