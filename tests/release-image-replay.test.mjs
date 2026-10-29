// Synthetic retained records only: no compiler, archived executable or platform authentication.
import assert from 'node:assert/strict';
import test from 'node:test';
import { releaseImageFixture as fixture } from './release-image-replay-fixture.mjs';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { replayNumericReleaseImage, replayReleaseImage } from '../scripts/stability-gates/release-image-replay.mjs';
function rejects(change, windows = false, afterSync = false) {
  const f = fixture(windows); change(f); if (!afterSync) f.sync(); assert.throws(() => replayReleaseImage(f));
}
test('synthetic Linux and Windows evidence replays on the reader host without executing anything', () => {
  for (const windows of [false, true]) {
    const f = fixture(windows), result = replayReleaseImage(f);
    assert.equal(result.rawBuildImageReconciled, true); assert.equal(result.tools.length, 6);
    assert.equal(result.image.sha256, f.retained.copy.sha256);
    for (const key of ['sourceAuthenticated', 'hostAuthenticated', 'performanceQualified', 'productionReleaseAdmission']) assert.equal(result[key], false);
    assert.equal(result.baselineAuthority, null); assert.equal(result.thresholds, null);
  }
});
test('release command/profile/environment labels cannot mask debug or substituted builds', () => {
  for (const change of [f => f.commands['release-build'].args.splice(3, 1), f => f.commands['release-build'].args.push('--config=profile.release.opt-level=0'),
    f => f.commands['release-build'].timeoutMs++, f => f.commands['release-build'].shell = true,
    f => f.build.profile.observed = 'debug', f => f.operator.environment.RUSTFLAGS = '--cfg skipped',
    f => f.build.environment.CARGO_BUILD_JOBS = '3', f => f.build.environment.CARGO_INCREMENTAL = '1',
    f => f.streams['release-build'].stderr = Buffer.from('Finished `dev` profile [unoptimized]\n'),
    f => f.streams['release-build'].stderr = Buffer.from('Finished `release` profile [optimized]\nFinished `release` profile [unoptimized]\n')]) rejects(change);
});
test('original command/result/log and observed/final byte records all remain required', () => {
  for (const name of ['release-build/commands/release-build.command.json', 'release-build/commands/release-build.stdout.raw', 'release-build/observed.json'])
    rejects(f => f.files.delete(name), false, true);
  rejects(f => f.files.set('release-build/observed.json', f.files.get('release-build/capture.json')), false, true);
  rejects(f => f.files.set('release-build/commands/release-build.stderr.raw', Buffer.from('different')), false, true);
  rejects(f => { const x = JSON.parse(f.files.get('release-build/commands/release-build.result.json')); x.command.args.push('--other');
    f.files.set('release-build/commands/release-build.result.json', Buffer.from(JSON.stringify(x) + '\n')); }, false, true);
});
test('every tool is pinned twice and retains its original exact probe command/status/streams', () => {
  for (const key of ['cargo', 'rustc', 'cc', 'linker', 'node', 'git']) {
    rejects(f => f.build.tools.after[key].sha256 = '0'.repeat(64));
    rejects(f => f.commands[key].args.push('--other'));
    rejects(f => f.results[key].termination.error = 'CLEANUP');
    rejects(f => f.results[key].termination.signal = 'SIGTERM');
    rejects(f => f.results[key].termination.exitCode = 1);
  }
  for (const key of ['cargo', 'rustc', 'node']) rejects(f => f.streams[key].stdout = Buffer.from('wrong version\n'));
  rejects(f => f.results['release-build'].termination.exitCode = 1100, true);
  rejects(f => f.streams.linker.stdout = Buffer.from('LINK : fatal error LNK1100\r\n'), true);
});
test('fresh/external exact target and retained copy bindings use selected-host path semantics', () => {
  for (const windows of [false, true]) for (const change of [f => f.build.cli.before.exists = true,
    f => f.retained.links.copy = 2, f => f.retained.links.builtAfter++, f => f.retained.copy.path = f.build.cli.after.path,
    f => f.retained.measuredPath = f.build.cli.after.path, f => f.retained.sourceAfter.bytes++,
    f => f.retained.copy.mode = '644', f => f.retained.targetDirectory += 'x',
    f => f.operator.root = f.retained.copy.path, f => f.retained.copy.path += ':alternate']) rejects(change, windows);
});
test('whole retained image digest/prefix and independent native format checks reject rebinding', () => {
  rejects(f => { const image = f.files.get('retained-release-cli/zryna'); image[200] ^= 1; });
  rejects(f => { const image = Buffer.from(f.files.get('retained-release-cli/zryna')); image[18] = 183; f.rebindImage(image); });
  rejects(f => f.rebindImage(f.files.get('retained-release-cli/zryna'), '644'));
  for (const mutate of [image => image.writeUInt16LE(0x14c, 132), image => image.writeUInt32LE(9999999, 60), image => image.write('NO', 0)])
    rejects(f => { const image = Buffer.from(f.files.get('retained-release-cli/zryna.exe')); mutate(image); f.rebindImage(image); }, true);
});
test('source epochs, complete absent config inventory, SDK and measured round bindings remain exact', () => {
  for (const change of [f => f.build.source.after.commit = 'e'.repeat(40), f => f.expectedSource.tree = 'e'.repeat(40),
    f => f.build.cargoConfiguration.before.pop(), f => f.build.cargoConfiguration.after[0].present = true,
    f => f.capture.tools.cli.sha256 = '0'.repeat(64), f => f.capture.tools.node.path += 'other',
    f => f.capture.environment.RUSTC += 'other', f => f.capture.round = 'baseline-round-2',
    f => f.capture.sourceCommit = 'e'.repeat(40), f => f.capture.cliBuildClass = 'debug', f => f.capture.sourceAuthenticated = true]) rejects(change);
  rejects(f => f.build.sdk.after[0].sha256 = '0'.repeat(64), true);
  rejects(f => { f.build.sdk.before.pop(); f.build.sdk.after.pop(); }, true);
  rejects(f => f.build.environment.RUSTFLAGS = '-Clink-arg=/INCREMENTAL', true);
});
test('bounded JSON permits both producer/canonical spelling and rejects duplicates malformed UTF8 and oversized reads', () => {
  const f = fixture(); const value = JSON.parse(f.files.get('release-build/capture.json'));
  f.files.set('release-build/capture.json', Buffer.from(canonical(value))); replayReleaseImage(f);
  for (const bytes of [Buffer.from([255]), Buffer.from('{"protocol":0,"protocol":1}\n'), Buffer.alloc(256 * 1024 + 1)])
    rejects(g => g.files.set('release-build/capture.json', bytes), false, true);
});

