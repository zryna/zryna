import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, copyFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { createRequire } from 'node:module';
import { capture } from './installed-editor/files.mjs';
import { EDITOR_ARCHIVES } from './installed-editor/pins.mjs';
import { command, startEditor } from './installed-editor/process.mjs';
import { descriptor, inventory, installedDirectory, ordinaryRoot, prepareSetup,
  verifyEditor, verifyEditorArchive, verifyInstalled } from './installed-editor/materials.mjs';
import { trustFreshFolder, waitJson } from './installed-editor/trust.mjs';

const require = createRequire(import.meta.url);
const { editorCli } = require('../portable-setup/setup.cjs');
const source = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const runner = join(source, 'scripts/m6/installed-editor');
const names = ['--handoff', '--editor-archive', '--source-commit', '--source-tree', '--source-epoch',
  '--setup-archive-sha256', '--setup-manifest-sha256', '--server-sha256', '--vsix-sha256', '--output'];

export function argumentsFor(args) {
  const prepareOnly = args.at(-1) === '--prepare-only';
  if (prepareOnly) args = args.slice(0, -1);
  assert.equal(args.length, names.length * 2);
  assert(names.every((name, index) => args[index * 2] === name));
  const values = Object.fromEntries(names.map((name, index) => [name.slice(2), args[index * 2 + 1]]));
  for (const key of ['handoff', 'editor-archive', 'output']) {
    assert(isAbsolute(values[key]) && values[key] === resolve(values[key]));
  }
  assert(/^[a-f0-9]{40}$/.test(values['source-commit']));
  assert(/^[a-f0-9]{40}$/.test(values['source-tree']));
  assert(/^[0-9]{1,12}$/.test(values['source-epoch']));
  for (const key of ['setup-archive-sha256', 'setup-manifest-sha256', 'server-sha256', 'vsix-sha256']) {
    assert(/^[a-f0-9]{64}$/.test(values[key]));
  }
  return { ...values, prepareOnly };
}

