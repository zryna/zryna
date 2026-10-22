import { expectedDocuments } from './docs-bundle/publication-inventory.mjs';
import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import './code-navigation-cases.mjs';
import './cross-target-profile-contract.test.mjs';
import './minimal-library-contract.test.mjs';
import './js-wasm-adapter-contract.test.mjs';
import './docs-playground.test.mjs';
import './docs-playground-output.test.mjs';
import './native-c-review-contract.test.mjs';
import './beta-documentation-contract.test.mjs';
import './m4-public-docs.test.mjs';

import {
  compilerWorkspaceRoot,
  exportDocsBundle,
  loadRegistry,
  validateDocsBundle,
} from '../scripts/docs/bundle.mjs';

const COMMIT = '0123456789abcdef0123456789abcdef01234567';
const PROVENANCE = {
  channel: 'next',
  sourceCommit: COMMIT,
  sourceRef: 'refs/heads/main',
  verifyGit: false,
  enforceWorkspaceOutput: false,
};

test('ownership authority documents code-format generic type spellings', async () => {
  function checkGenericSpellings(markdown) {
    for (const match of markdown.matchAll(/(?:Shared|Weak)<[^>\n]+>/g)) {
      assert.equal(markdown[match.index - 1], '`', 'generic type must start inside inline code');
      assert.equal(markdown[match.index + match[0].length], '`', 'generic type must end inside inline code');
    }
  }
  for (const name of ['SHARED_WEAK_AUTHORITY', 'SHARED_WEAK_EVIDENCE',
    'OWNERSHIP_COMPOSITION', 'OWNERSHIP_COMPOSITION_EVIDENCE']) {
    const markdown = await readFile(
      path.join(compilerWorkspaceRoot, `docs/M3_${name}.md`), 'utf8',
    );
    checkGenericSpellings(markdown);
  }
  for (const spelling of ['Shared<T>', 'Weak<T>']) {
    checkGenericSpellings(`one \`${spelling}\` value`);
    assert.throws(() => checkGenericSpellings(`one ${spelling} value`), /start inside inline code/);
    assert.throws(() => checkGenericSpellings(`one \`${spelling} value`), /end inside inline code/);
  }
});

