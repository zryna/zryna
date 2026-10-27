// Retained untimed correctness/architecture consistency; never executes tools or artifacts.
import assert from 'node:assert/strict';
import { posix, win32 } from 'node:path';
import { canonical, exact, MAX_DOCUMENT, MAX_LOG, parse, portable, sha256 } from './input.mjs';
import { checkKnownGraphs, checkSelectedBytes, matrixFor } from './cli-performance-methodology-v4.mjs';
import { MAX_ARTIFACTS } from './cli-performance-oracle-files.mjs';
import { validateOracleObservation } from './cli-performance-oracle-capture.mjs';
import { validateRetainedManifest } from './cli-performance-oracle-manifest.mjs';
import { validateWindowsOracleResult } from './cli-performance-oracle-windows.mjs';
import { parseWindowsHelperMetadata } from './cli-performance-windows-metrics.mjs';
const FLAGS = { sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
const projected = pin => ({ path: pin.path, bytes: pin.bytes, sha256: pin.sha256 });

export function replayOracleArchitecture({ document, host, operator, capture, read, readOriginal }) {
  const windows = host === 'windows-x86_64', path = windows ? win32 : posix;
  document = structuredClone(document); operator = structuredClone(operator); capture = structuredClone(capture);
  const rows = matrixFor(document, host);
  const load = (name, maximum) => {
    portable(name); const bytes = read(name, maximum);
    assert(Buffer.isBuffer(bytes) && bytes.length <= maximum, 'bounded retained oracle bytes');
    const copy = Buffer.alloc(bytes.length); bytes.copy(copy); return copy;
  };
  const json = name => parse(load(name, MAX_DOCUMENT));
  const same = (name, value) => assert(load(name, MAX_DOCUMENT).equals(Buffer.from(canonical(value))), `retained binding ${name}`);
  same('operator-input.json', operator); same('round/capture.json', capture); same('oracles/catalogue.json', document);
  assert.equal(operator.expectedHead, capture.sourceCommit); assert.equal(operator.expectedTree, capture.sourceTree);
  assert.equal(operator.round, capture.round); assert.equal(capture.host.platform, host);
  for (const [key, value] of Object.entries(FLAGS)) assert.equal(capture[key], value);
  const absolute = value => {
    assert(typeof value === 'string' && value.length < 4096 && path.isAbsolute(value) && path.normalize(value) === value);
    assert(!/[\x00-\x1f]/u.test(value));
    if (windows) assert(/^[A-Za-z]:\\/u.test(value) && !value.slice(2).includes(':'));
    return value;
  };
  absolute(operator.root);
  const cli = absolute(capture.tools.cli.path), output = path.dirname(path.dirname(cli));
  assert.equal(path.basename(path.dirname(cli)), 'retained-release-cli');
  const oracleDirectory = path.join(output, 'oracles');
  const pins = new Map(document.materials.map(pin => [pin.path, pin])), aliases = new Map(), originals = new Map();
  for (const family of document.families) {
    for (const pin of [...family.sources, family.oracle.material]) pins.set(pin.path, pin);
    for (const copy of family.staging?.copies ?? []) aliases.set(copy.to, copy.from);
  }
  const original = (logical, maximum) => {
    portable(logical); const source = aliases.get(logical) ?? logical, pin = pins.get(source);
    assert(pin && maximum >= pin.bytes, 'only frozen original/staged source mappings');
    if (!originals.has(source)) {
      const bytes = readOriginal(source, pin.bytes);
      assert(Buffer.isBuffer(bytes) && bytes.length === pin.bytes && sha256(bytes) === pin.sha256, `frozen original ${source}`);
      const copy = Buffer.alloc(bytes.length); bytes.copy(copy); originals.set(source, copy);
    }
    return originals.get(source);
  };
  checkSelectedBytes(document, original); checkKnownGraphs(document, original);
  for (const [alias, source] of aliases) assert(original(alias, pins.get(source).bytes).equals(original(source, pins.get(source).bytes)));
  const input = json('oracles/input.json'), result = json('oracles/result.json');
  const toolKeys = windows ? ['cli', 'node', 'helper'] : ['cli', 'node']; exact(input.tools, toolKeys, 'oracle tools');
  for (const key of toolKeys) {
    const pin = input.tools[key]; exact(pin, ['path', 'bytes', 'sha256', 'mode', 'prefixHex'], 'oracle tool pin'); absolute(pin.path);
    assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= 256 * 1024 * 1024);
    assert(/^[a-f0-9]{64}$/u.test(pin.sha256) && /^[0-7]{1,3}$/u.test(pin.mode));
    assert(typeof pin.prefixHex === 'string' && /^[a-f0-9]+$/u.test(pin.prefixHex) && pin.prefixHex.length === 2 * Math.min(pin.bytes, 128));
    assert.deepEqual(projected(pin), capture.tools[key]);
  }
  assert.deepEqual(input, { host, tools: input.tools, cells: rows.length, commands: rows.length * 2,
    executionGuardMs: 60000, combinedOutputBytes: MAX_LOG, artifactTreeBytes: MAX_ARTIFACTS });
  exact(result, ['status', 'host', 'cells', 'commands', 'tools', ...Object.keys(FLAGS), 'closureStatus', 'oracleFile'], 'complete oracle result');
  assert.deepEqual({ ...result, oracleFile: undefined }, { status: 'unmeasured-correctness-and-paired-replay-passed',
    host, cells: rows.length, commands: rows.length * 2, tools: input.tools, ...FLAGS, closureStatus: 'blocked', oracleFile: undefined });
  const oracleBytes = load('oracles/artifact-oracles.json', MAX_DOCUMENT), oracles = parse(oracleBytes);
  assert(load('round/artifact-oracles.json', MAX_DOCUMENT).equals(oracleBytes), 'identical paired oracle copies');
  assert.deepEqual(result.oracleFile, { path: path.join(oracleDirectory, 'artifact-oracles.json'), bytes: oracleBytes.length, sha256: sha256(oracleBytes) });
  assert.equal(capture.artifactOracleSha256, sha256(oracleBytes));
  const reconciled = [];
  for (const [index, row] of rows.entries()) {
    const pair = [];
    for (let replay = 0; replay < 2; ++replay) {
      const id = `${String(index).padStart(2, '0')}-${replay}`, prefix = `oracles/${id}`;
      const process = json(`${prefix}.process.json`);
      const stdout = load(`${prefix}.stdout`, MAX_DOCUMENT), stderr = load(`${prefix}.stderr`, MAX_LOG);
      const call = { file: cli, args: row.argv.map(value => value === '$SOURCE_ROOT' ? operator.root :
        value === '$NODE' ? capture.tools.node.path : value), timeoutMs: 60000 };
      const outerCall = windows ? { file: capture.tools.helper.path, args: ['--timeout-ms', '60000', '--output',
        path.join(oracleDirectory, `${id}.sample`), '--', call.file, ...call.args], timeoutMs: 71000 } : null;
      assert(Number.isFinite(process.elapsedMs) && process.elapsedMs >= 0);
      assert.deepEqual(process, { call, outerCall, exitCode: 0, signal: null, error: null, elapsedMs: process.elapsedMs,
        stdoutBytes: stdout.length, stderrBytes: stderr.length, stdoutTruncated: false, stderrTruncated: false,
        helperStdoutBytes: windows ? 0 : null, helperStderrBytes: windows ? 0 : null, readError: null });
      assert.deepEqual(json(`${prefix}.after.json`), { status: 'passed', error: null });
      if (windows) {
        const helperStdout = load(`${prefix}.helper.stdout`, MAX_LOG), helperStderr = load(`${prefix}.helper.stderr`, MAX_LOG);
        const metadata = load(`${prefix}.metadata.raw`, 4096);
        for (const [name, bytes] of [['result.json', metadata], ['stdout.raw', stdout], ['stderr.raw', stderr]])
          assert(load(`${prefix}.sample/${name}`, name === 'result.json' ? 4096 : MAX_LOG).equals(bytes), 'original helper sample equals flattened copy');
        validateWindowsOracleResult({ stdout, stderr, readError: process.readError,
          outer: { error: process.error, signal: process.signal, exitCode: process.exitCode, stdout: helperStdout, stderr: helperStderr } },
        call, () => parseWindowsHelperMetadata(metadata));
      }
      const files = json(`${prefix}.inventory.json`);
      const observed = validateOracleObservation(document, row, host, stdout, stderr, files);
      let manifest;
      for (const pin of files) {
        const bytes = load(`oracles/artifacts/${id}/${pin.path}`, pin.path === row.expectedResponse.manifest ? MAX_DOCUMENT : MAX_ARTIFACTS);
        assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256, 'complete retained artifact bytes');
        if (pin.path === row.expectedResponse.manifest) manifest = bytes;
      }
      validateRetainedManifest({ document, row, host, files, manifestBytes: manifest, readOriginal: original }); pair.push(observed);
    }
    assert.deepEqual(pair[0], pair[1], 'both same-path untimed attempts agree'); reconciled.push(pair[0]);
  }
  assert.deepEqual(oracles, reconciled, 'all ordered paired oracles match original attempts');
  const command = { executable: cli, args: ['architecture', 'check'], cwd: operator.root, timeoutMs: 60000 };
  assert.deepEqual(json('architecture.command.json'), command);
  const architecture = json('architecture.result.json');
  assert(Number.isFinite(architecture.elapsedMs) && architecture.elapsedMs >= 0);
  assert.deepEqual(architecture, { command, exitCode: 0, signal: null, error: null, elapsedMs: architecture.elapsedMs, logs: architecture.logs });
  exact(architecture.logs, ['stdout', 'stderr'], 'architecture raw streams'); let total = 0;
  for (const stream of ['stdout', 'stderr']) {
    const bytes = load(`architecture.${stream}.raw`, MAX_LOG); total += bytes.length;
    assert.deepEqual(architecture.logs[stream], { bytes: bytes.length, sha256: sha256(bytes) });
  }
  assert(total <= MAX_LOG);
  return { format: 'zryna.oracle-architecture-reconciliation.v1', host, cells: rows.length, untimedAttempts: rows.length * 2,
    originalMaterials: originals.size, stagingAliases: aliases.size, artifactOracleSha256: sha256(oracleBytes),
    rawOracleArchitectureReconciled: true, ...FLAGS, hostAuthenticated: false,
    scope: 'Retained attempts/artifacts/material and architecture claims agree; temporal staging/material checks and actual source/tool execution are not independently proven' };
}
