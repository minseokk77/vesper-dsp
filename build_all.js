const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function processProject(name, dir, password) {
  console.log(`\n==============================================`);
  console.log(`🚀 Processing ${name}...`);
  console.log(`==============================================\n`);
  
  process.env.TAURI_KEY_PASSWORD = password;
  
  // 1. Generate keys
  console.log(`[1/4] Generating new updater keys for ${name}...`);
  try {
    execSync(`pnpm tauri signer generate -w src-tauri/keys/updater -f`, { cwd: dir, stdio: 'inherit' });
  } catch (e) {
    console.error(`Error generating keys for ${name}:`, e.message);
    process.exit(1);
  }
  
  // 2. Read pubkey
  console.log(`[2/4] Reading new public key...`);
  const pubkey = fs.readFileSync(path.join(dir, 'src-tauri/keys/updater.pub'), 'utf-8').trim();
  
  // 3. Update tauri.conf.json
  console.log(`[3/4] Updating tauri.conf.json with new public key...`);
  const confPath = path.join(dir, 'src-tauri/tauri.conf.json');
  const conf = JSON.parse(fs.readFileSync(confPath, 'utf-8'));
  conf.plugins.updater.pubkey = pubkey;
  fs.writeFileSync(confPath, JSON.stringify(conf, null, 2), 'utf-8');
  console.log(`✅ Updated pubkey for ${name}`);
  
  // 4. Build
  console.log(`[4/4] Building ${name}... (This might take a while)`);
  const privateKey = fs.readFileSync(path.join(dir, 'src-tauri/keys/updater'), 'utf-8');
  process.env.TAURI_PRIVATE_KEY = privateKey;
  try {
    execSync(`pnpm tauri build`, { cwd: dir, stdio: 'inherit' });
    console.log(`🎉 Built ${name} successfully!`);
  } catch (e) {
    console.error(`Error building ${name}:`, e.message);
    process.exit(1);
  }
}

processProject('Vesper DSP', 'C:\\Users\\minse\\Documents\\antigravity\\noble-babbage\\vesper\\dsp', '베스퍼dsp');
processProject('Vesper Woofer', 'C:\\Users\\minse\\Documents\\antigravity\\noble-babbage\\vesper\\woofer', '베스퍼 우퍼');

console.log("\n✅ All projects successfully built and signed with new keys!");
