import assert from 'node:assert/strict';
import { chmodSync, constants, copyFileSync, lstatSync, mkdirSync, realpathSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { canonical, MAX_LOG, parse, readSafe, sha256 } from './input.mjs';
import { execute } from './process.mjs';
import { captureReleaseBuild } from './release-build-capture.mjs';
import { outside, pinFile } from './release-build-input.mjs';
import { captureCorrectnessOracles } from './cli-performance-oracle-capture.mjs';
import { parseMethodology } from './cli-performance-methodology-v4.mjs';
import { provisionLinuxTimeProvider } from './cli-performance-linux-provision-provider.mjs';
import { captureLinuxDraft } from './cli-performance-linux-io-v2.mjs';
import { captureWindowsDraft } from './cli-performance-windows-io.mjs';
import { HELPER_SOURCE_SHA256 } from './cli-performance-windows-binding.mjs';
import { captureHostedMetadata } from './cli-performance-host.mjs';
import { runArchitectureCheck, verifySourceAfterCollection } from './cli-performance-hosted-checks.mjs';

export function collectorEnvironment(environment, rustc, windows) {
  const keys = windows ? ['PATH', 'SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'CARGO_HOME',
    'RUSTUP_HOME', 'LIB', 'INCLUDE'] : ['PATH', 'HOME', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME'];
  const result = Object.fromEntries(keys.map(key => { assert(typeof environment[key] === 'string');
    return [key, environment[key]]; }));
  return { ...result, RUSTUP_TOOLCHAIN: '1.97.1', RUSTC: rustc };
}
export function observedPin(pin) { return { path: pin.path, bytes: pin.bytes, sha256: pin.sha256 }; }
export function validateRetainedCliBinding(record) {
  const { sourceBefore, sourceAfter, copy, targetDirectory, measuredPath } = record;
  assert.equal(record.format, 'zryna.cli-performance-retained-release-cli.v2');
  assert(Object.values(record.links).every(value => Number.isSafeInteger(value) && value > 0));
  assert.equal(record.links.builtBefore, record.links.builtAfter, 'built image links stable across copy');
  assert.equal(record.links.copy, 1, 'independent single-link measured CLI copy');
  assert.equal(sourceBefore.path, sourceAfter.path, 'same built image before and after copy');
  assert(!outside(targetDirectory, sourceBefore.path), 'built CLI remains in its original target tree');
  assert.equal(measuredPath, copy.path, 'measure exactly the independently rehashed private copy');
  assert(outside(targetDirectory, copy.path), 'retained CLI must be outside excluded target tree');
  for (const key of ['bytes', 'mode', 'sha256']) {
    assert.equal(sourceAfter[key], sourceBefore[key], 'source CLI stable across copy');
    assert.equal(copy[key], sourceBefore[key], 'independently rehashed retained CLI binding');
  }
}
export function retainReleaseCli(output, build) {
  const sourceBefore = pinFile(build.cli.after.path);
  const builtLinksBefore = lstatSync(sourceBefore.path).nlink;
  const directory = join(output, 'retained-release-cli'); mkdirSync(directory, { mode: 0o700 });
  const destination = join(directory, basename(sourceBefore.path));
  copyFileSync(sourceBefore.path, destination, constants.COPYFILE_EXCL);
  chmodSync(destination, parseInt(sourceBefore.mode, 8));
  const record = { format: 'zryna.cli-performance-retained-release-cli.v2',
    sourceBefore, sourceAfter: pinFile(sourceBefore.path), copy: pinFile(destination),
    links: { builtBefore: builtLinksBefore, builtAfter: lstatSync(sourceBefore.path).nlink,
      copy: lstatSync(destination).nlink },
    targetDirectory: build.environment.CARGO_TARGET_DIR, measuredPath: destination,
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  save(output, 'retained-release-cli.json', record); // Preserve observed mismatch before validation.
  assert.deepEqual(sourceBefore, build.cli.after, 'same actual successful release CLI');
  validateRetainedCliBinding(record);
  return record;
}
export function windowsAdmission(capture) {
  return { format: 'zryna.cli-performance-windows-operator-binding.v1', tools: capture.tools,
    helperBuiltFromSha256: HELPER_SOURCE_SHA256, sourceCommit: capture.sourceCommit,
    sourceTree: capture.sourceTree, catalogueSha256: capture.catalogueSha256,
    artifactOracleSha256: capture.artifactOracleSha256, reviewRecordSha256: capture.reviewRecordSha256 };
}
export function baselineRound(round) {
  assert(['baseline-round-1', 'baseline-round-2'].includes(round), 'only two unqualified baseline observation rounds');
  return round;
}
function save(output, name, value) {
  const bytes = Buffer.from(canonical(value));
  const path = join(output, name); writeFileSync(path, bytes, { flag: 'wx', mode: 0o600 });
  return { path, bytes: bytes.length, sha256: sha256(bytes) };
}
function readCalibration(directory, tools) {
  const result = JSON.parse(readSafe(directory, 'hosted-result.json').toString('utf8'));
  const controls = JSON.parse(readSafe(directory, 'controls/calibration.json').toString('utf8'));
  assert.equal(result.status, 'eight-controls-passed'); assert.equal(result.controls, 8);
  assert.equal(controls.status, 'controls-passed'); assert.equal(controls.results.length, 8);
  assert(controls.results.every(row => row.status === 'passed'));
  const built = JSON.parse(readSafe(directory, 'built-tools.json').toString('utf8'));
  const toolchain = JSON.parse(readSafe(directory, 'toolchain-before-build.json').toString('utf8'));
  for (const [key, observed] of [['cc', toolchain.compiler], ['linker', toolchain.linker], ['node', toolchain.node]])
    assert.deepEqual(observedPin(pinFile(tools[key])), observed, 'use actual calibrated toolchain');
  assert.deepEqual(observedPin(pinFile(built.helper.path)), built.helper, 'same actual helper-build executable');
  return { helper: built.helper, bindings: { hosted: observedPin(pinFile(join(directory, 'hosted-result.json'))),
    controls: observedPin(pinFile(join(directory, 'controls', 'calibration.json'))),
    toolchain: observedPin(pinFile(join(directory, 'toolchain-before-build.json'))) } };
}
export async function runHostedRound(input, output) {
  output = resolve(output); mkdirSync(output, { mode: 0o700 });
  const root = realpathSync(input.root); const windows = process.platform === 'win32';
  const host = windows ? 'windows-x86_64' : 'linux-x86_64';
  const target = windows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu';
  let stage = 'operator-input';
  let build;
  let sourcePostflightAttempted = false;
  save(output, 'operator-input.json', input);
  try {
    baselineRound(input.round);
    const document = parseMethodology(readSafe(root, 'tests/cli-performance-methodology-v4.json'));
    const review = readSafe(root, 'scripts/stability-gates/cli-performance-operator-review.json');
    const boundary = JSON.parse(review.toString('utf8'));
    assert.equal(boundary.sourceAuthenticated, false); assert.equal(boundary.performanceQualified, false);
    assert.equal(boundary.baselineAuthority, null); assert.equal(boundary.thresholds, null);
    assert.equal(boundary.hostWallGuardMs, 15000);
    writeFileSync(join(output, 'operator-review.raw.json'), review, { flag: 'wx', mode: 0o600 });
    save(output, 'operator-materials.json', { workflow: observedPin(pinFile(join(root, '.github/workflows/cli-performance.yml'))),
      review: observedPin(pinFile(join(root, 'scripts/stability-gates/cli-performance-operator-review.json'))),
      triggeringWorkflowObservation: input.environment.GITHUB_ACTIONS === 'true' ? 'declared-by-workflow-run' : 'unavailable',
      sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null });
    const reviewRecordSha256 = sha256(review);
    const env = input.environment;
    for (const [name, args, expected] of [['head', ['rev-parse', 'HEAD'], input.expectedHead],
      ['tree', ['rev-parse', 'HEAD^{tree}'], input.expectedTree],
      ['clean', ['status', '--porcelain=v1', '--untracked-files=all'], '']]) {
      const result = await execute(input.tools.git, args, { root, env, timeoutMs: 30000 });
      for (const stream of ['stdout', 'stderr']) writeFileSync(join(output, `checkout-${name}.${stream}.raw`),
        result[stream], { flag: 'wx', mode: 0o600 });
      save(output, `checkout-${name}.json`, { executable: input.tools.git, args,
        exitCode: result.exitCode, error: result.error, signal: result.signal,
        stdoutSha256: sha256(result.stdout), stderrSha256: sha256(result.stderr) });
      assert.equal(result.error, null); assert.equal(result.signal, null); assert.equal(result.exitCode, 0);
      assert.equal(name === 'clean' ? result.stdout.toString('utf8') : result.stdout.toString('utf8').trim(), expected);
    }
    await captureHostedMetadata({ root, output, phase: 'before', environment: env });
    stage = 'release-build';
    build = await captureReleaseBuild({ root, output: join(output, 'release-build'), target,
      tools: input.tools, environment: env });
    assert.equal(build.profile.observed, 'release');
    assert.equal(build.source.before.commit, input.expectedHead); assert.equal(build.source.before.tree, input.expectedTree);
    stage = 'retain-release-cli';
    const retainedCli = retainReleaseCli(output, build); // The retained private copy is the actual measured image.
    const cli = observedPin(retainedCli.copy); const node = observedPin(build.tools.after.node);
    const environment = collectorEnvironment(env, input.tools.rustc, windows);
    stage = 'architecture-check';
    await runArchitectureCheck({ root, output, cli: cli.path, environment: build.environment });
    stage = 'provider';
    const provider = windows ? readCalibration(input.calibrationDirectory, input.tools)
      : await provisionLinuxTimeProvider({ checkoutRoot: root });
    save(output, 'provider-binding.json', provider);
    stage = 'untimed-paired-correctness-oracles';
    const oracle = await captureCorrectnessOracles({ root, output: join(output, 'oracles'), document,
      tools: windows ? { cli: cli.path, node: node.path, helper: provider.helper.path }
        : { cli: cli.path, node: node.path }, environment, host });
    assert.equal(oracle.status, 'unmeasured-correctness-and-paired-replay-passed');
    const artifactOracleSha256 = sha256(canonical(oracle.oracles));
    assert.equal(artifactOracleSha256, oracle.oracleFile.sha256);
    const capture = { format: windows ? 'zryna.cli-performance-windows-capture.draft.v1'
      : 'zryna.cli-performance-linux-capture.draft.v2', catalogueSha256: sha256(canonical(document)),
      round: input.round, cliBuildClass: 'release', sourceCommit: build.source.before.commit,
      sourceTree: build.source.before.tree, host: { platform: host, observation: 'host-before.json' },
      tools: windows ? { helper: provider.helper, cli, node } : { time: provider.executable, cli, node },
      environment, hostWallGuardMs: 15000, cleanupGuardMs: 10000, combinedOutputBytes: MAX_LOG,
      reviewRecordSha256, artifactOracleSha256, sourceAuthenticated: false, performanceQualified: false,
      baselineAuthority: null, thresholds: null, productionReleaseAdmission: false };
    if (windows) capture.operatorAdmission = save(output, 'windows-operator-binding.json', windowsAdmission(capture));
    else Object.assign(capture, { providerRoot: provider.providerRoot, providerRecordSha256: provider.providerRecordSha256,
      providerReceiptSha256: provider.providerReceiptSha256 });
    stage = 'one-collector-round';
    const session = await (windows ? captureWindowsDraft : captureLinuxDraft)({ root,
      output: join(output, 'round'), document, capture, oracles: oracle.oracles });
    stage = 'source-postflight';
    sourcePostflightAttempted = true;
    verifySourceAfterCollection({ root, output, tools: input.tools, environment: build.environment,
      before: build.source.after });
    await captureHostedMetadata({ root, output, phase: 'after', environment: env });
    save(output, 'hosted-result.json', { status: 'collected-unqualified-observations', round: input.round,
      cells: session.cells.length, sourceAuthenticated: false, performanceQualified: false,
      baselineAuthority: null, thresholds: null, closureStatus: 'blocked' });
  } catch (error) {
    if (build && !sourcePostflightAttempted) {
      sourcePostflightAttempted = true;
      try {
        verifySourceAfterCollection({ root, output, tools: input.tools, environment: build.environment,
          before: build.source.after });
      } catch (postflight) {
        save(output, 'source-postflight-failure.json', { message: String(postflight.stack ?? postflight) });
      }
    }
    save(output, 'hosted-failure.json', { stage, message: String(error.stack ?? error),
      sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null });
    throw error;
  }
}
