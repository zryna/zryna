import { readFileSync } from 'node:fs';
import Ajv from 'ajv';
import { bytes, digest, parseCanonical, reject, sha256 } from './canonical.mjs';

const schema = JSON.parse(readFileSync(
  new URL('../../schemas/zryna-package-release-v1.schema.json', import.meta.url), 'utf8',
));
const validateSchema = new Ajv({ allErrors: false, strict: true }).compile(schema);

function require(value, code, detail) {
  if (!value) reject(code, detail);
}

function equal(left, right, code, detail) {
  require(bytes(left).equals(bytes(right)), code, detail);
}

function ordered(items, key, label) {
  const keys = items.map(key);
  require(keys.every((value, index) => index === 0 || keys[index - 1] < value),
    'P168-ORDER', `${label} must be sorted and unique`);
}

function path(value) {
  require(value.split('/').every(segment =>
    segment !== '' && segment !== '.' && segment !== '..' &&
    !segment.endsWith('.') &&
    !/^(con|prn|aux|nul|com[0-9]|lpt[0-9])($|\.)/.test(segment)),
  'P168-PATH', 'unsafe portable path');
}

function source(value) {
  if (value.kind === 'local') {
    require(/^[a-z0-9][a-z0-9./_-]*$/.test(value.locator) && value.revision === '',
      'P168-SOURCE', 'local source must use a relative path and empty revision');
    path(value.locator);
  } else {
    require(/^https:\/\/[a-z0-9]+(?:[.-][a-z0-9]+)*(?:\/[a-z0-9_-]+)+\.git$/.test(value.locator) &&
      /^[a-f0-9]{40}$/.test(value.revision),
    'P168-SOURCE', 'Git source requires canonical HTTPS locator and exact commit');
  }
}

function checksums(items, materials, label) {
  ordered(items, item => item.path, label);
  const paths = new Set(items.map(item => item.path));
  for (const item of items) {
    path(item.path);
    const parts = item.path.split('/');
    for (let end = 1; end < parts.length; end++) {
      require(!paths.has(parts.slice(0, end).join('/')), 'P168-PATH', 'file/directory collision');
    }
    const content = materials.get(item.sha256);
    require(content !== undefined && Buffer.byteLength(content) === item.size,
      'P168-INTEGRITY', `${label} content missing or size differs`);
  }
}

function graph(manifests, lock) {
  ordered(manifests, item => digest('manifest', item), 'manifests');
  ordered(lock.packages, item => item.id, 'lock packages');
  ordered(lock.compatibility.targets, item => item, 'lock targets');
  const byId = new Map(manifests.map(item => [digest('manifest', item), item]));
  equal(lock.packages.map(item => item.id), [...byId.keys()], 'P168-LOCK', 'manifest/lock set differs');
  require(byId.has(lock.root), 'P168-LOCK', 'unknown root');
  const identities = new Set();
  const adjacency = new Map();
  for (const pkg of lock.packages) {
    const manifest = byId.get(pkg.id);
    const identity = digest('selection', {
      name: manifest.name, version: manifest.version, source: manifest.source,
    });
    require(!identities.has(identity), 'P168-RESOLUTION', 'ambiguous exact selection');
    identities.add(identity);
    source(manifest.source);
    ordered(manifest.compatibility.targets, item => item, 'manifest targets');
    require(manifest.compatibility.compiler === lock.compatibility.compiler &&
      manifest.compatibility.profile === lock.compatibility.profile &&
      lock.compatibility.targets.every(target => manifest.compatibility.targets.includes(target)),
    'P168-COMPATIBILITY', 'exact compiler/profile or target coverage mismatch');
    equal(pkg.sourceSha256, digest('source-files', manifest.files),
      'P168-INTEGRITY', 'source inventory digest mismatch');
    ordered(manifest.dependencies, item => item.alias, 'dependencies');
    ordered(pkg.dependencies, item => item.alias, 'lock edges');
    const expected = manifest.dependencies.map(dependency => {
      source(dependency.source);
      const matches = [...byId].filter(([, candidate]) =>
        candidate.name === dependency.name && candidate.version === dependency.version &&
        bytes(candidate.source).equals(bytes(dependency.source)));
      require(matches.length === 1, 'P168-RESOLUTION', 'exact dependency absent or ambiguous');
      return { alias: dependency.alias, package: matches[0][0] };
    });
    equal(pkg.dependencies, expected, 'P168-LOCK', 'lock edge differs from exact declaration');
    adjacency.set(pkg.id, expected.map(edge => edge.package));
  }
  const active = new Set();
  const visited = new Set();
  function visit(id) {
    require(!active.has(id), 'P168-GRAPH', 'dependency cycle');
    if (visited.has(id)) return;
    active.add(id);
    for (const child of adjacency.get(id)) visit(child);
    active.delete(id);
    visited.add(id);
  }
  visit(lock.root);
  require(visited.size === manifests.length, 'P168-GRAPH', 'unreachable package');
  return byId;
}

