import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, renameSync, symlinkSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { checkRepository } from '../scripts/check-repository-structure.mjs';
import { physicalLines, POLICY_PATH, validatePolicy } from '../scripts/structure/policy.mjs';
import { git } from '../scripts/structure/repository.mjs';
import { PREFLIGHT_COMMANDS, validatePreflightCommands } from '../scripts/run-preflight.mjs';
import { validateManifestDocument, validatePackageDocument } from '../scripts/run-m0-conformance.mjs';

const today = '2026-09-06';
const lines = count => '// source\n'.repeat(count);
const metadata = { owner: 'compiler maintainers', reason: 'One cohesive reviewed responsibility', review: 'https://github.com/zryna/zryna/issues/314' };

test('frozen preflight and M0 declarations cannot silently omit structure enforcement', () => {
  assert.equal(PREFLIGHT_COMMANDS[0].id, 'repository-structure');
  assert.throws(() => validatePreflightCommands(PREFLIGHT_COMMANDS.slice(1)), /frozen/);
  const manifest = JSON.parse(readFileSync(new URL('./m0-conformance-v1.json', import.meta.url)));
  assert.doesNotThrow(() => validateManifestDocument(manifest));
  for (const id of ['repository-structure', 'repository-structure-tests']) {
    const changed = structuredClone(manifest);
    changed.commands = changed.commands.filter(command => command.id !== id);
    assert.throws(() => validateManifestDocument(changed));
  }
  const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url)));
  delete pkg.scripts['structure:check'];
  assert.throws(() => validatePackageDocument(pkg), /structure:check/);
  const workflow = readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  assert.match(workflow, /ZRYNA_STRUCTURE_BASE:.*pull_request.base.sha.*github.event.before/);
  assert.match(workflow, /os: \[ubuntu-latest, windows-latest\]/);
  assert.equal((workflow.match(/uses: actions\/checkout@/g) ?? []).length,
    (workflow.match(/fetch-depth: 0/g) ?? []).length);
});
function fixture(t, count = 600, testOnly = false) {
  const root = mkdtempSync(join(tmpdir(), 'zryna-structure-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (path, content) => {
    mkdirSync(join(root, path, '..'), { recursive: true });
    writeFileSync(join(root, path), content);
  };
  git(root, ['init', '-q']);
  git(root, ['config', 'user.name', 'Structure test']);
  git(root, ['config', 'user.email', 'structure@example.invalid']);
  git(root, ['config', 'core.autocrlf', 'false']);
  const save = () => { git(root, ['add', '.']); git(root, ['commit', '-qm', 'Checkpoint']); return git(root, ['rev-parse', 'HEAD']).trim(); };
  write('src/main.rs', lines(count));
  write('docs/CODE_NAVIGATION.md', '[source](../src/main.rs)\n');
  const anchor = save();
  const policy = { version: 1, anchor, baseline: count > 500 ? [{ path: 'src/main.rs', lines: count, production: !testOnly }] : [], classifications: testOnly ? [{ path: 'src/main.rs', kind: 'test', ...metadata }] : [], exceptions: [] };
  const savePolicy = () => write(POLICY_PATH, `${JSON.stringify(policy, null, 2)}\n`);
  savePolicy();
  const initial = save();
  const check = options => checkRepository({ root, today, bootstrap: anchor, base: 'HEAD', ...options });
  return { root, write, save, policy, savePolicy, check, anchor, initial };
}

test('physical counts are portable at exact warning and hard boundaries', () => {
  assert.equal(physicalLines(''), 0);
  assert.equal(physicalLines('text'), 1);
  assert.equal(physicalLines('\r\n'), 1);
  for (const count of [349, 350, 500, 501]) {
    assert.equal(physicalLines(lines(count)), count);
    assert.equal(physicalLines(lines(count).replaceAll('\n', '\r\n')), count);
    assert.equal(physicalLines(lines(count).slice(0, -1)), count);
  }
});

test('new files warn at 350 through 500 and reject 501, including fixture disguises', t => {
  const f = fixture(t);
  for (const count of [349, 350, 500, 501]) {
    f.write('src/fixtures/production.rs', lines(count));
    const result = f.check();
    assert.equal(result.ok, count <= 500);
    assert.equal(result.messages.some(message => message.startsWith('WARN')), count >= 350 && count <= 500);
  }
  f.write('src/fixtures/production.rs', lines(100));
  assert.equal(f.check().ok, true);
  f.write('.gitignore', 'ignored.rs\n');
  f.write('ignored.rs', lines(501));
  assert.equal(f.check().ok, false);
});

test('trusted reductions ratchet; working tree, index and committed base cannot raise the ceiling', t => {
  const f = fixture(t);
  assert.equal(f.check().ok, true);
  f.write('src/main.rs', lines(601));
  assert.equal(f.check().ok, false);
  git(f.root, ['add', 'src/main.rs']);
  assert.equal(f.check().ok, false);
  f.write('src/main.rs', lines(550));
  assert.equal(f.check().ok, true);
  f.save();
  f.write('src/main.rs', lines(551));
  assert.match(f.check().messages.join('\n'), /exceeds 550/);
  f.write('src/main.rs', lines(500));
  f.save();
  f.write('src/main.rs', lines(501));
  assert.match(f.check().messages.join('\n'), /exceeds 500/);
  f.policy.baseline[0].lines = 700;
  f.savePolicy();
  assert.throws(() => f.check(), /baseline differs/);
});

test('renames preserve grandfathering and reductions; deletion keeps historical inventory valid', t => {
  const f = fixture(t);
  git(f.root, ['mv', 'src/main.rs', 'src/renamed.rs']);
  f.write('docs/CODE_NAVIGATION.md', '[source](../src/renamed.rs)\n');
  assert.equal(f.check().ok, true);
  f.write('src/renamed.rs', lines(550));
  f.save();
  f.write('src/renamed.rs', lines(551));
  assert.match(f.check().messages.join('\n'), /exceeds 550/);
  rmSync(join(f.root, 'src/renamed.rs'));
  f.write('docs/CODE_NAVIGATION.md', '[policy](../scripts/repository-structure-policy.json)\n');
  assert.equal(f.check().ok, true);
});

test('exceptions require exact complete metadata, ceiling and exclusive UTC expiry', t => {
  const f = fixture(t, 100);
  f.write('src/main.rs', lines(501));
  f.policy.exceptions.push({ path: 'src/main.rs', ...metadata, ceiling: 501, expires: '2026-09-07' });
  f.savePolicy();
  assert.equal(f.check().ok, true);
  assert.match(f.check().messages.join('\n'), /explicit maintainer review/);
  f.write('src/main.rs', lines(502));
  assert.equal(f.check().ok, false);
  for (const mutate of [
    p => { p.exceptions[0].expires = today; },
    p => { p.exceptions[0].expires = '2026-02-30'; },
    p => { p.exceptions[0].path = 'src/*'; },
    p => { delete p.exceptions[0].owner; },
    p => { p.exceptions[0].reason = ''; },
    p => { p.exceptions[0].ceiling = '700'; },
    p => p.exceptions.push({ ...p.exceptions[0] }),
  ]) {
    const changed = structuredClone(f.policy);
    mutate(changed);
    assert.throws(() => validatePolicy(changed, today));
  }
  f.write('src/main.rs', lines(500));
  assert.throws(() => f.check(), /stale exception/);
});

test('test and generated exemptions are exact reviewed records, with stale paths rejected', t => {
  const f = fixture(t, 100);
  for (const kind of ['test', 'fixture', 'generated']) {
    f.write('tests/large.rs', lines(501));
    assert.equal(f.check().ok, false);
    f.policy.classifications = [{ path: 'tests/large.rs', kind, ...metadata }];
    f.savePolicy();
    assert.equal(f.check().ok, true);
    f.write('tests/unlisted.rs', '// generated\n' + lines(501));
    assert.equal(f.check().ok, false);
    rmSync(join(f.root, 'tests/unlisted.rs'));
    f.policy.classifications = [];
    f.savePolicy();
  }
  f.policy.classifications = [{ path: 'missing.rs', kind: 'test', ...metadata }];
  f.savePolicy();
  assert.throws(() => f.check(), /stale policy path/);
});

test('initial test-only source never acquires grandfathering after reclassification', t => {
  const f = fixture(t, 600, true);
  assert.equal(f.check().ok, true);
  f.policy.classifications = [];
  f.savePolicy();
  assert.equal(f.check().ok, false);
  f.policy.baseline[0].production = true;
  f.savePolicy();
  assert.throws(() => f.check(), /baseline differs/);
});

test('missing or ambiguous authority fails closed; linked worktrees and explicit PR bases work', t => {
  const f = fixture(t);
  assert.throws(() => f.check({ base: 'main' }), /full immutable/);
  assert.throws(() => f.check({ base: 'a'.repeat(40) }), /authority unavailable/);
  const linked = join(f.root, '..', `${f.root.split(/[\\/]/).at(-1)}-linked`);
  git(f.root, ['worktree', 'add', '--detach', linked, 'HEAD']);
  t.after(() => rmSync(linked, { recursive: true, force: true }));
  assert.equal(checkRepository({ root: linked, base: f.initial, today }).ok, true);
  f.write('src/main.rs', lines(550));
  f.save();
  assert.equal(f.check({ base: f.initial }).ok, true);
});

test('shallow clone without required anchor rejects rather than skipping history', t => {
  const f = fixture(t);
  const shallow = join(f.root, 'shallow');
  git(f.root, ['clone', '--depth=1', '--no-local', f.root, shallow]);
  assert.throws(() => checkRepository({ root: shallow, base: 'HEAD', today }), /authority unavailable/);
});

test('unsafe paths, portable case collisions, symlink ancestors and broken navigation fail', t => {
  const f = fixture(t);
  f.policy.classifications.push({ path: '../outside.rs', kind: 'test', ...metadata });
  f.savePolicy();
  assert.throws(() => f.check(), /invalid exact path/);
  f.policy.classifications = [];
  f.savePolicy();
  const hash = git(f.root, ['rev-parse', 'HEAD:src/main.rs']).trim();
  git(f.root, ['update-index', '--add', '--cacheinfo', `100644,${hash},src/Main.rs`]);
  assert.throws(() => f.check(), /case-colliding/);
  git(f.root, ['update-index', '--force-remove', 'src/Main.rs']);
  renameSync(join(f.root, 'src'), join(f.root, 'real'));
  symlinkSync(join(f.root, 'real'), join(f.root, 'src'), process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => f.check(), /no-symlink/);
  unlinkSync(join(f.root, 'src'));
  renameSync(join(f.root, 'real'), join(f.root, 'src'));
  for (const target of ['../missing.rs', '../../outside.rs', '../src', 'https://example.invalid']) {
    f.write('docs/CODE_NAVIGATION.md', `[bad](${target})\n`);
    assert.throws(() => f.check());
  }
  f.write('docs/CODE_NAVIGATION.md', '[source](../src/main.rs)\n');
  assert.equal(f.check().ok, true);
});

test('diagnostics and recovery are deterministic and checker never mutates input', t => {
  const f = fixture(t);
  f.write('src/main.rs', lines(601));
  const status = git(f.root, ['status', '--porcelain=v1']);
  const policy = readFileSync(join(f.root, POLICY_PATH));
  const source = readFileSync(join(f.root, 'src/main.rs'));
  assert.deepEqual(f.check(), f.check());
  assert.equal(git(f.root, ['status', '--porcelain=v1']), status);
  assert.deepEqual(readFileSync(join(f.root, POLICY_PATH)), policy);
  assert.deepEqual(readFileSync(join(f.root, 'src/main.rs')), source);
  f.write('src/main.rs', lines(599));
  assert.equal(f.check().ok, true);
});

test('squashed adoption works in a fresh full clone without the bootstrap object', t => {
  const f = fixture(t);
  // The reviewed branch reduced the file before its policy was squash-adopted.
  f.write('src/main.rs', lines(550));
  f.save();
  const tree = git(f.root, ['rev-parse', 'HEAD^{tree}']).trim();
  const squash = git(f.root, ['commit-tree', tree, '-m', 'Squash adopted policy']).trim();
  git(f.root, ['checkout', '-qb', 'adopted', squash]);
  const cloned = join(f.root, '..', `${f.root.split(/[\\/]/).at(-1)}-clone`);
  t.after(() => rmSync(cloned, { recursive: true, force: true }));
  git(f.root, ['clone', '--no-local', '--single-branch', '--branch', 'adopted', f.root, cloned]);
  assert.throws(() => git(cloned, ['cat-file', '-e', f.anchor]), /authority unavailable/);
  const check = () => checkRepository({ root: cloned, base: squash, today });
  assert.equal(check().ok, true);
  assert.deepEqual(check(), check());
  writeFileSync(join(cloned, 'src/main.rs'), lines(551));
  assert.match(check().messages.join('\n'), /exceeds 550/);
  writeFileSync(join(cloned, 'src/main.rs'), lines(550));
  git(cloned, ['mv', 'src/main.rs', 'src/renamed.rs']);
  writeFileSync(join(cloned, 'docs/CODE_NAVIGATION.md'), '[source](../src/renamed.rs)\n');
  assert.equal(check().ok, true);
  writeFileSync(join(cloned, 'src/renamed.rs'), lines(551));
  assert.match(check().messages.join('\n'), /exceeds 550/);
  writeFileSync(join(cloned, 'src/renamed.rs'), lines(550));
  assert.equal(check().ok, true);
});

test('adopted inventory and anchor cannot be changed even in a later comparison commit', t => {
  for (const mutate of [
    p => { p.baseline[0].lines = 601; },
    p => { p.baseline[0].production = false; },
    p => { p.anchor = 'a'.repeat(40); },
  ]) {
    const f = fixture(t);
    mutate(f.policy);
    f.savePolicy();
    assert.throws(() => f.check(), /baseline differs|anchor differs/);
    f.save();
    assert.throws(() => f.check(), /differs from original trusted policy adoption/);
  }
});

test('bootstrap still authenticates exact inventory and rejects invented anchors', t => {
  const f = fixture(t);
  assert.equal(f.check({ base: f.anchor }).ok, true);
  f.policy.baseline[0].lines = 601;
  f.savePolicy();
  assert.throws(() => f.check({ base: f.anchor }), /exact anchored source inventory/);
  f.policy.baseline[0].lines = 600;
  f.policy.anchor = 'a'.repeat(40);
  f.savePolicy();
  assert.throws(() => f.check({ base: f.anchor }), /anchor differs/);
});

test('deleted and reintroduced policy has ambiguous adoption and cannot reset ceilings', t => {
  const f = fixture(t);
  unlinkSync(join(f.root, POLICY_PATH));
  f.save();
  f.savePolicy();
  f.save();
  assert.throws(() => f.check(), /adoption history is absent or ambiguous/);
});
