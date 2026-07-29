import { readFile } from 'node:fs/promises';

const apps = [
  { directory: 'dsp', updater: 'vesper-dsp/updater.json' },
  { directory: 'woofer', updater: 'vesper-woofer/updater.json' }
];

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

for (const app of apps) {
  const packageJson = await readJson(`${app.directory}/package.json`);
  const tauriConfig = await readJson(`${app.directory}/src-tauri/tauri.conf.json`);
  const updater = await readJson(app.updater);
  const cargoToml = await readFile(`${app.directory}/src-tauri/Cargo.toml`, 'utf8');
  const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const versions = [packageJson.version, tauriConfig.version, cargoVersion, updater.version];

  if (versions.some((version) => version !== versions[0])) {
    throw new Error(`${app.directory} version mismatch: ${versions.join(', ')}`);
  }

  const platform = updater.platforms?.['windows-x86_64'];
  if (!platform?.url?.includes(`v${versions[0]}/`) || !platform.signature) {
    throw new Error(`${app.directory} updater metadata does not match v${versions[0]}`);
  }

  console.log(`${app.directory}: v${versions[0]} metadata verified`);
}
