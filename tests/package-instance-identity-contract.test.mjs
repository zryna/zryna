import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const contract = await readFile(
  path.join(root, 'spec/package/PACKAGE_INSTANCE_IDENTITY_V1.md'), 'utf8',
);
const packageSchema = JSON.parse(await readFile(
  path.join(root, 'schemas/zryna-package-release-v1.schema.json'), 'utf8',
));
const packageRelease = await readFile(path.join(root, 'spec/package/PACKAGE_RELEASE_V1.md'), 'utf8');
const crossTargetProfiles = await readFile(
  path.join(root, 'spec/language/CROSS_TARGET_PROFILES_V1.md'), 'utf8',
);

test('package instance identity keeps owning contracts and identity layers distinct', async () => {
  for (const requirement of [
    'Contract identity: `zryna.package-instance-identity.v1`',
    '[#168](https://github.com/zryna/zryna/issues/168) is the sole authority',
    '[#357](https://github.com/zryna/zryna/issues/357) is the sole authority',
    '(id, sourceSha256)',
    '(package instance, normalized package-relative module path)',
    '(semantic domain, module identity, declaration ordinal)',
    '`javascript`, `webassembly`, and `native` are not part of nominal identity',
    'graph role, not compiler-host location',
  ]) {
    assert.match(contract, new RegExp(requirement.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }

  const ownership = await readFile(path.join(root, 'spec/language/DATA_OWNERSHIP_V1.md'), 'utf8');
  assert.match(ownership, /nominal declaration identity is the sealed pair `\(ModuleId, declaration-index\)`/);
  assert.match(contract, /declaration ordinal is the existing zero-based source order/);
  assert.match(contract, /dense package, module, or type IDs[\s\S]*private indexes/);
});

test('package instance terms map exactly to merged package-release v1 fields and ownership', () => {
  assert.deepEqual(packageSchema.$defs.package.required, ['id', 'sourceSha256', 'dependencies']);
  assert.deepEqual(packageSchema.$defs.edge.required, ['alias', 'package']);
  assert.deepEqual(packageSchema.$defs.source.required, ['kind', 'locator', 'revision']);
  assert.deepEqual(packageSchema.$defs.compatibility.required, ['compiler', 'profile', 'targets']);
  assert.deepEqual(packageSchema.$defs.dependency.required, ['alias', 'name', 'version', 'source']);
  assert.equal(packageSchema.$defs.manifest.additionalProperties, false);
  assert.equal(Object.hasOwn(packageSchema.$defs.manifest.properties, 'features'), false);
  assert.match(packageRelease, /The package ID is the manifest digest\. `sourceSha256` is the `source-files` digest/);
  assert.match(packageRelease, /128 duplicate SBOM edges pass only the isolated array shape/);
  assert.match(contract, /there is no `packageInstance` or `sourceIdentity` wire field/);
  assert.match(contract, /lock edge's `package` is the selected package record's `id`/i);
  assert.match(contract, /Canonical bytes, digest domains,[\s\S]*remain wholly #168-owned/);
});

test('aliases, duplicate sources and versions have exact nominal compatibility decisions', () => {
  for (const statement of [
    'A dependency alias belongs only to its importing package edge.',
    'They share one module and nominal universe in the target graph.',
    'the same package name at different versions',
    'the same name and version from different Git locators or commits',
    'There is no structural fallback, semver type equivalence, source substitution',
    'An unknown feature field rejects under #168 even when its requested set is empty',
    'implicit feature unification and every feature request reject now',
  ]) {
    assert(contract.includes(statement), statement);
  }

  const examples = tableRows('Representative resolution and type examples');
  for (const id of [
    'alias-same-instance', 'diamond', 'version-coexistence', 'duplicate-name-source',
    'ambiguous-coordinate',
  ]) {
    assert(examples.has(id), id);
  }
  assert.match(examples.get('version-coexistence'), /incompatible-nominal-type before IR/);
  assert.match(examples.get('duplicate-name-source'), /nominally incompatible/);
  assert.match(examples.get('ambiguous-coordinate'), /during resolution/);
});

test('module visibility is fail closed without adding package or type-import syntax', async () => {
  for (const phrase of [
    'Modules and declarations are private by default at a package boundary.',
    'only to an explicitly exported module entry',
    'The referenced declaration must also carry the existing explicit source `export`.',
    'This contract assigns no new source spelling.',
    'The same applies to nominal type imports.',
    'existing M3 behavior remains unchanged',
  ]) {
    assert(contract.includes(phrase), phrase);
  }
  assert.match(contract, /Package-module import\s+syntax and its provider-neutral DTO require a separate approved/);
  assert.match(contract, /reviewed #168 schema extension that authenticates an explicitly owned\s+module-visibility document/);
  assert.match(contract, /detached language document or local convention cannot add an\s+export/);

  const modules = await readFile(path.join(root, 'docs/M2_MODULE_CLOSURE.md'), 'utf8');
  assert.match(modules, /There is no[\s\S]*package or bare resolution/);
  assert.match(modules, /Reject cycles and revalidate all retained directory and file identities/);

  const examples = tableRows('Representative resolution and type examples');
  for (const id of ['private-module', 'private-declaration', 'root-escape', 'future-type-import']) {
    assert(examples.has(id), id);
  }
});

test('host and target graphs cannot leak identity, imports or capability authority', async () => {
  const graphRows = tableRows('Graph roles, cycles, and compatibility');
  assert.match(graphRows.get('target/runtime'), /build-tool execution or host permissions/);
  assert.match(graphRows.get('host/build'), /target imports, target nominal equality, runtime capabilities/);
  assert.match(contract, /An edge has exactly one role\./);
  assert.match(contract, /source code\s+cannot import a host\/build dependency/);
  assert.match(contract, /appears once in each role-scoped closure and is validated independently/);
  assert.match(contract, /Package dependency graphs are acyclic in v1/);
  assert.match(contract, /repeated traversal through a diamond is not an error/);

  const capability = await readFile(path.join(root, 'spec/wit/CAPABILITY_PROFILES_V1.md'), 'utf8');
  assert.match(capability, /A profile is an upper bound, not an ambient grant/);
  assert.match(contract, /source digest authenticates bytes but grants no trust, execution, capability/i);

  const examples = tableRows('Representative resolution and type examples');
  for (const id of [
    'package-cycle', 'module-cycle', 'incompatible-transitive-profile', 'host-is-not-target',
    'host-import-leak',
  ]) {
    assert(examples.has(id), id);
  }
  assert.match(examples.get('incompatible-transitive-profile'), /under #357 before IR/);
});

test('source-only resolver outline is bounded, offline and independent of registry or native work', () => {
  const outline = section('Bounded source-only resolver outline');
  for (const step of [
    'Validate canonical #168 manifest, lock, checksum, and material records',
    "Build an exact catalog keyed by #168's manifest dependency selection tuple",
    'iterative work queue',
    'invoke #357 composition',
    'retained package source-root capability',
    'Sort complete module identities',
    'Revalidate cached inputs',
  ]) {
    assert(outline.includes(step), step);
  }
  assert.match(outline, /needs no registry service, native library, M7 FFI/);
  assert.match(contract, /Frozen\/offline resolution[\s\S]*cannot fetch, follow a ref, update, repair/);

  const limits = tableRows('Bounded source-only resolver outline');
  assert.match(limits.get('entire canonical package-release fixture, including LF'), /65,536 bytes/);
  assert.match(limits.get('manifests \/ lock packages \/ SBOM packages'), /16 each/);
  assert.match(limits.get('manifest dependencies \/ lock-package dependencies'), /8 per package/);
  assert.match(limits.get('SBOM edges'), /128[\s\S]*structural array bound only/);
  assert.match(limits.get('manifest files'), /16 per manifest/);
  assert.equal(packageSchema.properties.manifests.maxItems, 16);
  assert.equal(packageSchema.$defs.manifest.properties.dependencies.maxItems, 8);
  assert.equal(packageSchema.$defs.package.properties.dependencies.maxItems, 8);
  assert.equal(packageSchema.$defs.sbom.properties.edges.maxItems, 128);
  assert.equal(packageSchema.$defs.manifest.properties.files.maxItems, 16);
  assert.match(outline, /#360 introduces no competing lower package-fixture limit/);
  assert.match(outline, /consume #168's validated values rather than fork or\s+reimplement/);
});

test('cross-profile acceptance maps to merged issue 357 vocabulary and bounds', () => {
  assert.match(crossTargetProfiles, /Contract identity: `zryna\.cross-target-profiles\.v1`/);
  assert.match(contract, /Cross-profile acceptance follows #357/);
  for (const rule of [
    /source package (?:can|may) claim both only (?:if|when) separately verified\s+under each/i,
    /precompiled authority cannot be relabeled/,
    /Unknown compatibility rejects/i,
  ]) {
    assert.match(crossTargetProfiles, rule);
    assert.match(contract, rule);
  }
  assert.match(crossTargetProfiles, /at most 256 instances including root, 4,096 edges,[\s\S]*32 edges in a root-to-leaf path,[\s\S]*65,536 total UTF-8 identity bytes/);
  assert.match(contract, /#357's future limits of 256 instances including root, 4,096 edges, 32 edges in a\s+root-to-leaf path, and 65,536 total UTF-8 identity bytes/);
  assert.match(contract, /authenticated, finite, canonical graph with stable\s+instance IDs, exact source identities, and dependency edges/);
});

test('representative matrix covers every issue acceptance input with a rejecting phase', () => {
  const rows = tableRows('Representative resolution and type examples');
  assert.equal(rows.size, 16);
  for (const id of [
    'alias-same-instance', 'duplicate-name-source', 'version-coexistence', 'package-cycle',
    'incompatible-transitive-profile', 'exact-git-offline', 'missing-git-offline',
  ]) {
    assert(rows.has(id), id);
  }
  for (const id of [
    'version-coexistence', 'ambiguous-coordinate', 'private-module', 'private-declaration',
    'root-escape', 'package-cycle', 'module-cycle', 'incompatible-transitive-profile',
    'host-import-leak', 'missing-git-offline', 'future-type-import',
  ]) {
    assert.match(
      rows.get(id),
      /reject|incompatible|unsupported|ambiguous|private|path\/containment|cycle|graph-role-leak/i,
      `${id} rejection`,
    );
  }
});

function section(title) {
  const start = contract.indexOf(`## ${title}`);
  assert.notEqual(start, -1, title);
  const end = contract.indexOf('\n## ', start + 4);
  return contract.slice(start, end === -1 ? undefined : end);
}

function tableRows(title) {
  const document = section(title);
  const rows = new Map();
  for (const line of document.split('\n')) {
    if (!line.startsWith('| ') || line.includes('---')) continue;
    const cells = line.slice(2, -2).split(' | ');
    if (['Case', 'Graph role', '#168 quantity'].includes(cells[0])) continue;
    rows.set(cells[0].replaceAll('`', ''), line);
  }
  return rows;
}
