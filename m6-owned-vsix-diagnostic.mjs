// Isolated diagnostic only: original package/install/strict inventory path, no GUI.
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, realpath, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { createRequire } from 'node:module';
import { acquire } from './scripts/m6/installed-editor-acquire.mjs';
import { capture, descriptor, ordinaryRoot, readBytes } from './scripts/m6/installed-editor/files.mjs';
import { verifyEditor, verifyEditorArchive, inventory, installedDirectory, verifyInstalled,
  sha } from './scripts/m6/installed-editor/materials.mjs';
import { canonicalVsix, vsixEntries } from './scripts/portable-setup/vsix.mjs';
import { EDITOR_ARCHIVES } from './scripts/m6/installed-editor/pins.mjs';
import { command } from './scripts/m6/installed-editor/process.mjs';
const require = createRequire(import.meta.url);
const { editorCli } = require('./scripts/portable-setup/setup.cjs');
assert.equal(process.platform, 'win32');
assert.equal(process.version, 'v22.22.1');
assert.equal(process.arch, 'x64');
const source = await realpath(resolve('.'));
const root = await realpath(process.env.M6_VSIX_DIAGNOSTIC_ROOT);
await ordinaryRoot(root);
async function retain(name, value) {
  const bytes = Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
  assert(bytes.length <= 16 * 1024 * 1024, 'Diagnostic JSON cap.');
  await writeFile(join(root, name), bytes, { flag: 'wx' });
}
let accepted = false, stage = null, phase = 'ACQUISITION', error = null;
let inventoryRetained = false, listedCorrectly = false;
try {
  phase = 'PIN_SOURCE_OVERLAY';
  const preflightDeadline = performance.now() + 15000;
  const sourceInputPins = [];
  for (const [path, bytes, sha256] of [
    ['scripts/m6/installed-editor/materials.mjs', 9618, '7f3523b5552a664235e1d695e1d9d27e6a482e4c04f43873a8de5e78f3e77f68'],
    ['scripts/m6/installed-editor-acquire.mjs', 2874, '108669518da3b6220c4f32315a8cdd38d00fdf1233bae650fe3d68538bbef02b'],
  ]) {
    const observed = await descriptor(join(source, path), preflightDeadline);
    assert.equal(observed.bytes, bytes);
    assert.equal(observed.sha256, sha256);
    sourceInputPins.push(observed);
  }
  await retain('SOURCE_INPUTS.json', { source: process.env.GITHUB_SHA, sourceInputPins,
    scope: 'Exact selected material verifier and previously reviewed fixed UCRT64 acquisition overlay' });
  phase = 'ACQUISITION';
  const archivePath = await acquire(join(root, 'material'));
  const deadline = performance.now() + 600000;
  stage = await mkdtemp(join(await realpath(tmpdir()), 'm6-vsix-installation-diagnostic-'));
  const commands = join(root, 'commands');
  await mkdir(commands);
  const options = { cwd: source, stage, env: { ...process.env }, output: commands, deadline };
  phase = 'PROTECTED_ARCHIVE';
  const protectedEditor = await capture(archivePath, join(stage, 'authenticated-editor.archive'),
    EDITOR_ARCHIVES.win32, deadline);
  const archive = await verifyEditorArchive(protectedEditor.captured.path, deadline);
  archive.originalCapture = protectedEditor.source;
  const editorRoot = join(stage, 'editor');
  await mkdir(editorRoot);
  await ordinaryRoot(editorRoot);
  const helperSource = join(source, 'scripts/m6/installed-editor/extract-editor-windows.ps1');
  const helperPin = await descriptor(helperSource, deadline);
  assert.equal(helperPin.bytes, 1893);
  assert.equal(helperPin.sha256, 'a68bf4b3186ca4b2d733a2e32225838959a2ee79a12b36c2e542f61b530afd72');
  const helper = await capture(helperSource, join(stage, 'extract-editor-windows.ps1'), helperPin, deadline);
  const runtime = join(process.env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe');
  await retain('EXTRACTION_BEFORE.json', { archive, helper,
    runtime: await descriptor(await realpath(runtime), deadline), source: process.env.GITHUB_SHA, stage });
  phase = 'OWNED_EXTRACTION';
  await command(runtime, ['-NoLogo', '-NoProfile', '-NonInteractive', '-File',
    helper.captured.path, protectedEditor.captured.path, editorRoot],
  { ...options, name: 'extract-authenticated-editor' });
  phase = 'VERIFY_EDITOR';
  // Original strict verifier with the separately pinned existing-selector Windows app-root correction.
  const editor = await verifyEditor(editorRoot, deadline);
  await retain('EXTRACTED_EDITOR.json', editor);
  phase = 'PACKAGE_CURRENT_SOURCES';
  const packaged = join(root, 'current-editor.vsix');
  await command(process.execPath, [join(source, 'scripts/package-editor.mjs'), '--out', packaged],
    { ...options, name: 'package-current-editor-vsix' });
  const vsixPin = await descriptor(packaged, deadline, 4 * 1024 * 1024);
  const vsix = await capture(packaged, join(stage, 'current-editor.vsix'), vsixPin, deadline);
  const bytes = await readBytes(vsix.captured.path, deadline, 4 * 1024 * 1024);
  assert.equal(bytes.length, vsixPin.bytes);
  assert.equal(sha(bytes), vsixPin.sha256);
  assert(canonicalVsix(bytes).equals(bytes), 'Original packager must emit canonical VSIX.');
  const entries = vsixEntries(bytes);
  const sourceFiles = ['CHANGELOG.md', 'LICENSE', 'README.md', 'package.json',
    'src/connection.cjs', 'src/extension.cjs', 'src/installation.cjs', 'src/run-command.cjs',
    'src/run-input.cjs', 'src/run-process.cjs', 'src/run-project.cjs', 'syntaxes/zryna.tmLanguage.json'];
  assert.equal(entries.length, sourceFiles.length + 2);
  const sourcePins = [];
  for (const path of sourceFiles) {
    const packagedName = { 'CHANGELOG.md': 'changelog.md', 'README.md': 'readme.md',
      LICENSE: 'LICENSE.txt' }[path] ?? path;
    const entry = entries.find(entry => entry.name === `extension/${packagedName}`);
    const sourcePath = join(source, 'editors/vscode-zryna', path);
    const actual = await readBytes(sourcePath, deadline, 1024 * 1024);
    assert(entry && entry.data.equals(actual), `VSIX source binding differs: ${path}`);
    sourcePins.push({ path, bytes: actual.length, sha256: sha(actual) });
  }
  const payload = Object.fromEntries(entries.filter(entry => entry.name.startsWith('extension/'))
    .map(entry => [entry.name.slice(10), { bytes: entry.data.length, sha256: sha(entry.data) }]));
  assert.equal(Object.keys(payload).length, 12);
  await retain('VSIX_EXPECTED.json', { diagnosticOnly: true, qualification: false,
    source: process.env.GITHUB_SHA, vsix, sourcePins, payload });
  phase = 'INSTALL_CURRENT_VSIX';
  const userData = join(stage, 'profile'), extensions = join(stage, 'extensions');
  await mkdir(userData);
  await mkdir(extensions);
  const cli = editorCli(editor.executable);
  const cliPin = await descriptor(await realpath(cli), deadline);
  await retain('INSTALL_BEFORE.json', { editor: editor.descriptor, cli: cliPin,
    vsix: vsix.captured, userData, extensions, deadlineScope: 'Original single600s preparation clock' });
  const cliArgs = ['--user-data-dir', userData, '--extensions-dir', extensions];
  const cliEnv = { ...process.env, ELECTRON_RUN_AS_NODE: '1' };
  await command(editor.executable, [cli, ...cliArgs, '--install-extension', vsix.captured.path],
    { ...options, env: cliEnv, name: 'install-current-vsix' });
  phase = 'LIST_CURRENT_VSIX';
  const listed = await command(editor.executable, [cli, ...cliArgs, '--list-extensions', '--show-versions'],
    { ...options, env: cliEnv, name: 'list-installed-vsix' });
  assert.equal(listed.trim(), 'zryna.zryna@0.5.0');
  listedCorrectly = true;
  phase = 'RETAIN_INSTALLED_INVENTORY';
  await retain('EXTENSIONS_ROOT_INVENTORY.json', await inventory(extensions, deadline));
  const installed = await installedDirectory(extensions);
  const actual = await inventory(installed, deadline);
  await retain('INSTALLED_INVENTORY_BEFORE_VERIFY.json', { installed, actual });
  const packagePath = join(installed, 'package.json');
  const packagePin = await descriptor(packagePath, deadline, 1024 * 1024);
  const rawPackage = await readBytes(packagePath, deadline, 1024 * 1024);
  assert.equal(rawPackage.length, packagePin.bytes);
  assert.equal(sha(rawPackage), packagePin.sha256);
  await writeFile(join(root, 'INSTALLED_PACKAGE_JSON.raw'), rawPackage, { flag: 'wx' });
  await retain('INSTALLED_PACKAGE_PIN.json', { original: packagePin,
    retained: { path: 'INSTALLED_PACKAGE_JSON.raw', bytes: rawPackage.length, sha256: sha(rawPackage) } });
  inventoryRetained = true;
  phase = 'VERIFY_INSTALLED';
  // All actual data above survives any original strict validator rejection.
  const verified = await verifyInstalled(installed, payload, deadline);
  await retain('STRICT_INSTALLED_VERIFICATION.json', verified);
  accepted = true;
} catch (failure) {
  error = String(failure.stack ?? failure).slice(0, 16384);
}
await retain('DIAGNOSTIC_RESULT.json', { diagnosticOnly: true, qualification: false,
  scope: 'Fresh current-source canonical VSIX packaging and installation diagnostic only',
  acceptedVsixInstallationOnly: accepted, installedInventoryRetainedBeforeStrictValidation: inventoryRetained,
  listedCorrectly, installedHostAccepted: false, hostNineChecksExecuted: 0, editorGuiExecuted: false,
  source: process.env.GITHUB_SHA, phase, stage, error });
if (!accepted) process.exitCode = 1;