// Additive prospective numeric boundary controls; the original fixture and eight controls stay intact.
function numericFixture(windows = false, round = 'held-out-candidate') {
  const f = fixture(windows);
  f.operator.round = round; f.capture.round = round; f.sync();
  return f;
}
test('numeric held-out replay accepts the original role while legacy rejects the same untouched bytes', () => {
  for (const windows of [false, true]) {
    const f = numericFixture(windows);
    const before = [...f.files].map(([name, bytes]) => [name, sha256(bytes)]);
    assert.throws(() => replayReleaseImage(f));
    const result = replayNumericReleaseImage(f);
    assert.equal(result.round, 'held-out-candidate'); assert.equal(result.rawBuildImageReconciled, true);
    assert.deepEqual([...f.files].map(([name, bytes]) => [name, sha256(bytes)]), before);
    assert.equal(f.operator.round, 'held-out-candidate'); assert.equal(f.capture.round, 'held-out-candidate');
    for (const key of ['sourceAuthenticated', 'hostAuthenticated', 'performanceQualified', 'productionReleaseAdmission']) assert.equal(result[key], false);
    assert.equal(result.baselineAuthority, null); assert.equal(result.thresholds, null);
  }
});
test('numeric baseline replay exactly agrees with the legacy result and no extra mode argument is accepted', () => {
  assert.equal(replayReleaseImage.length, 1); assert.equal(replayNumericReleaseImage.length, 1);
  for (const windows of [false, true]) for (const round of ['baseline-round-1', 'baseline-round-2']) {
    const f = numericFixture(windows, round);
    assert.deepEqual(replayNumericReleaseImage(f), replayReleaseImage(f));
  }
  assert.throws(() => replayNumericReleaseImage(numericFixture(), true), /fixed numeric release image replay API/u);
});
test('numeric roles are closed even when both original records agree on a malformed role', () => {
  for (const windows of [false, true]) for (const role of ['io-qualification', 'baseline-round-3', 'heldout',
    'held-out-candidate ', '', null, 1, {}, []]) {
    const f = numericFixture(windows, role);
    assert.throws(() => replayNumericReleaseImage(f));
  }
});
test('numeric replay rejects original operator/capture role drift without relabeling or poisoning pristine replay', () => {
  for (const windows of [false, true]) for (const driftOperator of [false, true]) {
    const f = numericFixture(windows);
    if (driftOperator) f.operator.round = 'baseline-round-1'; else f.capture.round = 'baseline-round-1';
    f.sync(); assert.throws(() => replayNumericReleaseImage(f));
    const pristine = numericFixture(windows);
    assert.equal(replayNumericReleaseImage(pristine).round, 'held-out-candidate');
  }
});
test('numeric held-out replay retains independent build/image/tool/source and authority rejection', () => {
  const changes = [f => { f.commands['release-build'].args.splice(3, 1); },
    f => { f.streams['release-build'].stderr = Buffer.from('Finished `dev` profile [unoptimized]\n'); },
    f => { f.build.tools.after.rustc.sha256 = '0'.repeat(64); },
    f => { f.expectedSource.tree = 'e'.repeat(40); }, f => { f.capture.sourceAuthenticated = true; },
    f => { f.retained.links.copy = 2; },
    f => { const name = f.host === 'windows-x86_64' ? 'zryna.exe' : 'zryna'; f.files.get(`retained-release-cli/${name}`)[200] ^= 1; }];
  for (const windows of [false, true]) for (const change of changes) {
    const f = numericFixture(windows); change(f); f.sync();
    assert.throws(() => replayNumericReleaseImage(f));
    assert.equal(replayNumericReleaseImage(numericFixture(windows)).round, 'held-out-candidate');
  }
});
