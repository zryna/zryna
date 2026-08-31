import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

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
    [
      { id: 'reference/aggregate-layout-v1', source: 'spec/memory-model/AGGREGATE_LAYOUT_V1.md', path: 'documents/reference/aggregate-layout-v1.md', title: 'Aggregate layout v1' },
      { id: 'reference/architecture', source: 'docs/ARCHITECTURE.md', path: 'documents/reference/architecture.md', title: 'Compiler architecture' },
      { id: 'reference/cli', source: 'docs/CLI.md', path: 'documents/reference/cli.md', title: 'CLI reference' },
      { id: 'reference/control-flow-modules-v1', source: 'spec/language/CONTROL_FLOW_MODULES_V1.md', path: 'documents/reference/control-flow-modules-v1.md', title: 'Scalar control flow and modules v1' },
      { id: 'reference/data-ownership-v1', source: 'spec/language/DATA_OWNERSHIP_V1.md', path: 'documents/reference/data-ownership-v1.md', title: 'Data and ownership v1' },
      { id: 'reference/documentation-bundles', source: 'docs/DOCUMENTATION_BUNDLES.md', path: 'documents/reference/documentation-bundles.md', title: 'Compiler documentation bundles' },
      { id: 'reference/frontends', source: 'docs/FRONTENDS.md', path: 'documents/reference/frontends.md', title: 'Frontend providers' },
      { id: 'reference/language-overview', source: 'spec/language/OVERVIEW.md', path: 'documents/reference/language-overview.md', title: 'Language overview' },
      { id: 'reference/m2-conformance', source: 'docs/M2_CONFORMANCE.md', path: 'documents/reference/m2-conformance.md', title: 'M2 three-target conformance' },
      { id: 'reference/m2-control-flow-semantics', source: 'docs/M2_CONTROL_FLOW_SEMANTICS.md', path: 'documents/reference/m2-control-flow-semantics.md', title: 'M2 control-flow semantics' },
      { id: 'reference/m2-javascript-backend', source: 'docs/M2_JAVASCRIPT_BACKEND.md', path: 'documents/reference/m2-javascript-backend.md', title: 'M2 deterministic JavaScript backend' },
      { id: 'reference/m2-manifest-v2', source: 'docs/M2_MANIFEST_V2.md', path: 'documents/reference/m2-manifest-v2.md', title: 'M2 manifest and atomic bundles' },
      { id: 'reference/m2-module-closure', source: 'docs/M2_MODULE_CLOSURE.md', path: 'documents/reference/m2-module-closure.md', title: 'M2 deterministic module closure' },
      { id: 'reference/m2-native-backend', source: 'docs/M2_NATIVE_BACKEND.md', path: 'documents/reference/m2-native-backend.md', title: 'M2 Linux x86-64 native backend' },
      { id: 'reference/m2-native-mir', source: 'docs/M2_NATIVE_MIR.md', path: 'documents/reference/m2-native-mir.md', title: 'M2 verified native MIR' },
      { id: 'reference/m2-straight-line-semantics', source: 'docs/M2_STRAIGHT_LINE_SEMANTICS.md', path: 'documents/reference/m2-straight-line-semantics.md', title: 'M2 straight-line semantics' },
      { id: 'reference/m2-webassembly-backend', source: 'docs/M2_WEBASSEMBLY_BACKEND.md', path: 'documents/reference/m2-webassembly-backend.md', title: 'M2 direct core WebAssembly backend' },
      { id: 'reference/memory-model', source: 'spec/memory-model/OVERVIEW.md', path: 'documents/reference/memory-model.md', title: 'Memory model direction' },
      { id: 'reference/ownership-runtime-v1', source: 'spec/abi/OWNERSHIP_RUNTIME_V1.md', path: 'documents/reference/ownership-runtime-v1.md', title: 'Ownership runtime ABI v1' },
      { id: 'reference/scalar-abi-v1', source: 'spec/abi/SCALAR_V1.md', path: 'documents/reference/scalar-abi-v1.md', title: 'Scalar ABI v1' },
      { id: 'reference/syntax-protocol-v4', source: 'docs/SYNTAX_PROTOCOL_V4.md', path: 'documents/reference/syntax-protocol-v4.md', title: 'Syntax protocol v4' },
      { id: 'status/current', source: 'docs/STATUS.md', path: 'documents/status/current.md', title: 'Compiler status' },
      { id: 'status/m0-conformance', source: 'docs/M0_CONFORMANCE.md', path: 'documents/status/m0-conformance.md', title: 'M0 architecture conformance' },
      { id: 'status/m1-conformance', source: 'docs/M1_CONFORMANCE.md', path: 'documents/status/m1-conformance.md', title: 'M1 three-target conformance' },
      { id: 'status/roadmap', source: 'docs/ROADMAP.md', path: 'documents/status/roadmap.md', title: 'Roadmap' },
    ],
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
