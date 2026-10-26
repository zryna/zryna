// Unmeasured paired correctness only. No source, release, baseline, or threshold authority.
import assert from 'node:assert/strict';
import { mkdirSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, join, posix, resolve } from 'node:path';
import { TextDecoder } from 'node:util';
import { canonical, exact, MAX_DOCUMENT, MAX_LOG, portable, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { execute } from './process.mjs';
import { outside, pinFile } from './release-build-input.mjs';
import { MAX_ARTIFACTS, oracleFiles, writePrivate } from './cli-performance-oracle-files.mjs';
import { windowsOracleRunner } from './cli-performance-oracle-windows.mjs';
function validateOptions({ document, host, tools, environment }) {
  const rows = matrixFor(document, host);
  exact(tools, host === 'windows-x86_64' ? ['cli', 'node', 'helper'] : ['cli', 'node'], 'oracle tools');
  for (const path of Object.values(tools)) assert(typeof path === 'string' && isAbsolute(path) && path === resolve(path));
  assert(environment && typeof environment === 'object' && !Array.isArray(environment));
  assert(Object.keys(environment).length <= 64);
  let total = 0;
  for (const [key, value] of Object.entries(environment)) {
    assert(/^[A-Za-z_][A-Za-z0-9_]{0,127}$/u.test(key) && typeof value === 'string' &&
      Buffer.byteLength(value) <= 8192 && !value.includes('\0'), 'bounded explicit trusted environment');
    total += Buffer.byteLength(key) + Buffer.byteLength(value);
  }
  assert(total <= 65536, 'explicit environment byte budget');
  return rows;
}
export function validateOracleObservation(document, row, host, stdout, stderr, files) {
  assert(Buffer.isBuffer(stdout) && Buffer.isBuffer(stderr));
  assert(stdout.length <= MAX_DOCUMENT && stdout.length + stderr.length <= MAX_LOG);
  assert.equal(stderr.length, 0, 'zero public stderr');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(stdout);
  const response = JSON.parse(text); canonical(response);
  assert.deepEqual(response, row.expectedResponse, 'original frozen typed public response');
  assert(Array.isArray(files) && files.length > 0 && files.length <= 64);
  let bytes = 0;
  const prefix = `${posix.dirname(row.expectedResponse.manifest)}/`;
  const paths = files.map(pin => {
    exact(pin, ['path', 'bytes', 'sha256'], 'oracle artifact pin'); portable(pin.path);
    assert(pin.path.startsWith(prefix));
    assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= MAX_ARTIFACTS);
    assert.match(pin.sha256, /^[a-f0-9]{64}$/u); bytes += pin.bytes; return pin.path;
  });
  assert(bytes <= MAX_ARTIFACTS, 'owned artifact byte budget');
  assert.deepEqual(paths, [...new Set(paths)].sort(), 'complete sorted unique artifact inventory');
  assert(paths.includes(row.expectedResponse.manifest), 'public manifest is retained');
  const family = document.families.find(f => row.id.startsWith(`${f.id}/`));
  if (row.phase === 'build' && family.existingBuildGoldens) {
    const golden = family.existingBuildGoldens.find(g => g.target === row.target);
    assert(golden, 'unchanged M2 target golden');
    assert(files.some(pin => pin.bytes === golden.bytes && pin.sha256 === golden.sha256), 'unchanged existing M2 build golden');
  }
  return { selector: row.id, manifest: row.expectedResponse.manifest, stdout: text, stderr: '', files };
}
const errorText = error => String(error.message ?? error).slice(0, 4096);
const bytes = value => Buffer.isBuffer(value) ? value : Buffer.alloc(0);
async function afterCheck(io, staged, name) {
  let error = null;
  try { await io.checkMaterials(staged); } catch (cause) { error = cause; }
  await io.retain(name, Buffer.from(canonical({ status: error ? 'failed' : 'passed', error: error ? errorText(error) : null })));
  return error;
}
async function retainAttempt(io, id, call, result) {
  const outer = result.outer;
  await io.retain(`${id}.stdout`, bytes(result.stdout).subarray(0, MAX_LOG));
  await io.retain(`${id}.stderr`, bytes(result.stderr).subarray(0, MAX_LOG));
  if (result.metadata !== undefined) await io.retain(`${id}.metadata.raw`, bytes(result.metadata).subarray(0, MAX_DOCUMENT));
  if (result.outerCall) {
    await io.retain(`${id}.helper.stdout`, bytes(outer.stdout).subarray(0, MAX_LOG));
    await io.retain(`${id}.helper.stderr`, bytes(outer.stderr).subarray(0, MAX_LOG));
  }
  const count = value => Buffer.isBuffer(value) ? value.length : null;
  await io.retain(`${id}.process.json`, Buffer.from(canonical({ call, outerCall: result.outerCall ?? null,
    exitCode: outer.exitCode ?? null, signal: outer.signal ?? null, error: outer.error ?? null,
    elapsedMs: outer.elapsedMs ?? null, stdoutBytes: count(result.stdout), stderrBytes: count(result.stderr),
    stdoutTruncated: count(result.stdout) > MAX_LOG, stderrTruncated: count(result.stderr) > MAX_LOG,
    helperStdoutBytes: result.outerCall ? count(outer.stdout) : null,
    helperStderrBytes: result.outerCall ? count(outer.stderr) : null, readError: result.readError ?? null })));
}
// Finite fake-IO seam. The real API below admits native paths and supplies every operation itself.
export async function collectCorrectnessOracles(options, io) {
  const { root, output, document, tools, host } = options;
  const rows = validateOptions(options), oracles = [];
  const receipt = { status: 'invalid-or-incomplete', host, cells: 0, commands: 0, tools: io.toolPins,
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null, closureStatus: 'blocked' };
  let staged = false;
  await io.retain('catalogue.json', Buffer.from(canonical(document)));
  await io.retain('input.json', Buffer.from(canonical({ host, tools: io.toolPins, cells: rows.length,
    commands: rows.length * 2, executionGuardMs: 60000, combinedOutputBytes: MAX_LOG, artifactTreeBytes: MAX_ARTIFACTS })));
  try {
    await io.checkMaterials(false); await io.stage(); staged = true; await io.checkMaterials(true);
    for (const [index, row] of rows.entries()) {
      let first;
      for (let replay = 0; replay < 2; ++replay) {
        const id = `${String(index).padStart(2, '0')}-${replay}`;
        receipt.currentAttempt = { id, selector: row.id, replay,
          ownedBundle: posix.dirname(row.expectedResponse.manifest) };
        await io.checkMaterials(true); await io.beforeAttempt(row);
        const call = { file: tools.cli, args: row.argv.map(value => value === '$SOURCE_ROOT' ? root :
          value === '$NODE' ? tools.node : value), timeoutMs: 60000 };
        let result;
        try { result = await io.run(call, id); }
        catch (error) { result = { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0),
          outer: { exitCode: null, signal: null, error: errorText(error), stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) } }; }
        receipt.commands++;
        let afterError;
        try { await retainAttempt(io, id, call, result); }
        finally { afterError = await afterCheck(io, true, `${id}.after.json`); }
        if (afterError) throw afterError;
        assert.equal(result.outer.error, null, 'no timeout/spawn/output/unknown cleanup failure');
        assert.equal(result.outer.signal, null); assert.equal(result.outer.exitCode, 0);
        assert(Buffer.isBuffer(result.stdout) && Buffer.isBuffer(result.stderr));
        assert(result.stdout.length + result.stderr.length <= MAX_LOG, 'combined public stream budget');
        assert(Buffer.isBuffer(result.outer.stdout) && Buffer.isBuffer(result.outer.stderr));
        assert(result.outer.stdout.length + result.outer.stderr.length <= MAX_LOG, 'combined outer stream budget');
        await io.validateResult(result, call);
        const files = await io.snapshotArtifacts(row, id); // both complete copies precede replay equality
        await io.retain(`${id}.inventory.json`, Buffer.from(canonical(files)));
        const current = validateOracleObservation(document, row, host, result.stdout, result.stderr, files);
        if (replay === 0) first = current;
        else assert.deepEqual(current, first, 'same-path paired public bytes and artifact inventory');
        await io.checkMaterials(true); await io.checkArtifacts(row, id, files);
        await io.cleanup(row, id, files); // only the verified owned bundle, outside any timing protocol
        delete receipt.currentAttempt;
      }
      oracles.push(first); receipt.cells++;
    }
    await io.checkMaterials(true); await io.finishStage(); staged = false; await io.checkMaterials(false);
    const oracleBytes = Buffer.from(canonical(oracles));
    await io.retain('artifact-oracles.json', oracleBytes);
    receipt.oracleFile = { path: join(output, 'artifact-oracles.json'), bytes: oracleBytes.length, sha256: sha256(oracleBytes) };
    receipt.status = 'unmeasured-correctness-and-paired-replay-passed';
    await io.retain('result.json', Buffer.from(canonical(receipt)));
    return { ...receipt, oracles };
  } catch (error) {
    receipt.failure = errorText(error);
    receipt.stagePath = document.families[2].staging.directory;
    await afterCheck(io, staged, 'failure-after.json');
    await io.retain('INVALID.json', Buffer.from(canonical(receipt)));
    throw error;
  }
}
export async function captureCorrectnessOracles(options) {
  validateOptions(options);
  const { root, output, document, tools, environment, host } = options;
  assert.equal(process.arch, 'x64'); assert.equal(process.platform, host === 'windows-x86_64' ? 'win32' : 'linux');
  assert(isAbsolute(root) && root === resolve(root) && root === realpathSync(root));
  assert(isAbsolute(output) && output === resolve(output));
  assert.equal(realpathSync(dirname(output)), dirname(output), 'real private output parent');
  assert(outside(root, dirname(output)), 'fresh oracle output outside checkout');
  const toolPins = Object.fromEntries(Object.entries(tools).map(([key, path]) => [key, pinFile(path, 256 * 1024 * 1024)]));
  assert.equal(realpathSync(tools.node), realpathSync(process.execPath), 'bound Node runs capture');
  mkdirSync(output, { mode: 0o700 }); mkdirSync(join(output, 'artifacts'), { mode: 0o700 });
  const native = host === 'windows-x86_64' ? windowsOracleRunner({ output, tools, root, environment }) : {
    run: async call => {
      const outer = await execute(call.file, call.args, { root, env: environment, timeoutMs: 60000 });
      return { stdout: outer.stdout, stderr: outer.stderr, outer };
    }, validateResult: () => {},
  };
  return collectCorrectnessOracles(options, { toolPins,
    ...oracleFiles({ root, output, document, tools, toolPins }), ...native,
    retain: (name, content) => { portable(name); assert(!name.includes('/') && Buffer.isBuffer(content) && content.length <= MAX_LOG);
      writePrivate(join(output, name), content); },
  });
}
