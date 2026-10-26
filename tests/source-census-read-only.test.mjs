import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { captureSourceCensus } from '../scripts/stability-gates/source-census-v2.mjs';

test('source census preserves actual index bytes when Git would refresh stale stat metadata', t => {
  const stage = mkdtempSync(join(tmpdir(), 'm418-readonly-git-'));
  t.after(() => rmSync(stage, { recursive: true, force: true }));
  const root = join(stage, 'source'); mkdirSync(root);
  const git = args => execFileSync('git', ['-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=' + stage,
    '-c', 'user.name=Census test', '-c', 'user.email=census-test@example.invalid', ...args],
  { cwd: root, env: process.env, timeout: 30000, stdio: ['ignore', 'pipe', 'pipe'] });
  git(['init', '--object-format=sha1']);
  const source = join(root, 'program.txt');
  writeFileSync(source, 'tracked source bytes\n'); git(['add', 'program.txt']); git(['commit', '-m', 'Fixture']);
  const indexPath = join(root, '.git', 'index');
  const before = readFileSync(indexPath);
  const identity = statSync(source);
  utimesSync(source, identity.atime, new Date(identity.mtimeMs - 5000));
  const result = captureSourceCensus({ root, directory: join(stage, 'observed'), executable: 'git', env: process.env });
  assert.equal(result.files, 1);
  assert(readFileSync(indexPath).equals(before), 'read-only census must not refresh the Git index stat cache');
});
