import { createHash } from 'node:crypto';

// Test construction is independent of the validator and its serializer.
export function wire(value) {
  const sort = item => Array.isArray(item) ? item.map(sort) :
    item !== null && typeof item === 'object' ?
      Object.fromEntries(Object.entries(item).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)
        .map(([key, child]) => [key, sort(child)])) : item;
  return Buffer.from(JSON.stringify(sort(value)) + '\n');
}
export const hash = content => createHash('sha256').update(content).digest('hex');
export const recordHash = (kind, value) => hash(Buffer.concat([
  Buffer.from('ZRYNA-PACKAGE-RELEASE-V1\0' + kind + '\0'), wire(value),
]));
export const compatibility = () => ({
  compiler: '0.1.0', profile: 'i32-v1', targets: ['javascript', 'webassembly'],
});
export function material(content) {
  return { sha256: hash(content), content };
}
export function checksum(path, content) {
  return { path, size: Buffer.byteLength(content), sha256: hash(content) };
}
export function subject(release) {
  const { notes, rollback, ...value } = release;
  return recordHash('release-subject', value);
}
export function bindRelease(fixture) {
  const release = fixture.release;
  release.lockSha256 = recordHash('lock', fixture.lock);
  release.provenance.recipeSha256 = recordHash('recipe', release.reproduction.recipe);
  release.provenance.environmentSha256 = recordHash('environment', release.reproduction.environment);
  release.provenance.materials = fixture.materials.map(item => item.sha256);
  release.notes.subjectSha256 = subject(release);
  for (const signature of release.notes.signatures) {
    signature.payloadSha256 = recordHash('release-note', {
      subjectSha256: release.notes.subjectSha256, text: release.notes.text,
    });
  }
  release.rollback.from = release.notes.subjectSha256;
  return fixture;
}
export function bindGraph(fixture, rootName) {
  fixture.manifests.sort((a, b) =>
    recordHash('manifest', a).localeCompare(recordHash('manifest', b), 'en'));
  const packages = fixture.manifests.map(manifest => ({
    id: recordHash('manifest', manifest),
    sourceSha256: recordHash('source-files', manifest.files),
    dependencies: manifest.dependencies.map(dependency => {
      const target = fixture.manifests.find(item => item.name === dependency.name &&
        item.version === dependency.version && wire(item.source).equals(wire(dependency.source)));
      return { alias: dependency.alias, package: recordHash('manifest', target) };
    }),
  }));
  fixture.lock.packages = packages;
  fixture.lock.root = recordHash('manifest', fixture.manifests.find(item => item.name === rootName));
  fixture.release.sbom.packages = packages.map(pkg => ({
    package: pkg.id, sourceSha256: pkg.sourceSha256, license: 'NOASSERTION',
  }));
  fixture.release.sbom.edges = packages.flatMap(pkg =>
    pkg.dependencies.map(edge => ({ from: pkg.id, alias: edge.alias, to: edge.package })));
  return bindRelease(fixture);
}
export function makeFixture(count = 2) {
  const sourceText = 'synthetic source fixture\n';
  const outputText = 'synthetic artifact fixture\n';
  const toolText = 'synthetic pinned tool fixture\n';
  const manifests = Array.from({ length: count }, (_, index) => ({
    format: 'zryna.package.v1', name: `package-${String(index).padStart(2, '0')}`,
    version: '0.1.0',
    source: index === 0 ?
      { kind: 'git', locator: 'https://example.invalid/sample.git', revision: '1'.repeat(40) } :
      { kind: 'local', locator: `packages/p${index}`, revision: '' },
    compatibility: compatibility(), files: [checksum('src/main.zry', sourceText)], dependencies: [],
  }));
  for (let index = 1; index < count; index++) {
    manifests[index].dependencies = manifests.slice(Math.max(0, index - 8), index).map(item => ({
      alias: item.name, name: item.name, version: item.version, source: { ...item.source },
    }));
  }
  const artifacts = [checksum('artifacts/sample.txt', outputText)];
  const fixture = {
    format: 'zryna.package-release.fixture.v1', manifests,
    lock: { format: 'zryna.lock.v1', root: '', compatibility: compatibility(), packages: [] },
    materials: [sourceText, outputText, toolText].map(material).sort((a, b) =>
      a.sha256.localeCompare(b.sha256, 'en')),
    release: {
      format: 'zryna.release.v1', tag: 'v0.1.0', commit: '2'.repeat(40), lockSha256: '',
      checksums: artifacts, sbom: { format: 'zryna.sbom.v1', packages: [], edges: [] },
      provenance: {
        format: 'zryna.provenance.v1', builder: 'fixture-builder', recipeSha256: '',
        environmentSha256: '', materials: [], sourceDateEpoch: 0,
      },
      reproduction: {
        environment: {
          os: 'linux-x86_64', compilerSha256: hash(toolText), compilerVersion: '0.1.0',
          toolchains: [{ name: 'fixture-compiler', version: '0.1.0', sha256: hash(toolText) }],
          variables: [{ name: 'LC_ALL', value: 'C' }, { name: 'SOURCE_DATE_EPOCH', value: '0' },
            { name: 'TZ', value: 'UTC' }],
          network: 'disabled', workspace: 'empty', cache: 'empty',
        },
        recipe: { executable: 'fixture-compiler', arguments: ['architecture', 'check'] },
        expected: structuredClone(artifacts), observed: structuredClone(artifacts),
        verification: 'comparison-only',
      },
      notes: {
        text: 'Synthetic contract fixture; no release or signature is verified.',
        subjectSha256: '', signatures: [{
          keyId: '3'.repeat(64), algorithm: 'ed25519', signature: '0'.repeat(128),
          status: 'unverified-fixture', payloadSha256: '',
        }],
      },
      rollback: {
        from: '', to: '4'.repeat(64), reason: 'Synthetic rollback proposal',
        authorizationSha256: '5'.repeat(64), mode: 'proposal-only',
      },
    },
  };
  return bindGraph(fixture, `package-${String(count - 1).padStart(2, '0')}`);
}