test('planned ownership composition binds existing declarations and located tests without claiming production', async () => {
  const contract = await readFile(path.join(compilerWorkspaceRoot, 'docs/M3_OWNERSHIP_COMPOSITION.md'), 'utf8');
  const evidence = await readFile(path.join(compilerWorkspaceRoot, 'docs/M3_OWNERSHIP_COMPOSITION_EVIDENCE.md'), 'utf8');
  const root = 'crates/zryna-semantics/src/data_ownership_v1/';
  const declarations = [
    ['type_model.rs', 'fn map_node_types('],
    ['owner_state.rs', 'struct OwnerState {'],
    ['owner_state.rs', 'enum OwnerDelta {'],
    ['owned_lowering_resources.rs', "struct OwnedCleanupAccounting<'a> {"],
    ['owned_cfg_state.rs', 'struct OwnedCfgState {'],
  ];
  const references = [...evidence.matchAll(/^\| `([a-z_]+\.rs)` \| `([a-z_0-9]+)` \|/gm)]
    .map(([, file, name]) => [file, name]);
  assert.equal(references.length, 9);
  assert.equal(new Set(references.map(([file, name]) => `${file}:${name}`)).size, 9);
  const files = [...new Set([...declarations.map(([file]) => file), ...references.map(([file]) => `tests/${file}`)])];
  const sources = new Map(await Promise.all(files.map(async file => [file,
    await readFile(path.join(compilerWorkspaceRoot, root, file), 'utf8')])));
  const upgradeRequirements = [
    'producer-facing edge-shape descriptor before a complete CFG exists',
    'The descriptor proves type/signature shape only.',
    'does not require an already-verified upgraded program',
    'Mandatory full IR verification subsequently proves the complete operation',
    'descriptor does not decide which outcome will execute',
    'separate from real target/runtime execution',
  ];
  function check(text, sourceFiles) {
    assert.match(text, /planned implementation contract, not implemented generic source semantics/);
    assert.match(text, /callee's call-scoped borrow access ends on call completion,\non either success or trap/);
    assert.match(text, /caller's original lexical borrow authority is separately retained\nand discharged under lexical rules; the call does not implicitly extend it or permit escape/);
    const sections = [...text.matchAll(/^## C([1-8]): /gm)].map(match => match[1]);
    assert.deepEqual(sections, ['1', '2', '3', '4', '5', '6', '7', '8']);
    const upgrade = text.split('## C8: Upgrade-success edge signature\n')[1]
      ?.split('\n## Integration and closure')[0].replace(/\s+/g, ' ') ?? '';
    for (const requirement of upgradeRequirements) assert(upgrade.includes(requirement), requirement);
    for (const [file, declaration] of declarations) assert(sourceFiles.get(file)?.includes(declaration), declaration);
    for (const [file, name] of references) {
      assert.match(sourceFiles.get(`tests/${file}`) ?? '', new RegExp(`#\\[test\\]\\s*fn ${name}\\(`), name);
    }
  }
  check(contract, sources);
  assert.throws(() => check(contract.replace('on either success or trap', 'only on success'), sources));
  assert.throws(() => check(contract.replace('separately retained', 'implicitly extended'), sources));
  for (const [index, fragment] of ['edge-shape descriptor', 'type/signature shape only',
    'already-verified upgraded', 'Mandatory full IR verification',
    'descriptor does not decide', 'separate from real target/runtime execution'].entries()) {
    const changed = contract.replace(fragment, 'removed requirement');
    assert.notEqual(changed, contract);
    assert.throws(() => check(changed, sources), error => error.message.includes(upgradeRequirements[index]));
  }
  for (let index = 1; index <= 8; index++) {
    assert.throws(() => check(contract.replace(`## C${index}: `, '## Missing: '), sources));
  }
  assert.throws(() => check(contract.replace('## C8: ', '## C7: '), sources));
  for (const [file, declaration] of declarations) {
    const changed = new Map(sources);
    changed.set(file, changed.get(file).replace(declaration, 'removed declaration'));
    assert.throws(() => check(contract, changed));
  }
  for (const [file, name] of references) {
    const changed = new Map(sources);
    changed.set(`tests/${file}`, changed.get(`tests/${file}`).replace(`fn ${name}(`, 'fn missing_test('));
    assert.throws(() => check(contract, changed));
  }
  assert.match(evidence, /Every name in this table is proposed, not an existing test or API/);
  assert.match(evidence, /This candidate includes the #233–#237 extractions and the #288 constructor-order correction/);
  assert.match(evidence, /candidate tree, not the merge status of its prerequisite pull requests/);
  const dispatch = await readFile(path.join(compilerWorkspaceRoot, root, 'function_dispatch.rs'), 'utf8');
  assert.match(dispatch, /fn /);
  assert.match(evidence, /500 lines triggers a split\/cohesion review, not a compulsory limit/);
  assert.match(evidence, /larger cohesive module is allowed with a documented rationale/);
  const upgradeEvidenceRequirements = ['producer-facing type/signature shape before CFG construction',
    'mandatory full IR verification seals', 'separate bounded transition-model proof', 'not target execution'];
  function checkUpgradeEvidence(text) {
    const rows = text.split('\n').filter(line => line.startsWith('| `generic_upgrade_synthetic_success_signature` |'));
    assert.equal(rows.length, 1, 'one exact upgrade evidence row');
    const [row] = rows;
    for (const requirement of upgradeEvidenceRequirements) {
      assert(row.includes(requirement), requirement);
    }
    return row;
  }
  // Matching prose elsewhere must not redirect a mutation away from the checked row.
  for (const text of [evidence, `${upgradeEvidenceRequirements.join('; ')}\n${evidence}`]) {
    const row = checkUpgradeEvidence(text);
    for (const requirement of upgradeEvidenceRequirements) {
      const changedRow = row.replace(requirement, 'removed requirement');
      assert.notEqual(changedRow, row);
      const changed = text.replace(row, changedRow);
      assert.notEqual(changed, text);
      assert.throws(() => checkUpgradeEvidence(changed), error => error.message === requirement);
    }
    assert.throws(() => checkUpgradeEvidence(text.replace(row, '')), /one exact upgrade evidence row/);
    assert.throws(() => checkUpgradeEvidence(`${text}\n${row}`), /one exact upgrade evidence row/);
  }
});

test('beginner setup preserves direct runtime and repository-local editing boundaries', async () => {
  const guide = await readFile(path.join(compilerWorkspaceRoot, 'docs/GETTING_STARTED.md'), 'utf8');
  for (const required of [
    'git clone https://github.com/zryna/zryna.git\ncd zryna',
    'node -p "process.execPath"',
    'the CLI still performs the authoritative file/reparse-point and version',
    'examples/universal/my-add.zry',
    'This is editing guidance, not an additional executed example.',
    'no architecture bypass',
    'internal ownership tests do not enable that CLI profile',
  ]) assert(guide.includes(required), required);
  assert.match(guide, /Rust \*\*1\.97\.1\*\*/);
  assert.match(guide, /pnpm \*\*11\.18\.0\*\*/);
  assert.match(guide, /Node\.js \*\*22\.22\.1\*\*/);
});

test('preview release policy is exported and keeps release-facing wording and boundaries aligned', async () => {
  const statement = [
    'Zryna v0.1.0 is an experimental, source-based Developer Preview. It is not',
    'production-ready. It provides the documented repository-local M1–M3 workflows; it does not',
    'include standalone binaries, package installation, or stable compatibility guarantees.',
  ].join('\n');
  for (const relative of [
    'README.md',
    'docs/DEVELOPER_PREVIEW.md',
    'docs/GETTING_STARTED.md',
    'docs/M3_GETTING_STARTED.md',
    'docs/STATUS.md',
  ]) {
    const text = await readFile(path.join(compilerWorkspaceRoot, relative), 'utf8');
    assert(text.includes(statement), relative);
    assert(text.includes('`v0.1.0` tag and pre-release are not yet published'), relative);
  }
  const policy = await readFile(path.join(compilerWorkspaceRoot, 'docs/DEVELOPER_PREVIEW.md'), 'utf8');
  for (const required of [
    '`--profile control-flow-v1`',
    '`--profile data-ownership-v1`',
    'Linux x86-64 and Windows x64',
    'Node.js | direct regular executable, exact `22.22.1`',
    '`pnpm docs:check`',
    '`pnpm preflight`',
    '`pnpm m0:check`',
    '`pnpm m1:check`',
    '`pnpm m2:check`',
    '`pnpm m3:check`',
    '`refs/tags/v0.1.0`',
    'Never move or recreate `v0.1.0`',
    'private security advisory channel',
  ]) assert(policy.includes(required), required);
  assert.match(policy, /does not publish a release, authorize a tag, or make work planned for M4 and later public/);
});

async function temporaryOutput() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'zryna-docs-test-'));
  return { root, output: path.join(root, 'bundle') };
}

async function bundleFiles(root, relative = '', result = new Map()) {
  const entries = await readdir(path.join(root, relative), { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
  for (const entry of entries) {
    const child = relative ? path.posix.join(relative, entry.name) : entry.name;
    if (entry.isDirectory()) await bundleFiles(root, child, result);
    else result.set(child, await readFile(path.join(root, ...child.split('/'))));
  }
  return result;
}

async function isolatedWorkspace() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'zryna-docs-workspace-'));
  for (const relative of [
    'package.json',
    'schemas/zryna-docs-bundle-v1.schema.json',
    'docs/website-bundle-v1.json',
  ]) {
    await cp(path.join(compilerWorkspaceRoot, relative), path.join(root, relative), { recursive: true });
  }
  const registry = await loadRegistry(compilerWorkspaceRoot);
  for (const document of registry.documents) {
    await cp(path.join(compilerWorkspaceRoot, document.source), path.join(root, document.source), {
      recursive: true,
    });
  }
  return root;
}

test('registry exports the exact implemented and planned publication inventory', async () => {
  const registry = await loadRegistry(compilerWorkspaceRoot);
  assert.deepEqual(
    registry.documents,
    expectedDocuments,
  );
});

test('exports identical canonical bytes for identical compiler input', async (context) => {
  const first = await temporaryOutput();
  const second = await temporaryOutput();
  context.after(() =>
    Promise.all([
      rm(first.root, { recursive: true, force: true }),
      rm(second.root, { recursive: true, force: true }),
    ]),
  );
  const firstResult = await exportDocsBundle({ output: first.output, ...PROVENANCE });
  const secondResult = await exportDocsBundle({ output: second.output, ...PROVENANCE });
  assert.equal(firstResult.manifestSha256, secondResult.manifestSha256);
  const firstFiles = await bundleFiles(first.output);
  const secondFiles = await bundleFiles(second.output);
  assert.deepEqual([...firstFiles.keys()], [...secondFiles.keys()]);
  for (const [name, bytes] of firstFiles) assert.deepEqual(bytes, secondFiles.get(name));
  assert.equal(firstResult.manifest.generatedAt, undefined);
});

test('independent validation binds digest channel commit ref and every document', async (context) => {
  const temporary = await temporaryOutput();
  context.after(() => rm(temporary.root, { recursive: true, force: true }));
  const result = await exportDocsBundle({ output: temporary.output, ...PROVENANCE });
  const validated = await validateDocsBundle(temporary.output, {
    expectedManifestSha256: result.manifestSha256,
    expectedChannel: 'next',
    expectedSourceCommit: COMMIT,
    expectedSourceRef: 'refs/heads/main',
  });
  assert.deepEqual(validated.manifest, result.manifest);
  const paths = validated.manifest.documents.map(({ path: documentPath }) => documentPath);
  assert.deepEqual(paths, [...paths].sort());
});

test('rejects tampering even when the bundled checksum remains unchanged', async (context) => {
  const temporary = await temporaryOutput();
  context.after(() => rm(temporary.root, { recursive: true, force: true }));
  const result = await exportDocsBundle({ output: temporary.output, ...PROVENANCE });
  await writeFile(path.join(temporary.output, 'documents/status/current.md'), '# Forged\n');
  await assert.rejects(
    validateDocsBundle(temporary.output, {
      expectedManifestSha256: result.manifestSha256,
      expectedChannel: 'next',
      expectedSourceCommit: COMMIT,
      expectedSourceRef: 'refs/heads/main',
    }),
    /does not match its manifest record/,
  );
});

test('rejects an unauthenticated digest or mismatched source provenance', async (context) => {
  const temporary = await temporaryOutput();
  context.after(() => rm(temporary.root, { recursive: true, force: true }));
  const result = await exportDocsBundle({ output: temporary.output, ...PROVENANCE });
  const expectations = {
    expectedManifestSha256: result.manifestSha256,
    expectedChannel: 'next',
    expectedSourceCommit: COMMIT,
    expectedSourceRef: 'refs/heads/main',
  };
  await assert.rejects(
    validateDocsBundle(temporary.output, { ...expectations, expectedManifestSha256: '0'.repeat(64) }),
    /authentication failed/,
  );
  await assert.rejects(
    validateDocsBundle(temporary.output, { ...expectations, expectedSourceCommit: '1'.repeat(40) }),
    /provenance does not match/,
  );
});

test('channel and ref rules fail before output publication', async (context) => {
  const temporary = await temporaryOutput();
  context.after(() => rm(temporary.root, { recursive: true, force: true }));
  await assert.rejects(
    exportDocsBundle({ ...PROVENANCE, output: temporary.output, sourceRef: 'refs/heads/feature' }),
    /next channel requires refs\/heads\/main/,
  );
  await assert.rejects(
    exportDocsBundle({ ...PROVENANCE, output: temporary.output, channel: '0.1.0' }),
    /matching source version and immutable/,
  );
  await assert.rejects(readFile(path.join(temporary.output, 'manifest.json')), /ENOENT/);
});

test('an existing destination is preserved create-only', async (context) => {
  const temporary = await temporaryOutput();
  context.after(() => rm(temporary.root, { recursive: true, force: true }));
  await exportDocsBundle({ output: temporary.output, ...PROVENANCE });
  const before = await bundleFiles(temporary.output);
  await assert.rejects(exportDocsBundle({ output: temporary.output, ...PROVENANCE }), /output already exists/);
  const after = await bundleFiles(temporary.output);
  assert.deepEqual([...after.keys()], [...before.keys()]);
  for (const [name, bytes] of before) assert.deepEqual(bytes, after.get(name));
});

test('registry rejects invalid source paths without output', async (context) => {
  const workspace = await isolatedWorkspace();
  const temporary = await temporaryOutput();
  context.after(() =>
    Promise.all([
      rm(workspace, { recursive: true, force: true }),
      rm(temporary.root, { recursive: true, force: true }),
    ]),
  );
  const registryPath = path.join(workspace, 'docs/website-bundle-v1.json');
  const registry = JSON.parse(await readFile(registryPath, 'utf8'));
  registry.documents[1].source = '../escape.md';
  await writeFile(registryPath, `${JSON.stringify(registry, null, 2)}\n`);
  await assert.rejects(
    exportDocsBundle({ workspaceRoot: workspace, output: temporary.output, ...PROVENANCE }),
    /portable Markdown path/,
  );
  await assert.rejects(readFile(path.join(temporary.output, 'manifest.json')), /ENOENT/);
});

test('registry document and aggregate byte budgets fail before output', async (context) => {
  const workspace = await isolatedWorkspace();
  const temporary = await temporaryOutput();
  context.after(() =>
    Promise.all([
      rm(workspace, { recursive: true, force: true }),
      rm(temporary.root, { recursive: true, force: true }),
    ]),
  );
  const registryPath = path.join(workspace, 'docs/website-bundle-v1.json');
  const registry = JSON.parse(await readFile(registryPath, 'utf8'));
  registry.documents = Array.from({ length: 513 }, (_, index) => ({
    id: `status/budget-${String(index).padStart(3, '0')}`,
    source: `docs/budget-${String(index).padStart(3, '0')}.md`,
    path: `documents/status/budget-${String(index).padStart(3, '0')}.md`,
    title: `Budget ${index}`,
  }));
  await writeFile(registryPath, `${JSON.stringify(registry, null, 2)}\n`);
  await assert.rejects(
    exportDocsBundle({ workspaceRoot: workspace, output: temporary.output, ...PROVENANCE }),
    /registry document budget exceeded/,
  );

  const largeDocument = `${'x'.repeat(2 * 1024 * 1024 - 1)}\n`;
  registry.documents = Array.from({ length: 17 }, (_, index) => ({
    id: `status/bytes-${String(index).padStart(2, '0')}`,
    source: `docs/bytes-${String(index).padStart(2, '0')}.md`,
    path: `documents/status/bytes-${String(index).padStart(2, '0')}.md`,
    title: `Bytes ${index}`,
  }));
  await writeFile(registryPath, `${JSON.stringify(registry, null, 2)}\n`);
  await Promise.all(
    registry.documents.map((document) => writeFile(path.join(workspace, document.source), largeDocument)),
  );
  await assert.rejects(
    exportDocsBundle({ workspaceRoot: workspace, output: temporary.output, ...PROVENANCE }),
    /aggregate document byte budget exceeded/,
  );
  await assert.rejects(readFile(path.join(temporary.output, 'manifest.json')), /ENOENT/);
});

test('source symlinks fail closed where the host permits them', async (context) => {
  const workspace = await isolatedWorkspace();
  const temporary = await temporaryOutput();
  context.after(() =>
    Promise.all([
      rm(workspace, { recursive: true, force: true }),
      rm(temporary.root, { recursive: true, force: true }),
    ]),
  );
  const source = path.join(workspace, 'docs/STATUS.md');
  const target = path.join(workspace, 'docs/STATUS-target.md');
  await cp(source, target);
  await rm(source);
  try {
    await symlink(target, source, 'file');
  } catch (error) {
    if (error.code === 'EPERM' || error.code === 'EACCES') {
      context.skip('host forbids test symlinks');
      return;
    }
    throw error;
  }
  await assert.rejects(
    exportDocsBundle({ workspaceRoot: workspace, output: temporary.output, ...PROVENANCE }),
    /is not a regular file/,
  );
});

test('invalid UTF-8 source fails before output publication', async (context) => {
  const workspace = await isolatedWorkspace();
  const temporary = await temporaryOutput();
  context.after(() =>
    Promise.all([
      rm(workspace, { recursive: true, force: true }),
      rm(temporary.root, { recursive: true, force: true }),
    ]),
  );
  await writeFile(path.join(workspace, 'docs/STATUS.md'), Buffer.from([0xc3, 0x28]));
  await assert.rejects(
    exportDocsBundle({ workspaceRoot: workspace, output: temporary.output, ...PROVENANCE }),
    /encoded data was not valid/,
  );
  await assert.rejects(readFile(path.join(temporary.output, 'manifest.json')), /ENOENT/);
});
