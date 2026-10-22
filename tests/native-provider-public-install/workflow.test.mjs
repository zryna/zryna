import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const text = readFileSync(new URL('../../.github/workflows/native-provider-install.yml', import.meta.url), 'utf8');

test('native installation workflow binds actual candidate and both supported physical hosts', () => {
  assert(text.includes('os: [ubuntu-24.04, windows-2022]'));
  assert(text.includes('timeout-minutes: 120'));
  assert(text.includes('ref: ${{ github.event.pull_request.head.sha || github.sha }}'));
  assert(text.includes('persist-credentials: false'));
  assert(text.includes('toolchain: 1.97.1')); assert(text.includes('node-version: 22.22.1'));
  assert(text.includes('contents: read')); assert(!text.includes('pnpm'));
  assert(text.includes('Launch-VsDevShell.ps1')); assert(text.includes('"LIB=$env:LIB"'));
  assert(text.includes('"CARGO_HOME=$((Resolve-Path')); assert(text.includes('"RUSTUP_HOME=$((Resolve-Path'));
  for (const line of text.split('\n').filter(line => line.includes('uses: '))) {
    assert(/@[a-f0-9]{40}(?:\s|$)/.test(line), 'whole pinned action revision');
  }
});

test('native installation workflow executes real producer and retains failures without target trees', () => {
  assert(text.includes('cargo fetch --locked'));
  assert(text.includes('node scripts/distribution/produce-native-installation.mjs --source "$env:GITHUB_WORKSPACE" --output "$output" --jobs 2'));
  assert(text.includes("if ($LASTEXITCODE -ne 0) { throw 'Actual native installation qualification failed"));
  assert(text.includes('if: always()')); assert(text.includes('include-hidden-files: true'));
  for (const pattern of ['qualification/*.json', 'qualification/*.tar.gz', 'qualification/*.zip',
    'qualification/producer-commands/**', 'qualification/smoke-commands/**', 'qualification/ordinary-projects/**']) {
    assert(text.includes(pattern));
  }
  assert(!text.includes('qualification/**')); assert(!text.includes('continue-on-error'));
  assert(!text.includes('--source-ref')); assert(!text.includes('RESULT.json >'));
});
