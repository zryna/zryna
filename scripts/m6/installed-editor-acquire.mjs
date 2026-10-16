import assert from 'node:assert/strict';
import { mkdir, realpath, rename, statfs, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { EDITOR_ARCHIVES, EDITOR_VERSION } from './installed-editor/pins.mjs';
import { verifyEditorArchive, ordinaryRoot } from './installed-editor/materials.mjs';
import { command } from './installed-editor/process.mjs';

export async function acquire(output) {
  assert(process.version === 'v22.22.1' && process.arch === 'x64');
  assert(isAbsolute(output) && output === resolve(output));
  const pin = EDITOR_ARCHIVES[process.platform];
  assert(pin, 'Unsupported editor acquisition platform.');
  await ordinaryRoot(dirname(output));
  const volume = await statfs(dirname(output), { bigint: true });
  assert(volume.bavail * volume.bsize >= BigInt(pin.bytes) + 512n * 1024n * 1024n,
    'Editor acquisition needs a separate 512 MiB free-space reserve.');
  await mkdir(output, { recursive: false, mode: 0o700 });
  const partial = join(output, 'editor.partial');
  const deadline = performance.now() + 250000;
  let client = 'curl';
  if (process.platform === 'win32') {
    // Trusted hosted bootstrap location; never a supplied material or PATH candidate.
    const selected = process.env.ProgramFiles;
    assert(typeof selected === 'string' && selected.length <= 32768 && isAbsolute(selected));
    const programFiles = await realpath(selected);
    await ordinaryRoot(programFiles);
    client = join(programFiles, 'Git', 'mingw64', 'bin', 'curl.exe');
    await ordinaryRoot(dirname(client));
    assert.equal(await realpath(client), client);
  }
  const stdout = await command(client,
    ['--fail', '--location', '--proto', '=https', '--proto-redir', '=https', '--max-time', '240',
      '--max-filesize', String(pin.bytes), '--output', partial, '--write-out', '%{http_code}', pin.url],
    { cwd: output, env: { ...process.env }, output, name: 'official-editor-download',
      deadline });
  assert.equal(stdout, '200');
  const actual = await verifyEditorArchive(partial);
  const archive = join(output, process.platform === 'win32' ? 'editor.zip' : 'editor.tar.gz');
  await rename(partial, archive);
  await writeFile(join(output, 'EDITOR_ARCHIVE.json'), `${JSON.stringify({
    version: EDITOR_VERSION, publisherSelection: pin, actual: { ...actual, path: archive },
    proxyAndCa: 'Inherited normal client proxy and TLS verification; no bypass.',
    status: 'Authenticated artifact acquisition only; no extraction/editor execution.',
  }, null, 2)}\n`, { flag: 'wx' });
  return archive;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3);
  process.stdout.write(`${await acquire(process.argv[2])}\n`);
}
