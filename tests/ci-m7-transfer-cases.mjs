// Exact M7 job relocation inverse before the unchanged historical workflow authority.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { parseDocument } from 'yaml';
const BASE_PARSED_SHA = '7c0524b1319ac4aebc994cb6cf31b652996d6a1cb01541069d12f76471933e6c';
const BASE_TEXT_SHA = '0b2a306faf024161237b7ab2de9e21d135919987285f082738b24165f8577e4a';
const REQUIRE = ' && test "$M7_PLATFORM_RESULT" = success';
const JOB_TEXT = `  m7-platform:
    name: m7 generic (\${{ matrix.os }})
    needs: [rust]
    # Three unchanged 25-minute commands on Windows, one on Linux; 15 minutes for setup and retention.
    timeout-minutes: \${{ matrix.os == 'windows-latest' && 90 || 40 }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: \${{ matrix.os }}
    env:
      CARGO_PROFILE_DEV_DEBUG: "\${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}"
      CARGO_PROFILE_TEST_DEBUG: "\${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}"
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c
        with:
          toolchain: 1.97.1
          components: rustfmt, clippy
      - uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7.0.0
        with:
          node-version: 22.22.1
      - uses: pnpm/action-setup@0977fd99725f1db4007ccb2928dbb4e90d06cc86 # v6.0.10
        with:
          version: 11.18.0
      - uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7.0.0
        with:
          node-version: 22.22.1
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - name: Fetch locked Rust dependencies
        run: cargo fetch --locked
      - name: Prepare pinned SQLite source fixture
        if: runner.os == 'Linux'
        timeout-minutes: 3
        shell: bash
        run: |
          python3 -B scripts/m5/prepare_sqlite_fixture.py --output "$RUNNER_TEMP/zryna-sqlite-source"
          printf 'ZRYNA_SQLITE_FIXTURE_ROOT=%s\\n' "$RUNNER_TEMP/zryna-sqlite-source" >> "$GITHUB_ENV"
      - name: Verify public bounded-generics acceptance packet
        id: m7-generic
        env:
          ZRYNA_M7_UPSTREAM_M0_RESULT: \${{ needs.rust.result }}
          ZRYNA_M7_PR_HEAD: \${{ github.event.pull_request.head.sha || '' }}
        run: node scripts/run-m7-generic-acceptance.mjs --evidence "\${{ runner.temp }}/m7-generic-acceptance" --expected-source-revision "\${{ github.sha }}"
      - name: Retain M7 generic acceptance commands and actual counts
        if: \${{ !cancelled() && steps.m7-generic.outcome != 'skipped' }}
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          name: m7-generic-\${{ matrix.os }}-\${{ github.sha }}-\${{ github.run_id }}-\${{ github.run_attempt }}
          path: \${{ runner.temp }}/m7-generic-acceptance
          if-no-files-found: error
          retention-days: 90
`;
const OLD_PACKET_TEXT = `      - name: Verify public bounded-generics acceptance packet
        id: m7-generic
        env:
          ZRYNA_M7_UPSTREAM_M0_RESULT: \${{ steps.m0-complete.outcome }}
          ZRYNA_M7_PR_HEAD: \${{ github.event.pull_request.head.sha || '' }}
        run: node scripts/run-m7-generic-acceptance.mjs --evidence "\${{ runner.temp }}/m7-generic-acceptance" --expected-source-revision "\${{ github.sha }}"
      - name: Retain M7 generic acceptance commands and actual counts
        if: \${{ !cancelled() && steps.m7-generic.outcome != 'skipped' }}
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          name: m7-generic-\${{ matrix.os }}-\${{ github.sha }}-\${{ github.run_id }}-\${{ github.run_attempt }}
          path: \${{ runner.temp }}/m7-generic-acceptance
          if-no-files-found: error
          retention-days: 90
`;
const expectedJob = parseDocument('jobs:\n' + JOB_TEXT).toJS().jobs['m7-platform'];
const digest = value => createHash('sha256').update(value).digest('hex');
function ordered(value) {
  if (Array.isArray(value)) return value.map(ordered);
  if (value !== null && typeof value === 'object')
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, ordered(value[key])]));
  return value;
}
export function withoutM7Transfer(candidate) {
  const original = structuredClone(candidate);
  if (!Object.hasOwn(original.jobs, 'm7-platform')) return original; // Older reviewed fixtures continue to their existing authority.
  assert.deepEqual(original.jobs['m7-platform'], expectedJob, 'exact dedicated both-host M7 job');
  const rust = original.jobs.rust;
  assert.deepEqual(rust.steps.at(-1), { id: 'm0-complete',
    env: { RUST_TEST_THREADS: '2' }, run: 'node scripts/run-m0-conformance.mjs' },
  'exact bounded M0 test scheduling; no additional environment grant');
  delete rust.steps.at(-1).env; // Restore only this reviewed scheduling declaration before old authority.
  const moved = structuredClone(expectedJob.steps.slice(-2));
  moved[0].env.ZRYNA_M7_UPSTREAM_M0_RESULT = '${{ steps.m0-complete.outcome }}';
  rust.steps.push(...moved); delete original.jobs['m7-platform'];
  const aggregate = original.jobs.m0, gate = aggregate.steps.at(-1);
  assert.equal(aggregate.needs.at(-1), 'm7-platform');
  assert.equal(aggregate.needs.filter(id => id === 'm7-platform').length, 1); aggregate.needs.pop();
  assert.equal(gate.env.M7_PLATFORM_RESULT, '${{ needs.m7-platform.result }}'); delete gate.env.M7_PLATFORM_RESULT;
  assert(gate.run.endsWith(REQUIRE)); gate.run = gate.run.slice(0, -REQUIRE.length);
  assert.equal(digest(JSON.stringify(ordered(original))), BASE_PARSED_SHA, 'every other parsed base workflow value unchanged');
  return original;
}
export function withoutM7TransferText(text) {
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 512 * 1024);
  const document = parseDocument(text, { uniqueKeys: true }); assert.deepEqual(document.errors, []);
  if (!Object.hasOwn(document.toJS().jobs, 'm7-platform')) return text;
  withoutM7Transfer(document.toJS());
  assert.equal(text.split(JOB_TEXT + '\n').length, 2, 'exact dedicated M7 job text once');
  let legacy = text.replace(JOB_TEXT + '\n', '');
  const anchor = '      - id: m0-complete\n        run: node scripts/run-m0-conformance.mjs\n';
  const current = '      - id: m0-complete\n        env:\n          RUST_TEST_THREADS: \'2\'\n' +
    '        run: node scripts/run-m0-conformance.mjs\n';
  assert.equal(legacy.split(current).length, 2, 'exact step-local bounded M0 scheduling placement');
  legacy = legacy.replace(current, anchor + OLD_PACKET_TEXT);
  legacy = legacy.replace(', m7-platform]', ']')
    .replace('          M7_PLATFORM_RESULT: ${{ needs.m7-platform.result }}\n', '').replace(REQUIRE, '');
  assert.equal(digest(legacy), BASE_TEXT_SHA, 'every other original raw workflow byte unchanged'); return legacy;
}
