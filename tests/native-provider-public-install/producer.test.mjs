import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, realpathSync, mkdirSync, readFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { produceNativeInstallation } from '../../scripts/distribution/produce-native-installation.mjs';
import { executeRecorded, successful } from './commands.mjs';

const source = fileURLToPath(new URL('../..', import.meta.url));

test('real command recorder preserves nonzero output without manufacturing command success', () => {
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-native-command-')));
  try {
    const observed = executeRecorded(root, 'negative', process.execPath,
      ['-e', 'process.stdout.write("original-output"); process.stderr.write("original-error"); process.exit(7);'],
      root, { PATH: root });
    assert.equal(observed.exitCode, 7);
    assert.equal(observed.stdoutBytes.toString(), 'original-output');
    assert.equal(observed.stderrBytes.toString(), 'original-error');
    assert.equal(readFileSync(observed.stdout.path).toString(), 'original-output');
    assert.equal(readFileSync(observed.stderr.path).toString(), 'original-error');
    assert.throws(() => successful(observed), /actual command failed/);
    const retained = JSON.parse(readFileSync(join(root, 'negative.result.json')));
    assert.equal(retained.descendantCleanupClaimed, false);
  } finally { rmSync(root, { recursive: true }); }
});

test('real command timeout retains partial evidence and never claims descendant cleanup', () => {
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-native-timeout-')));
  try {
    assert.throws(() => executeRecorded(root, 'timeout', process.execPath,
      ['-e', 'setInterval(() => {}, 1000);'], root, { PATH: root }, { timeoutMs: 50 }),
    /command transport\/timeout/);
    const retained = JSON.parse(readFileSync(join(root, 'timeout.result.json')));
    assert.equal(retained.errorCode, 'ETIMEDOUT');
    assert.equal(retained.descendantCleanupClaimed, false);
    assert.equal(retained.timeoutMeansCleanupUnconfirmed, true);
    assert(existsSync(retained.stdout.path)); assert(existsSync(retained.stderr.path));
  } finally { rmSync(root, { recursive: true }); }
});

test('producer rejects checkout-owned or preexisting output before running Cargo', async () => {
  await assert.rejects(produceNativeInstallation({ source, output: source }), /fresh external/);
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-native-existing-')));
  try {
    await assert.rejects(produceNativeInstallation({ source, output: root }), /fresh external/);
    assert(!existsSync(join(root, 'producer-commands')));
  } finally { rmSync(root, { recursive: true }); }
});

test('producer preserves an actual failed source admission and creates no positive receipt', async () => {
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-native-source-rejection-')));
  const fake = join(root, 'not-a-repository'), output = join(root, 'evidence'); mkdirSync(fake);
  try {
    await assert.rejects(produceNativeInstallation({ source: fake, output }), /actual command failed/);
    const failure = JSON.parse(readFileSync(join(output, 'FAILURE.json')));
    assert.equal(failure.status, 'failed-no-native-installation-acceptance-claim');
    assert.equal(failure.stage, 'exact-source-and-builder-admission');
    assert.equal(failure.cleanupConfirmed, false);
    assert(!existsSync(join(output, 'RESULT.json')));
    assert(!existsSync(join(output, 'architecture-receipt.json')));
    assert(!existsSync(join(output, 'reproduction.json')));
  } finally { rmSync(root, { recursive: true }); }
});