async function run(values) {
  assert(process.version === 'v22.22.1' && process.arch === 'x64');
  assert(['linux', 'win32'].includes(process.platform));
  const output = values.output;
  await ordinaryRoot(dirname(output));
  await mkdir(output, { recursive: false, mode: 0o700 });
  const stage = await mkdtemp(join(tmpdir(), 'zryna-m6-installed-editor-'));
  const logs = join(output, 'commands');
  await mkdir(logs);
  const prepareDeadline = performance.now() + 600000;
  const options = { cwd: source, env: { ...process.env }, output: logs, deadline: prepareDeadline };
  let editorProcess, primaryFailure, cleanupFailure, result, prepared;
  try {
    const head = (await command('git', ['rev-parse', 'HEAD'], { ...options, name: 'source-head' })).trim();
    const tree = (await command('git', ['show', '-s', '--format=%T', 'HEAD'],
      { ...options, name: 'source-tree' })).trim();
    assert.equal(head, values['source-commit']);
    assert.equal(tree, values['source-tree']);
    assert.equal((await command('git', ['status', '--porcelain', '--untracked-files=all'],
      { ...options, name: 'source-clean' })).trim(), '');
    const setup = await prepareSetup(values.handoff, { commit: head, tree,
      epoch: Number(values['source-epoch']), archive: values['setup-archive-sha256'],
      manifest: values['setup-manifest-sha256'], server: values['server-sha256'],
      vsix: values['vsix-sha256'] }, stage, prepareDeadline);
    await copyFile(join(stage, 'setup-authentication.json'), join(output, 'setup-authentication.json'));
    const protectedEditor = await capture(values['editor-archive'], join(stage, 'authenticated-editor.archive'),
      EDITOR_ARCHIVES[process.platform], prepareDeadline);
    const archive = await verifyEditorArchive(protectedEditor.captured.path, prepareDeadline);
    archive.originalCapture = protectedEditor.source;
    const editorRoot = join(stage, 'editor');
    await mkdir(editorRoot);
    await command('tar', ['-xf', protectedEditor.captured.path, '-C', editorRoot],
      { ...options, name: 'extract-authenticated-editor' });
    const editor = await verifyEditor(editorRoot, prepareDeadline);
    prepared = { source: { commit: head, tree }, stage, setup, editor, archive,
      status: 'MATERIAL PREPARATION ONLY; no installed-host pass',
      note: 'Editor archive pins authenticate extracted bytes; executable hash is an observation.' };
    await writeFile(join(output, 'PREPARED.json'), `${JSON.stringify(prepared, null, 2)}\n`, { flag: 'wx' });
    if (values.prepareOnly) return;
    const userData = join(stage, 'profile'), extensions = join(stage, 'extensions');
    const workspace = join(stage, `m6-exact-folder-${basename(stage)}`);
    await mkdir(join(userData, 'User'), { recursive: true });
    await mkdir(extensions);
    await mkdir(workspace);
    await writeFile(join(userData, 'User/settings.json'), `${JSON.stringify({
      'zryna.installationPath': setup.setup, 'zryna.installationDigest': setup.digest,
      '[zryna]': { 'editor.defaultFormatter': 'zryna.zryna' },
    }, null, 2)}\n`, { flag: 'wx' });
    const cliArgs = ['--user-data-dir', userData, '--extensions-dir', extensions];
    const cli = process.platform === 'win32' ? [editorCli(editor.executable)] : [];
    const cliEnv = { ...process.env, ...(process.platform === 'win32' ? { ELECTRON_RUN_AS_NODE: '1' } : {}) };
    await command(editor.executable, [...cli, ...cliArgs, '--install-extension', setup.vsix],
      { ...options, env: cliEnv, name: 'install-exact-vsix' });
    const listed = await command(editor.executable, [...cli, ...cliArgs, '--list-extensions', '--show-versions'],
      { ...options, env: cliEnv, name: 'list-installed-vsix' });
    assert.equal(listed.trim(), 'zryna.zryna@0.5.0');
    const installedExtension = await installedDirectory(extensions);
    const installedInventory = await verifyInstalled(installedExtension, setup.payload);
    await writeFile(join(output, 'installed-inventory.json'),
      `${JSON.stringify(installedInventory, null, 2)}\n`, { flag: 'wx' });
    const config = { setup: setup.setup, digest: setup.digest, installedExtension,
      payload: setup.payload, workspace, initialPath: join(output, 'initial-untrusted.json'),
      resultPath: join(output, 'host-result.json'), failurePath: join(output, 'host-failure.json') };
    const configPath = join(stage, 'host-config.json');
    await writeFile(configPath, JSON.stringify(config), { flag: 'wx' });
    const deadline = performance.now() + 360000;
    const args = ['--new-window', '--skip-welcome', '--skip-release-notes', '--disable-updates',
      '--locale=en-US', '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0',
      `--user-data-dir=${userData}`, `--extensions-dir=${extensions}`,
      `--extensionDevelopmentPath=${runner}`, workspace];
    const env = { ...process.env, ZRYNA_EDITOR_HOST_CONFIG: configPath };
    delete env.ELECTRON_RUN_AS_NODE;
    editorProcess = await startEditor(editor.executable, args, { cwd: stage, env,
      output: logs, deadline });
    const failed = () => editorProcess.failure() ?? (editorProcess.ended()
      ? new Error('Editor ended before required installed-host results.') : null);
    const trust = await trustFreshFolder({ userData, workspaceName: basename(workspace),
      initialPath: config.initialPath, deadline, failed });
    const observed = await waitJson(config.resultPath, deadline, failed);
    assert.equal(observed.passed, true);
    assert.equal(observed.checks.length, 9);
    assert.equal(observed.generated.length, 3);
    await mkdir(join(output, 'generated'));
    for (const [index, artifact] of observed.generated.entries()) {
      const actual = await descriptor(artifact.file);
      assert.equal(actual.bytes, artifact.bytes);
      assert.equal(actual.sha256, artifact.sha256);
      artifact.retained = (await capture(artifact.file,
        join(output, 'generated', `${index}.${artifact.target === 'javascript' ? 'mjs' : 'wasm'}`),
        { bytes: artifact.bytes, sha256: artifact.sha256 }, deadline)).captured;
    }
    assert.deepEqual(await inventory(installedExtension), installedInventory);
    result = { format: 'zryna.m6-installed-editor.v1', source: { commit: head, tree },
      editor: { version: editor.version, binary: editor.descriptor, productCommit: editor.productCommit,
        archive, inventorySha256: editor.inventory.sha256 }, setup: { digest: setup.digest,
        installedExtension, payload: setup.payload }, trust, observed,
      platform: { os: process.platform, arch: process.arch,
        display: process.platform === 'linux' ? 'Native Linux editor under Xvfb virtual display' : 'Native Windows GUI process' },
      passed: 9, failed: 0, skipped: 0, productionAdmission: 'not granted by this receipt' };
  } catch (error) { primaryFailure = String(error.stack ?? error); }
  finally {
    if (editorProcess) {
      try { await editorProcess.finish(); }
      catch (error) { cleanupFailure = String(error.stack ?? error); }
    }
    const final = { ...(result ?? {}), accepted: Boolean(result) && !primaryFailure && !cleanupFailure,
      primaryFailure, cleanupFailure, preparedOnly: values.prepareOnly,
      stageRetained: stage, note: 'Transient profile/materials retained for bounded owner cleanup; not public runtime authority.' };
    await writeFile(join(output, 'RESULT.json'), `${JSON.stringify(final, null, 2)}\n`, { flag: 'wx' });
    if ((!values.prepareOnly && !final.accepted) || primaryFailure || cleanupFailure) process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await run(argumentsFor(process.argv.slice(2)));
}
