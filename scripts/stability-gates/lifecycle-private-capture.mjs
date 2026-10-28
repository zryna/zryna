import assert from 'node:assert/strict';
import { mkdirSync, opendirSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { captureLifecyclePrebuild } from './lifecycle-owner-capture.mjs';
import { securityRustMaterials } from '../distribution/rust-security-materials.mjs';
import { pinFile } from './release-build-input.mjs';
import { execute } from './process.mjs';
import { sha256, readSafe } from './input.mjs';
import { save, download } from './lifecycle-private-io.mjs';
import { encodeTar } from '../distribution/archive-tar.mjs';

async function packCapture(output, base) {
  const pending = [''], files = [];
  let entries = 0;
  while (pending.length) {
    const prefix = pending.pop(), directory = opendirSync(join(output, prefix), { bufferSize: 1 });
    try {
      for (let entry; (entry = directory.readSync()) !== null;) {
        assert(++entries <= 2 * 512 * 12);
        const path = prefix ? prefix + '/' + entry.name : entry.name;
        if (path === 'target') continue;
        assert(!entry.isSymbolicLink());
        if (entry.isDirectory()) pending.push(path);
        else {
          assert(entry.isFile()); assert(files.length < 1024);
          files.push({ path, mode: 0o644, data: readSafe(output, path, 16 * 1024 ** 2) });
        }
      }
    } finally { directory.closeSync(); }
  }
  files.sort((a, b) => a.path < b.path ? -1 : 1);
  const pins = [];
  for (const after of [false, true]) {
    const data = await encodeTar('capture', files.filter(file => file.path.startsWith('licenses-after/') === after), 0);
    const path = after ? 'capture-after.tar.gz' : 'capture-main.tar.gz';
    save(join(base, path), data); pins.push({ path, bytes: data.length, sha256: sha256(data) });
  }
  return pins;
}

export async function capture(base, input) {
  const root = realpathSync(process.env.GITHUB_WORKSPACE), output = join(base, 'capture');
  const rustup = (tool) => execFileSync('rustup', ['which', '--toolchain', '1.97.1', tool], { encoding: 'utf8' }).trim();
  const tools = { cargo: rustup('cargo'), rustc: rustup('rustc'), git: '/usr/bin/git' };
  const environment = { PATH: dirname(tools.rustc) + ':/usr/bin:/bin', HOME: process.env.HOME,
    CARGO_HOME: process.env.CARGO_HOME || join(process.env.HOME, '.cargo'),
    RUSTUP_HOME: process.env.RUSTUP_HOME || join(process.env.HOME, '.rustup'), LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8',
    RUSTC: tools.rustc, RUSTUP_TOOLCHAIN: '1.97.1', CARGO_TARGET_DIR: join(output, 'target'),
    CARGO_NET_OFFLINE: 'true', CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' };
  const license = securityRustMaterials('x86_64-unknown-linux-gnu').flatMap(pin => pin.files)
    .find(file => file.origin.startsWith('https://raw.githubusercontent.com/bytecodealliance/wasmtime/'));
  await download({ url: license.origin, bytes: license.size, sha256: license.sha256 }, join(base, 'upstream-license'));
  const source = { commit: process.env.GITHUB_SHA, tree: execFileSync(tools.git, ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8' }).trim() };
  const discovery = await captureLifecyclePrebuild({ root, output, source, tools, environment, upstreamLicense: join(base, 'upstream-license') });
  mkdirSync(join(base, 'binaries'));
  for (const key of ['test', 'compiler']) {
    const pin = discovery.discoveredArtifacts[key], bytes = readSafe(dirname(pin.path), pin.path.split('/').at(-1), 128 * 1024 ** 2);
    assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256); save(join(base, 'binaries', key), bytes);
  }
  const cc = realpathSync('/usr/bin/cc'), fixed = join(root, 'examples/playground/restricted/daemon');
  const sources = [cc, join(fixed, 'lifecycle-owner/lifecycle-supervisor.c'), join(fixed, 'supervisor_sys.h')];
  const before = sources.map(path => pinFile(path));
  const args = ['-std=c11', '-O2', '-fno-builtin', '-fno-stack-protector', '-fno-pie', '-no-pie', '-nostdlib', '-static', '-Wl,--build-id=none', sources[1], '-o', join(base, 'binaries/supervisor')];
  save(join(base, 'cc-command.json'), { executable: cc, args, before });
  const result = await execute(cc, args, { root, timeoutMs: 30000, env: { PATH: '/usr/bin:/bin', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8' } });
  save(join(base, 'cc-stdout.raw'), result.stdout); save(join(base, 'cc-stderr.raw'), result.stderr);
  save(join(base, 'cc-result.json'), { ...result, stdout: undefined, stderr: undefined });
  assert.deepEqual(sources.map(path => pinFile(path)), before);
  assert.deepEqual([result.exitCode, result.signal, result.error], [0, null, null]);
  const supervisor = pinFile(join(base, 'binaries/supervisor'));
  mkdirSync(join(base, 'originals'));
  save(join(base, 'download-selection.json'), input);
  for (const row of input.downloads) {
    await download(row, join(base, 'originals', row.name));
  }
  save(join(base, 'capture-complete.json'), { source, artifacts: discovery.discoveredArtifacts,
    supervisor, rawCapture: await packCapture(output, base),
    assetsSha256: sha256(Buffer.from(JSON.stringify(input) + '\n')) });
}
