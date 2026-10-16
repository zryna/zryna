// Synthetic schema carriers only. Never compiler/browser/hosted/publication evidence.
import { fixture as legacy, encode, hash, retained } from './fixture.mjs';
import { sourceRegistry } from '../../scripts/m6/source-registry.mjs';
import { sourceRegistrySha256 } from '../../scripts/m6/source-evidence.mjs';

export function sourceFixture() {
  const { record, policy } = legacy();
  const rename = role => ({ 'playground-toolkit': 'source-composition',
    'bubblewrap-linux': 'daemon-supervisor-linux', 'python-linux': 'docker-engine-linux' }[role] ?? role);
  for (const value of record.artifacts) {
    value.role = rename(value.role);
    value.path = `artifacts/${value.role}.bin`;
    if (value.role === 'daemon-supervisor-linux') value.version = '1.0.0';
    if (value.role === 'docker-engine-linux') value.version = '28.4.0';
  }
  for (const role of sourceRegistry.requiredArtifacts) {
    if (record.artifacts.some(value => value.role === role)) continue;
    record.artifacts.push({ role, path: `artifacts/${role}.bin`, version: '1.0.0', bytes: 1,
      sha256: hash(`artifact:${role}`), source: structuredClone(record.artifacts[0].source),
      url: retained(`${role}.bin`) });
  }
  record.format = 'zryna.m6-source-evidence.v2'; record.version = 2;
  record.registrySha256 = sourceRegistrySha256;
  for (const gate of record.gates) {
    const family = sourceRegistry.gates.find(value => value.platforms.some(p => `${value.id}-${p}` === gate.id));
    gate.artifactRoles = family.artifactRoles.map(role => role.replace('-platform', `-${gate.environment.platform}`));
    gate.cases = family.cases.map(id => ({ id, passed: 1, failed: 0, skipped: 0 }));
  }
  const selected = role => record.artifacts.find(value => value.role === role).sha256;
  record.publication = { status: 'published-and-verified', compilerArchiveSha256: selected('playground-compiler'),
    compilerBinarySha256: hash('synthetic compiler binary'), compositionSha256: selected('source-composition'),
    docsManifestSha256: selected('docs-bundle'),
    verificationReceipt: structuredClone(record.gates.find(value => value.id === 'publication-linux').receipt) };
  policy.version = 2; policy.artifactPins = structuredClone(record.artifacts);
  policy.recordSha256 = hash(encode(record));
  return { record, policy };
}
