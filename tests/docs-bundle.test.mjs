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
