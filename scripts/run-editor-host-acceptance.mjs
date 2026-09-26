import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { downloadAndUnzipVSCode } from '@vscode/test-electron';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const extension = join(root, 'editors', 'vscode-zryna');
const suite = join(extension, 'test', 'host-acceptance.cjs');
const [setupArg, digest, codeArg] = process.argv.slice(2);
if (!setupArg || !/^[a-f0-9]{64}$/.test(digest ?? '')) {
  console.error('Usage: node scripts/run-editor-host-acceptance.mjs <verified-setup-directory> <setup.json-sha256> [VS-Code-executable]');
  process.exit(2);
}

const setup = resolve(setupArg);
const temporary = await mkdtemp(join(tmpdir(), 'zryna-editor-host-'));
const workspace = join(temporary, 'workspace');
const resultPath = join(temporary, 'result.json');
const configPath = join(temporary, 'config.json');
const started = Date.now();
let timer;
try {
  await mkdir(workspace);
  const actual = createHash('sha256').update(await readFile(join(setup, 'setup.json'))).digest('hex');
  if (actual !== digest) throw new Error('Setup digest differs from the supplied reviewed digest.');
  const code = codeArg ? resolve(codeArg) : await downloadAndUnzipVSCode({
    version: '1.138.0', cachePath: join(tmpdir(), 'zryna-vscode-test-cache'),
  });
  const downloadMs = Date.now() - started;
  await writeFile(configPath, JSON.stringify({ setup, digest, resultPath }));
  const args = [
    '--new-window', '--skip-welcome', '--skip-release-notes', '--disable-updates',
    `--user-data-dir=${join(temporary, 'user-data')}`,
    `--extensions-dir=${join(temporary, 'extensions')}`,
    `--extensionDevelopmentPath=${extension}`,
    `--extensionTestsPath=${suite}`,
    workspace,
  ];
  const child = spawn(code, args, {
    env: { ...process.env, ZRYNA_EDITOR_HOST_CONFIG: configPath },
    stdio: ['ignore', 'pipe', 'pipe'], windowsHide: false,
  });
  let output = '';
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => {
    output += bytes.toString();
    if (output.length > 100000) output = output.slice(-100000);
  });
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => { child.kill(); reject(new Error('VS Code extension-host acceptance timed out after 6 minutes.')); }, 360000);
  });
  const exit = new Promise((fulfill, reject) => {
    child.once('error', reject);
    child.once('exit', (status, signal) => fulfill({ status, signal }));
  });
  const ended = await Promise.race([exit, timeout]);
  const result = await readFile(resultPath, 'utf8').then(JSON.parse, () => null);
  console.log(JSON.stringify({ durationMs: Date.now() - started, setupAndDownloadMs: downloadMs,
    editorExit: ended, result, ...(ended.status !== 0 || !result?.passed ? { output: output.slice(-12000) } : {}) }, null, 2));
  if (ended.status !== 0 || !result?.passed) process.exitCode = 1;
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  clearTimeout(timer);
  await rm(temporary, { recursive: true, force: true });
}
