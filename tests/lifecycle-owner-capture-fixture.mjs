// In-memory adapter observations only. No native binary, Cargo, Python or filesystem execution.
import assert from 'node:assert/strict';
import { graphFixture } from '../scripts/m6/compiler-release/material-fixtures.mjs';
import { sha256 } from '../scripts/stability-gates/input.mjs';

const PACKAGE = 'zryna-playground-compiler';
const raw = value => Buffer.from(JSON.stringify(value) + '\n');
export function captureFixture() {
  const { metadata, sourceRoot: root } = graphFixture();
  const output = '/workspace/synthetic-capture', target = output + '/target';
  const tools = { cargo: '/tools/cargo', rustc: '/tools/rustc', git: '/tools/git' };
  const environment = { PATH: '/tools:/usr/bin:/bin', HOME: '/home/operator', CARGO_HOME: '/cache/cargo',
    RUSTUP_HOME: '/cache/rustup', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', RUSTC: tools.rustc,
    RUSTUP_TOOLCHAIN: '1.97.1', CARGO_TARGET_DIR: target, CARGO_NET_OFFLINE: 'true',
    CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' };
  const input = { root, output, tools, environment, upstreamLicense: '/licenses/wasmtime',
    source: { commit: 'a'.repeat(40), tree: 'b'.repeat(40) } };
  const source = structuredClone(input.source);
  const directory = root + '/apps/' + PACKAGE, id = 'path+file://' + directory + '#0.1.0';
  metadata.packages[0].id = id; metadata.packages[0].dependencies = [];
  metadata.workspace_members = [id]; metadata.resolve.nodes[0].id = id;
  metadata.packages[1].targets = [{ name: 'build-script-build', kind: ['custom-build'] }];
  const artifacts = { test: target + '/release/deps/lifecycle-0123456789abcdef', compiler: target + '/release/' + PACKAGE };
  const rows = [['test', 'lifecycle', 'test', '/tests/lifecycle.rs'], ['compiler', PACKAGE, 'bin', '/src/main.rs']]
    .map(([key, name, kind, source]) => ({ reason: 'compiler-artifact', package_id: id,
      target: { name, kind: [kind], crate_types: ['bin'], src_path: directory + source },
      profile: { test: key === 'test', opt_level: '3' }, executable: artifacts[key], filenames: [artifacts[key]] }));
  rows.push({ reason: 'build-script-executed', package_id: 'crate' }, { reason: 'build-finished', success: true });
  const files = new Map(), calls = [], state = { after: false, prepared: false, metadataCalls: 0, treeCalls: 0 };
  const hooks = { run: () => {}, snapshot: () => {}, materials: () => {} };
  const writer = () => (name, value) => {
    assert(!files.has(name), 'create-only observation');
    const bytes = Buffer.isBuffer(value) ? Buffer.from(value) : raw(value); files.set(name, bytes);
    return { path: name, bytes: bytes.length, sha256: sha256(bytes) };
  };
  const adapters = {
    host: () => ({ platform: 'linux', arch: 'x64', node: 'v22.22.1' }),
    prepare: () => { assert(!state.prepared, 'fresh output and target'); state.prepared = true; },
    writer,
    census: () => {
      const value = { ...source, files: 1, bytes: 12, inventorySha256: 'c'.repeat(64), shards: [] };
      hooks.snapshot('source', value, state); return value;
    },
    pin: path => {
      const prefix = Buffer.alloc(64); Buffer.from([127, 69, 76, 70, 2, 1]).copy(prefix); prefix.writeUInt16LE(62, 18);
      const value = { path, bytes: 64, sha256: sha256(Buffer.from('synthetic ' + path)), mode: '755', prefixHex: prefix.toString('hex') };
      hooks.snapshot(path, value, state); return value;
    },
    configuration: () => {
      // Linux coordinates are fixed even when these inert tests execute on Windows.
      const directories = ['/cache/cargo', root + '/.cargo', '/.cargo'];
      const value = directories.flatMap(path => ['config', 'config.toml'].map(name => ({ path: path + '/' + name, present: false })));
      hooks.snapshot('configuration', value, state); return value;
    },
    workspace: () => hooks.snapshot('workspace', null, state),
    treeProof: bytes => ({ syntheticTreeObservation: sha256(bytes) }),
    materials: () => {
      const value = { closure: raw({ syntheticCacheCorrespondence: true }),
        licenses: [{ path: 'licenses/rust/fixture/LICENSE', data: Buffer.from('inert license') }] };
      hooks.materials(value, state); return value;
    },
    run: async (executable, args, options) => {
      let name, stdout = Buffer.alloc(0), stderr = Buffer.alloc(0);
      if (args[0] === '--version') { name = 'cargo-version'; stdout = Buffer.from('cargo 1.97.1 (c980f4866 2026-06-30)\n'); }
      else if (args[0] === '-vV') { name = 'rustc-version'; stdout = Buffer.from('rustc 1.97.1 (8bab26f4f 2026-07-14)\nhost: x86_64-unknown-linux-gnu\n'); }
      else if (args[0] === '--print') { name = 'rustc-' + args[1]; stdout = Buffer.from('/tools/sysroot\n'); }
      else if (args[0] === 'metadata') { name = 'metadata-' + (++state.metadataCalls === 1 ? 'before' : 'after'); stdout = raw(metadata); }
      else if (args[0] === 'tree') {
        name = 'tree-' + (++state.treeCalls === 1 ? 'before' : 'after');
        stdout = Buffer.from(`${PACKAGE} v0.1.0 (${directory})\nautocfg v1.5.1\n`);
      } else {
        assert.equal(args[0], 'test'); name = 'build'; state.after = true;
        stdout = Buffer.concat([Buffer.from('[autocfg 1.5.1] cargo:rustc-cfg=synthetic\n'), ...rows.map(raw)]);
        stderr = Buffer.from(`     Running \`CARGO_BIN_EXE_${PACKAGE}=${artifacts.compiler} /tools/rustc --crate-name lifecycle --test ${directory}/tests/lifecycle.rs\`\n`);
      }
      assert(files.has(name + '.command.json'), 'command durable before child invocation');
      calls.push({ name, executable, args: [...args], options: structuredClone(options) });
      const result = { exitCode: 0, signal: null, error: null, elapsedMs: 1, stdout, stderr };
      hooks.run(name, result, state); return result;
    },
  };
  return { input, adapters, files, calls, hooks, state, metadata, artifacts };
}
