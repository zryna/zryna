// Strict reviewed relocation only; never a fallback or replacement CI authority.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { parseDocument } from 'yaml';
const LEGACY_SHA = '8e1cce2fa04942e3e18cd676684b922582ade90e6fb38ae2f408f9d3e4de8173';
const LEGACY_TEXT_SHA = '77bc79ec0734b9510b2665f2ec0706a3771b5371e4b8289476c5c7a7d8b0c7a2';
const HOST_DEBUG_PROFILE = {
  CARGO_PROFILE_DEV_DEBUG: "${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}",
  CARGO_PROFILE_TEST_DEBUG: "${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}",
};
const HOST_DEBUG_TEXT = '    env:\n' + Object.entries(HOST_DEBUG_PROFILE)
  .map(([key, value]) => `      ${key}: "${value}"\n`).join('');
const M4_DEBUG_KEYS = ['CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG'];
const M4_DEBUG_TEXT = '      # Keep full core verification inside its fixed job budget without debug-symbol generation.\n' +
  "      CARGO_PROFILE_DEV_DEBUG: '0'\n      CARGO_PROFILE_TEST_DEBUG: '0'\n";
const MOVED = [
  {
    "name": "Verify M2 semantics",
    "run": "cargo test --locked -p zryna-semantics --lib control_flow_v1"
  },
  {
    "name": "Prepare pinned SQLite source fixture",
    "if": "runner.os == 'Linux'",
    "timeout-minutes": 3,
    "shell": "bash",
    "run": "python3 -B scripts/m5/prepare_sqlite_fixture.py --output \"$RUNNER_TEMP/zryna-sqlite-source\"\nprintf 'ZRYNA_SQLITE_FIXTURE_ROOT=%s\\n' \"$RUNNER_TEMP/zryna-sqlite-source\" >> \"$GITHUB_ENV\"\n"
  },
  {
    "name": "Verify M2 driver security",
    "run": "cargo test --locked -p zryna-driver --lib"
  },
  {
    "name": "Verify native source resource limits",
    "if": "runner.os == 'Windows'",
    "run": "node scripts/run-native-source-resource-tests.mjs"
  },
  {
    "name": "Verify exact Windows directory lifecycle",
    "if": "runner.os == 'Windows'",
    "run": "cargo test --locked -p zryna-windows-filesystem -- --show-output"
  },
  {
    "name": "Verify public M2 CLI profile",
    "run": "cargo test --locked -p zryna --test cli control_flow_"
  },
  {
    "name": "Run Windows CLI smoke",
    "if": "runner.os == 'Windows'",
    "run": "cargo test --locked -p zryna --test cli javascript_build_and_run_publish_exact_bundles -- --exact"
  }
];
const RESULT = '${{ needs.m2-platform.result }}';
const REQUIRE = ' && test "$M2_PLATFORM_RESULT" = success';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
function ordered(value) {
  if (Array.isArray(value)) return value.map(ordered);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, ordered(value[key])]));
  }
  assert(value === null || ['string', 'boolean', 'number'].includes(typeof value));
  if (typeof value === 'number') assert(Number.isFinite(value));
  return value;
}
function acyclic(jobs) {
  const visiting = new Set(), complete = new Set();
  function visit(id) {
    assert(Object.hasOwn(jobs, id), 'known workflow dependency');
    assert(!visiting.has(id), 'acyclic workflow dependencies');
    if (complete.has(id)) return;
    visiting.add(id);
    const needs = jobs[id].needs ?? [];
    for (const parent of typeof needs === 'string' ? [needs] : needs) visit(parent);
    visiting.delete(id); complete.add(id);
  }
  for (const id of Object.keys(jobs)) visit(id);
}
export function withoutDriverTransfer(candidate) {
  const original = structuredClone(candidate);
  const rust = original.jobs.rust, m2 = original.jobs['m2-platform'], m0 = original.jobs.m0;
  if (Object.hasOwn(rust, 'env')) {
    assert.deepEqual(rust.env, HOST_DEBUG_PROFILE, 'only reviewed Linux host debug profile');
    delete rust.env;
  }
  const m4 = original.jobs['m4-platform'];
  if (M4_DEBUG_KEYS.some(key => Object.hasOwn(m4.env, key))) {
    for (const key of M4_DEBUG_KEYS) {
      assert.equal(m4.env[key], '0', 'only the complete reviewed M4 debug-symbol pair');
      delete m4.env[key];
    }
  }
  assert.deepEqual(m2.steps.slice(7, 14), MOVED, 'exact seven moved steps after fetch');
  assert.equal(m2.steps[6].name, 'Fetch locked Rust dependencies');
  assert.deepEqual(m2.steps[14], { run: 'pnpm m2:check' }, 'moved work precedes canonical M2');
  assert.equal(m2.steps.length, 15);
  assert.equal(rust.steps[7].name, 'Fetch locked Rust dependencies');
  assert.deepEqual(rust.steps[8], MOVED[1], 'Rust retains its own exact job-local SQLite acquisition');
  assert.equal(rust.steps[9].name, 'Verify restricted playground contracts');
  assert.equal(rust.steps.length, 20);
  assert.deepEqual(m2.steps[1], rust.steps[1], 'same pinned Rust tool and components');
  assert.equal(m2.steps[1].with.components, 'rustfmt, clippy');
  assert.deepEqual(m2.strategy, rust.strategy, 'same two host matrix/fail-fast policy');
  assert.equal(rust['timeout-minutes'], "${{ matrix.os == 'windows-latest' && 60 || 40 }}");
  assert.equal(m2['timeout-minutes'], 30);
  assert.equal(m2.needs, undefined, 'M2 platform remains independently scheduled');
  assert.equal(m0.needs.at(-1), 'm2-platform', 'M0 waits for transferred proof');
  assert.equal(m0.needs.filter(id => id === 'm2-platform').length, 1);
  const gate = m0.steps.at(-1);
  assert.equal(gate.env.M2_PLATFORM_RESULT, RESULT, 'actual upstream result binding');
  assert(gate.run.endsWith(REQUIRE), 'M0 requires successful transferred proof');
  acyclic(original.jobs);
  m2.steps.splice(7, 7);
  rust.steps.splice(8, 1, ...structuredClone(MOVED));
  delete m2.steps[1].with.components;
  m0.needs.pop(); delete gate.env.M2_PLATFORM_RESULT;
  gate.run = gate.run.slice(0, -REQUIRE.length);
  assert.equal(digest(JSON.stringify(ordered(original))), LEGACY_SHA,
    'whole legacy parsed workflow authority unchanged');
  return original;
}
function editJob(text, id, edit) {
  const marker = `  ${id}:\n`, start = text.indexOf(`\n${marker}`) + 1;
  assert(start > 0 && text.indexOf(`\n${marker}`, start) === -1, 'one workflow job');
  const rest = text.slice(start + marker.length);
  const next = /\n  [A-Za-z0-9_-]+:\n/u.exec(rest);
  const end = next ? start + marker.length + next.index + 1 : text.length;
  return text.slice(0, start) + edit(text.slice(start, end)) + text.slice(end);
}
export function withoutDriverTransferText(text) {
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 512 * 1024);
  const parsed = parseDocument(text, { uniqueKeys: true }); assert.deepEqual(parsed.errors, []);
  withoutDriverTransfer(parsed.toJS()); // once at the raw-reader boundary, before old projections
  let moved;
  let legacy = editJob(text, 'm2-platform', job => {
    const start = job.indexOf('      - name: Verify M2 semantics\n');
    const end = job.indexOf('      - run: pnpm m2:check\n'); assert(start > 0 && end > start);
    moved = job.slice(start, end);
    return (job.slice(0, start) + job.slice(end)).replace('          components: rustfmt, clippy\n', '');
  });
  legacy = editJob(legacy, 'rust', job => {
    if (Object.hasOwn(parsed.toJS().jobs.rust, 'env')) {
      const profile = '    runs-on: ${{ matrix.os }}\n' + HOST_DEBUG_TEXT + '    steps:\n';
      assert.equal(job.split(profile).length, 2, 'exact job-local host debug profile placement');
      job = job.replace(profile, '    runs-on: ${{ matrix.os }}\n    steps:\n');
    }
    const anchor = '      - name: Fetch locked Rust dependencies\n        run: cargo fetch --locked\n';
    const begin = moved.indexOf('      - name: Prepare pinned SQLite source fixture\n');
    const end = moved.indexOf('      - name: Verify M2 driver security\n');
    assert(begin >= 0 && end > begin);
    const sqlite = moved.slice(begin, end);
    assert.equal(job.split(anchor + sqlite).length, 2, 'exact remaining Rust SQLite prerequisite');
    return job.replace(anchor + sqlite, anchor + moved);
  });
  legacy = editJob(legacy, 'm0', job => job.replace(', m2-platform]', ']')
    .replace('          M2_PLATFORM_RESULT: ${{ needs.m2-platform.result }}\n', '')
    .replace(REQUIRE, ''));
  legacy = editJob(legacy, 'm4-platform', job => {
    if (M4_DEBUG_KEYS.some(key => Object.hasOwn(parsed.toJS().jobs['m4-platform'].env, key))) {
      const block = '      RUST_TEST_THREADS: 3\n' + M4_DEBUG_TEXT + '    steps:\n';
      assert.equal(job.split(block).length, 2, 'exact reviewed M4 debug-symbol text placement');
      return job.replace(block, '      RUST_TEST_THREADS: 3\n    steps:\n');
    }
    return job;
  });
  assert.equal(digest(legacy), LEGACY_TEXT_SHA, 'exact legacy text for unchanged text assertions');
  return legacy;
}