export function releaseSubject(release) {
  const { notes, rollback, ...subject } = release;
  return digest('release-subject', subject);
}

export function validateFixture(input) {
  const fixture = parseCanonical(input);
  require(validateSchema(fixture), 'P168-SCHEMA', 'closed record schema rejected input');
  const { manifests, lock, release, materials } = fixture;
  ordered(materials, item => item.sha256, 'materials');
  const materialMap = new Map();
  for (const material of materials) {
    require(sha256(Buffer.from(material.content)) === material.sha256,
      'P168-INTEGRITY', 'material checksum mismatch');
    materialMap.set(material.sha256, material.content);
  }
  for (const manifest of manifests) checksums(manifest.files, materialMap, 'source files');
  const byId = graph(manifests, lock);
  require(release.tag === `v${byId.get(lock.root).version}`,
    'P168-RELEASE', 'release tag differs from root version');
  require(release.lockSha256 === digest('lock', lock), 'P168-INTEGRITY', 'lock digest mismatch');
  checksums(release.checksums, materialMap, 'artifacts');

  const expectedPackages = lock.packages.map(pkg => ({
    package: pkg.id, sourceSha256: pkg.sourceSha256,
  }));
  const sbomPackages = release.sbom.packages.map(({ license, ...pkg }) => pkg);
  equal(sbomPackages, expectedPackages, 'P168-SBOM', 'SBOM package set or source identity differs');
  const expectedEdges = lock.packages.flatMap(pkg => pkg.dependencies.map(edge => ({
    from: pkg.id, alias: edge.alias, to: edge.package,
  })));
  equal(release.sbom.edges, expectedEdges, 'P168-SBOM', 'SBOM edge set differs');

  const reproduction = release.reproduction;
  const environment = reproduction.environment;
  ordered(environment.toolchains, item => item.name, 'toolchains');
  ordered(environment.variables, item => item.name, 'environment variables');
  require(environment.compilerVersion === lock.compatibility.compiler &&
    environment.toolchains.some(tool => tool.sha256 === environment.compilerSha256 &&
      tool.version === environment.compilerVersion),
  'P168-COMPATIBILITY', 'compiler version/digest is not bound to the pinned tool inventory');
  require(environment.toolchains.some(tool => tool.name === reproduction.recipe.executable),
    'P168-REPRODUCTION', 'recipe executable is not a pinned tool');
  const provenance = release.provenance;
  for (const [name, value] of [
    ['LC_ALL', 'C'], ['SOURCE_DATE_EPOCH', String(provenance.sourceDateEpoch)], ['TZ', 'UTC'],
  ]) {
    require(environment.variables.some(variable => variable.name === name && variable.value === value),
      'P168-REPRODUCTION', 'canonical locale, timezone or source epoch is missing');
  }
  require(provenance.recipeSha256 === digest('recipe', reproduction.recipe) &&
    provenance.environmentSha256 === digest('environment', environment),
  'P168-INTEGRITY', 'recipe/environment digest mismatch');
  // Inventory completeness is checked against supplied bytes, not a producer claim.
  equal(provenance.materials, materials.map(item => item.sha256),
    'P168-PROVENANCE', 'provenance material inventory differs');
  for (const id of [environment.compilerSha256, ...environment.toolchains.map(tool => tool.sha256)]) {
    require(materialMap.has(id), 'P168-REPRODUCTION', 'pinned tool material missing');
  }
  equal(reproduction.expected, release.checksums, 'P168-REPRODUCTION', 'expected artifacts differ');
  equal(reproduction.observed, release.checksums, 'P168-REPRODUCTION', 'observed artifacts differ');
  require(!lock.compatibility.targets.includes('native-linux-x86_64') ||
    environment.os === 'linux-x86_64', 'P168-COMPATIBILITY', 'native target requires Linux environment');

  const subject = releaseSubject(release);
  require(release.notes.subjectSha256 === subject,
    'P168-INTEGRITY', 'release-note subject mismatch');
  ordered(release.notes.signatures, item => item.keyId, 'signature key IDs');
  const payload = digest('release-note', {
    subjectSha256: subject, text: release.notes.text,
  });
  require(release.notes.signatures.every(signature => signature.payloadSha256 === payload),
    'P168-INTEGRITY', 'release-note payload mismatch');
  require(release.rollback.from === subject && release.rollback.to !== subject,
    'P168-ROLLBACK', 'rollback must leave this release for a distinct prior subject');
  return Object.freeze({
    status: 'contract-fixture-valid',
    lockSha256: release.lockSha256,
    subjectSha256: subject,
    signatures: 'not-verified',
    reproduction: 'not-executed',
    release: 'not-activated',
  });
}
