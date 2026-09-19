'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { verifyInstallation } = require('./installation.cjs');

// This entry point belongs to an already authenticated extracted candidate. It never changes
// an existing editor profile or project. Workspace trust remains the editor's own decision.
const args = process.argv.slice(2);
if (![2, 6].includes(args.length) || args[0] !== '--digest'
  || (args.length === 6 && (args[2] !== '--editor' || args[4] !== '--profile'))) {
  throw new Error('Usage: install --digest <reviewed setup.json SHA256> [--editor <Code executable> --profile <new absolute directory>]');
}
const root = __dirname;
const installation = verifyInstallation(root, args[1]);
if (args.length === 2) {
  console.log(`Verified ${installation.manifest.candidate}; compiler 0.2.3; server/editor 0.3.0.`);
} else {
  const editor = args[3];
  const profile = args[5];
  if (![editor, profile].every(value => path.isAbsolute(value) && path.resolve(value) === value)
    || /\.(cmd|bat)$/i.test(editor) || fs.existsSync(profile)
    || profile === root || profile.startsWith(`${root}${path.sep}`)) {
    throw new Error('Select an absolute editor executable and a new profile outside the installation.');
  }
  const parent = path.dirname(profile);
  if (fs.realpathSync.native(parent).toLowerCase() !== parent.toLowerCase()
    || !fs.statSync(parent).isDirectory()) throw new Error('Profile parent must be an existing ordinary directory.');
  fs.mkdirSync(profile, { mode: 0o700 });
  const userData = path.join(profile, 'data');
  const extensions = path.join(profile, 'extensions');
  fs.mkdirSync(path.join(userData, 'User'), { recursive: true });
  fs.mkdirSync(extensions);
  fs.writeFileSync(path.join(userData, 'User', 'settings.json'), `${JSON.stringify({
    'zryna.installationPath': root, 'zryna.installationDigest': args[1],
    '[zryna]': { 'editor.defaultFormatter': 'zryna.zryna' },
  }, null, 2)}\n`, { flag: 'wx' });
  const executableArgs = ['--user-data-dir', userData, '--extensions-dir', extensions,
    '--install-extension', path.join(root, 'editor', 'zryna-0.3.0.vsix')];
  const electron = process.platform === 'win32'
    ? [path.join(path.dirname(editor), 'resources', 'app', 'out', 'cli.js'), ...executableArgs] : executableArgs;
  const result = spawnSync(editor, electron, { shell: false, windowsHide: true, stdio: 'inherit',
    timeout: 120000, env: { ...process.env, ...(process.platform === 'win32' ? { ELECTRON_RUN_AS_NODE: '1' } : {}) } });
  if (result.error || result.status !== 0) throw new Error('Editor installation failed. The new profile was retained for inspection.');
  console.log(`Installed isolated editor profile: ${profile}`);
  console.log('Open your project using the same --user-data-dir and --extensions-dir paths.');
}
