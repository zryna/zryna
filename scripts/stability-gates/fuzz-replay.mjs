import assert from 'node:assert/strict';
import { resolve, posix, win32 } from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { exact, MAX_LOG, parse, readSafe, sha256 } from './input.mjs';
import { counts } from './proof.mjs';
import { hostName } from './registry.mjs';
import { loadFuzzRegistry, REGISTRY_SHA256, ROOT } from './fuzz-registry.mjs';
import { retained, successful } from './fuzz-files.mjs';
import { replayWitness } from './fuzz-witness.mjs';

const utf8 = bytes => new TextDecoder('utf-8', { fatal: true }).decode(bytes);

export function namedProof(stdout, name) {
  assert.match(name, /^[a-z0-9_]+$/);
  const matches = [...stdout.matchAll(new RegExp(`^test (?:[A-Za-z0-9_]+::)*${name} \\.\\.\\. ok\\r?$`, 'gm'))];
  assert.equal(matches.length, 1, `exact named proof ${name}`);
}

export function rustProof(bytes, gate) {
  const stdout = utf8(bytes);
  assert(counts(stdout, 'rust-tests').complete, 'complete Rust proof');
  if (gate.test) {
    namedProof(stdout, gate.test);
    const summaries = [...stdout.matchAll(/^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/gm)];
    assert.equal(summaries.length, 1);
    assert.deepEqual(summaries[0].slice(1, 5), ['1', '0', '0', '0']);
  }
}

export function checkObservation(output, record, gate) {
  exact(record, ['id', 'executable', 'args', 'attempt', 'stdout', 'stderr', 'witness', 'totals'], 'observation');
  assert.equal(record.id, gate.id);
  assert.equal(record.executable, gate.executable);
  assert.deepEqual(record.args, gate.args);
  successful(record.attempt, gate.timeoutMs);
  const stdout = retained(output, record.stdout, `${gate.id}.stdout`);
  const stderr = retained(output, record.stderr, `${gate.id}.stderr`);
  assert(stdout.length + stderr.length <= MAX_LOG, 'combined output cap');
  rustProof(stdout, gate);
  if (gate.test) {
    const witness = retained(output, record.witness, `${gate.id}.jsonl`);
    const totals = replayWitness(witness, gate);
    assert.deepEqual(record.totals, totals);
  } else {
    assert.equal(record.witness, null);
    assert.equal(record.totals, null);
  }
  return stdout;
}

function checkSource(source, expectedCommit) {
  exact(source, ['repository', 'commit', 'tree', 'inventorySha256', 'files', 'bytes'], 'source');
  assert.equal(source.repository, 'https://github.com/zryna/zryna');
  assert.match(expectedCommit, /^[a-f0-9]{40}$/);
  assert.equal(source.commit, expectedCommit);
  assert.match(source.tree, /^[a-f0-9]{40}$/);
  assert.match(source.inventorySha256, /^[a-f0-9]{64}$/);
  assert(Number.isSafeInteger(source.files) && source.files > 0 && source.files <= 50000);
  assert(Number.isSafeInteger(source.bytes) && source.bytes > 0 && source.bytes <= 64 * 1024 * 1024);
}

function checkTools(output, records, toolchains, runtime) {
  assert.equal(records.length, Object.keys(toolchains).length);
  for (const [index, name] of Object.keys(toolchains).sort().entries()) {
    const record = records[index];
    exact(record, ['name', 'executable', 'args', 'version', 'attempt', 'stdout', 'stderr'], 'observed tool');
    assert.equal(record.name, name);
    if (name === 'pnpm') {
      const paths = runtime.platform === 'win32' ? win32 : posix;
      assert.equal(record.executable, runtime.nodePath, 'same explicit Node executes pnpm JS');
      assert.equal(record.args.length, 2);
      assert.equal(record.args[1], '--version');
      assert(paths.isAbsolute(record.args[0]), 'absolute direct pnpm entry');
      assert.equal(paths.basename(record.args[0]), 'pnpm.cjs');
    } else {
      assert.equal(record.executable, name);
      assert.deepEqual(record.args, ['--version']);
    }
    successful(record.attempt, 30000);
    const stdout = utf8(retained(output, record.stdout, `tool-${name}.stdout`, 65536)).trim();
    retained(output, record.stderr, `tool-${name}.stderr`, 65536);
    const version = ['rustc', 'cargo'].includes(name) ? stdout.split(' ')[1] : stdout;
    assert.equal(version, toolchains[name]);
    assert.equal(record.version, version);
  }
}

export function replayFuzz(output, expectedCommit, { root = ROOT, host = hostName() } = {}) {
  const { registry, bytes } = loadFuzzRegistry(root);
  assert(!existsSync(resolve(output, 'failure.json')), 'failed collection is not a passing campaign');
  assert.deepEqual(parse(readSafe(output, 'started.json')), { expectedCommit, attempt: 1 });
  assert.deepEqual(readSafe(output, 'registry.json'), bytes);
  const receipt = parse(readSafe(output, 'receipt.json'));
  exact(receipt, ['format', 'registrySha256', 'source', 'host', 'runtime', 'tools', 'results',
    'closureStatus', 'globalM7'], 'campaign receipt');
  assert.equal(receipt.format, 'zryna.m7.fuzz-campaign.v2');
  assert.equal(receipt.registrySha256, REGISTRY_SHA256);
  assert.equal(sha256(bytes), REGISTRY_SHA256);
  assert.equal(receipt.host, host);
  assert(registry.hosts.includes(host));
  exact(receipt.runtime, ['nodePath', 'nodeVersion', 'platform', 'architecture'], 'runtime');
  assert.equal(receipt.runtime.nodeVersion, registry.toolchains.node);
  assert.equal(typeof receipt.runtime.nodePath, 'string');
  assert(receipt.runtime.nodePath.length > 0);
  assert.equal(receipt.runtime.platform, host === 'linux-x86_64' ? 'linux' : 'win32');
  assert.equal(receipt.runtime.architecture, 'x64');
  assert.equal(receipt.closureStatus, 'blocked');
  assert.equal(receipt.globalM7, false);
  checkSource(receipt.source, expectedCommit);
  assert.deepEqual(parse(readSafe(output, 'source.json')), receipt.source);
  assert.deepEqual(parse(readSafe(output, 'source-final.json')), receipt.source);
  checkTools(output, receipt.tools, registry.toolchains, receipt.runtime);
  const gates = [...registry.campaigns, registry.fullOwner];
  assert.equal(receipt.results.length, gates.length);
  for (const [index, gate] of gates.entries()) {
    const record = receipt.results[index];
    assert.deepEqual(parse(readSafe(output, `result-${gate.id}.json`)), record);
    const stdout = checkObservation(output, record, gate);
    if (!gate.test) for (const name of registry.namedBoundaries) namedProof(utf8(stdout), name);
  }
  return receipt;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assert.equal(process.argv.length, 4, 'usage: fuzz-replay.mjs /absolute/output full-commit');
    replayFuzz(resolve(process.argv[2]), process.argv[3]);
    console.log('Bounded campaign originals replayed; M7 closure remains blocked.');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
