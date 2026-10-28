// Retained Cargo observations only; no execution or configuration filesystem probes.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { TextDecoder } from 'node:util';
import { exact, MAX_LOG } from './input.mjs';
const PACKAGE = 'zryna-playground-compiler';
function text(bytes, limit = MAX_LOG) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= limit, 'bounded retained bytes');
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}
function json(bytes) {
  const raw = text(bytes), value = JSON.parse(raw);
  assert.equal(JSON.stringify(value) + '\n', raw, 'complete producer JSON without duplicate keys'); return value;
}
// Cargo's display uses POSIX shell escaping. Decode words; never evaluate that display.
function cargoWords(command) {
  assert(command.length <= 256 * 1024, 'bounded verbose command');
  const words = []; let value = '', quoted = false, started = false;
  for (let i = 0; i < command.length; i++) {
    const char = command[i];
    if (quoted) {
      if (char === "'") quoted = false; else value += char;
      started = true;
    } else if (char === "'") { quoted = true; started = true; }
    else if (char === '\\') { assert(i + 1 < command.length); value += command[++i]; started = true; }
    else if (char === ' ' || char === '\t') {
      if (started) { words.push(value); value = ''; started = false; }
    } else { assert(!/[;&|<>`$()"]/u.test(char), 'no evaluated verbose shell syntax'); value += char; started = true; }
  }
  assert(!quoted, 'closed verbose quoting'); if (started) words.push(value);
  assert(words.length <= 8192, 'bounded verbose words'); return words;
}
function compilerCall(words, expected) {
  const allowed = new Set(['CARGO', 'CARGO_BIN_NAME', 'CARGO_CRATE_NAME', 'CARGO_MANIFEST_DIR', 'CARGO_MANIFEST_PATH',
    'CARGO_PRIMARY_PACKAGE', 'CARGO_TARGET_TMPDIR', 'LD_LIBRARY_PATH', 'OUT_DIR', 'CARGO_BIN_EXE_' + PACKAGE,
    ...['AUTHORS', 'DESCRIPTION', 'HOMEPAGE', 'LICENSE', 'LICENSE_FILE', 'NAME', 'README', 'REPOSITORY',
      'RUST_VERSION', 'VERSION', 'VERSION_MAJOR', 'VERSION_MINOR', 'VERSION_PATCH', 'VERSION_PRE'].map(name => 'CARGO_PKG_' + name)]);
  const environment = new Map(); let offset = 0;
  for (; offset < words.length; offset++) {
    const match = /^([A-Za-z_][A-Za-z0-9_-]*)=(.*)$/u.exec(words[offset]); if (!match) break;
    assert(allowed.has(match[1]) && !environment.has(match[1]), 'one allowed Cargo environment assignment');
    environment.set(match[1], match[2]);
  }
  assert.equal(words[offset], expected.tools.rustc.path, 'actual compiler command, never a wrapper argument');
  const args = words.slice(offset + 1); assert.deepEqual(args.slice(0, 2), ['--crate-name', 'lifecycle']);
  assert.equal(environment.get('CARGO_BIN_EXE_' + PACKAGE), expected.artifacts.compiler.path, 'Cargo embedded compiler association');
  return args;
}
function configurations(expected, captured) {
  exact(captured, ['before', 'after'], 'Cargo configuration observations');
  const directories = [expected.environment.CARGO_HOME];
  for (let dir = expected.root; ; dir = posix.dirname(dir)) {
    directories.push(posix.join(dir, '.cargo')); if (dir === '/') break;
  }
  const paths = [...new Set(directories.flatMap(dir => ['config', 'config.toml'].map(name => posix.join(dir, name))))];
  assert(paths.length <= 40, 'bounded complete configuration search');
  const absent = paths.map(path => ({ path, present: false }));
  assert.deepEqual(captured.before, absent, 'all source-ancestor and Cargo-home configurations absent before build');
  assert.deepEqual(captured.after, absent, 'same complete absence after build');
}
// Cargo -vv echoes build-script output even with --message-format=json. Account for
// every original line; tagged text never becomes a Cargo JSON record or authority.
export function parseLifecycleCargoOutput(bytes, metadata) {
  assert(Array.isArray(metadata.packages) && metadata.packages.length <= 4096);
  const scripts = new Map();
  for (const pkg of metadata.packages.filter(pkg => pkg.targets?.some(target => target.kind?.includes('custom-build')))) {
    const tag = `${pkg.name} ${pkg.version}`;
    assert(!scripts.has(tag) && typeof pkg.id === 'string', 'unambiguous metadata build-script identity');
    scripts.set(tag, pkg.id);
  }
  const lines = text(bytes).split('\n'); assert.equal(lines.pop(), '');
  assert(lines.length > 0 && lines.length <= 4096, 'bounded complete Cargo output');
  const rows = [], tagged = new Set();
  for (const line of lines) {
    assert(Buffer.byteLength(line) <= 256 * 1024, 'bounded Cargo output line');
    if (line.startsWith('{')) {
      const row = json(Buffer.from(line + '\n'));
      assert(['compiler-artifact', 'compiler-message', 'build-script-executed', 'build-finished'].includes(row.reason), 'known Cargo JSON record');
      rows.push(row);
    } else {
      const match = /^\[([A-Za-z0-9_-]+ [0-9][A-Za-z0-9+._-]*)\] ([^\x00-\x08\x0b-\x1f\x7f]*)$/u.exec(line);
      assert(match && scripts.has(match[1]), 'only original metadata build-script tagged output'); tagged.add(scripts.get(match[1]));
    }
  }
  assert([...tagged].every(id => rows.some(row => row.reason === 'build-script-executed' && row.package_id === id)), 'tagged output has a Cargo build-script record');
  assert.equal(rows.at(-1)?.reason, 'build-finished', 'Cargo completion is the final JSON record');
  assert(lines.at(-1).startsWith('{'), 'no tagged output after Cargo completion');
  return rows;
}
export function validateLifecyclePrebuild(expected, capture) {
  exact(capture, ['source', 'inputs', 'tools', 'versions', 'configuration', 'command', 'termination', 'metadata', 'stdout', 'stderr', 'artifacts'], 'prebuild capture');
  configurations(expected, capture.configuration);
  for (const [key, wanted] of [['source', expected.source], ['inputs', expected.cargoInputs], ['tools', expected.tools]]) {
    exact(capture[key], ['before', 'after'], 'prebuild snapshots');
    assert.deepEqual(capture[key].before, wanted); assert.deepEqual(capture[key].after, wanted, 'stable selected prebuild inputs');
  }
  exact(capture.versions, ['cargo', 'rustc'], 'retained tool versions');
  assert.equal(text(capture.versions.cargo, 16384).trim(), 'cargo 1.97.1 (c980f4866 2026-06-30)');
  const version = text(capture.versions.rustc, 16384).trim().split('\n');
  assert.equal(version[0], 'rustc 1.97.1 (8bab26f4f 2026-07-14)'); assert(version.includes('host: x86_64-unknown-linux-gnu'));
  const args = ['test', '--locked', '--offline', '--release', '-p', PACKAGE, '--test', 'lifecycle', '--no-run', '--message-format=json', '-vv'];
  assert.deepEqual(capture.command, { executable: expected.tools.cargo.path, args, cwd: expected.root, environment: expected.environment });
  assert.deepEqual(capture.termination, { exitCode: 0, signal: null, error: null });
  assert.deepEqual(capture.artifacts, expected.artifacts, 'independently selected actual artifact identities');
  validateLifecycleCargoArtifacts({ root: expected.root, tools: expected.tools, artifacts: expected.artifacts },
    { metadata: capture.metadata, stdout: capture.stdout, stderr: capture.stderr });
}
// Structural replay of original Cargo bytes only. This does not establish tool
// snapshots, source capture, independent artifact selection or execution approval.
export function validateLifecycleCargoArtifacts(expected, capture) {
  exact(expected, ['root', 'tools', 'artifacts'], 'Cargo artifact replay selection');
  exact(capture, ['metadata', 'stdout', 'stderr'], 'original Cargo artifact streams');
  const metadata = json(capture.metadata);
  assert(Array.isArray(metadata.packages) && metadata.packages.length <= 4096);
  const packages = metadata.packages.filter(row => row.name === PACKAGE); assert.equal(packages.length, 1);
  const pkg = packages[0], directory = expected.root + '/apps/' + PACKAGE;
  assert.equal(pkg.version, '0.1.0'); assert.equal(pkg.source, null); assert.equal(pkg.manifest_path, directory + '/Cargo.toml');
  assert([`path+file://${directory}#0.1.0`, `path+file://${directory}#${PACKAGE}@0.1.0`].includes(pkg.id));
  const rows = parseLifecycleCargoOutput(capture.stdout, metadata), finished = rows.filter(row => row.reason === 'build-finished');
  assert.equal(finished.length, 1); assert.equal(finished[0], rows.at(-1)); assert.equal(finished[0].success, true);
  for (const [key, name, kind, sourcePath] of [['test', 'lifecycle', 'test', directory + '/tests/lifecycle.rs'],
    ['compiler', PACKAGE, 'bin', directory + '/src/main.rs']]) {
    const matches = rows.filter(row => row.reason === 'compiler-artifact' && row.package_id === pkg.id && row.target.name === name);
    assert.equal(matches.length, 1); const row = matches[0];
    assert.deepEqual(row.target.kind, [kind]); assert.deepEqual(row.target.crate_types, ['bin']); assert.equal(row.target.src_path, sourcePath);
    assert.equal(row.profile.test, key === 'test'); assert.equal(row.profile.opt_level, '3');
    assert.equal(row.executable, expected.artifacts[key].path);
    assert(Array.isArray(row.filenames) && row.filenames.filter(name => name === row.executable).length === 1, 'unique actual artifact filename');
  }
  const stderr = text(capture.stderr); assert(!/[\x00-\x08\x0b-\x1f\x7f]/u.test(stderr), 'plain complete verbose capture');
  const calls = stderr.split('\n').map(line => /^\s*Running `(.*)`$/u.exec(line)).filter(Boolean)
    .map(match => cargoWords(match[1])).filter(words => words.includes('lifecycle') && words.includes('--crate-name'));
  assert.equal(calls.length, 1, 'one actual lifecycle compiler invocation'); const words = compilerCall(calls[0], expected);
  assert.equal(words.filter(word => word === '--test').length, 1);
  assert.equal(words.filter(word => word === '--crate-name').length, 1);
  assert.equal(words.filter(word => [directory + '/tests/lifecycle.rs', 'apps/' + PACKAGE + '/tests/lifecycle.rs'].includes(word)).length, 1, 'same lifecycle source invocation');
  assert.equal(words[words.indexOf('--crate-name') + 1], 'lifecycle');
}
