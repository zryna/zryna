import { dirname } from 'node:path';
import { bytes, requireValue, sha256 } from '../../distribution/canonical.mjs';
import { compilerMetadataSelection, COMPILER_TARGET } from './material-graph.mjs';
import { identity, REGISTRY } from './material-lock.mjs';

const FOOTERS = new Set([
  '(*) - dependencies omitted (already shown)',
  '(*) - dependencies already shown elsewhere in the graph (use --no-dedupe to repeat)',
]);

// Compares package identities only. This is not an assertion about resolver2 compilation units
// or the precise features actually executed by rustc. The builder retains the real fixed command.
export function compilerTreeSelection(stdout, metadata, sourceRoot) {
  requireValue(Buffer.isBuffer(stdout) && stdout.length > 0 && stdout.length <= 2 * 1024 * 1024,
    'compiler tree byte budget');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(stdout);
  requireValue(text.endsWith('\n') && !/[\r\x00]/.test(text), 'compiler tree stdout framing');
  const lines = text.slice(0, -1).split('\n');
  requireValue(lines.length <= 32768, 'compiler tree line budget');
  const { selected, packages } = compilerMetadataSelection(metadata, sourceRoot);
  const expected = new Map();
  for (const id of selected) {
    const pkg = packages.get(id);
    requireValue(!expected.has(identity(pkg)) && (pkg.source === null || pkg.source === REGISTRY),
      'compiler tree metadata identity ambiguity');
    expected.set(identity(pkg), pkg);
  }
  const seen = new Set();
  let first = true;
  let footer = false;
  for (const line of lines) {
    requireValue(line.length <= 1024, 'compiler tree line byte budget');
    if (line === '') continue;
    if (FOOTERS.has(line)) {
      requireValue(!footer && seen.size > 0, 'compiler tree duplicate footer');
      footer = true;
      continue;
    }
    requireValue(!footer, 'compiler tree trailing content');
    if (line === '[build-dependencies]') continue;
    const match = /^([A-Za-z0-9_-]+) v([0-9][A-Za-z0-9+._-]*)(.*)$/.exec(line);
    requireValue(match, 'compiler tree package line');
    const key = `${match[1]}\0${match[2]}`;
    const pkg = expected.get(key);
    requireValue(pkg, 'compiler tree unknown or metadata-extra package');
    if (first) {
      requireValue(pkg.name === 'zryna-playground-compiler' && pkg.version === '0.1.0'
        && pkg.source === null, 'compiler tree exact root package');
      first = false;
    }
    const deduped = match[3].endsWith(' (*)');
    const suffix = deduped ? match[3].slice(0, -4) : match[3];
    const location = pkg.source === null ? ` (${dirname(pkg.manifest_path)})` : '';
    const variants = [location];
    if (pkg.targets?.some(target => target.kind?.includes('proc-macro'))) {
      variants.push(` (proc-macro)${location}`, `${location} (proc-macro)`);
    }
    requireValue(variants.includes(suffix), 'compiler tree closed package annotation');
    seen.add(key);
  }
  requireValue(!first, 'compiler tree exact app normal/build package closure');
  // Cargo metadata resolves all workspace features. The fixed package-scoped Cargo tree
  // selects its actual identities; only explicitly optional metadata edges may disappear.
  const packageIds = new Set([...seen].map(key => expected.get(key).id));
  const projected = compilerMetadataSelection(metadata, sourceRoot, packageIds);
  const wanted = [...projected.selected].map(id => identity(projected.packages.get(id))).sort();
  requireValue(bytes([...seen].sort()).equals(bytes(wanted)), 'compiler tree exact app normal/build package closure');
  const records = wanted.map(key => {
    const pkg = expected.get(key);
    return { name: pkg.name, version: pkg.version, source: pkg.source };
  });
  const proof = { format: 'zryna.playground-compiler-cargo-tree-identities.v2', version: 2,
    target: COMPILER_TARGET, selection: { package: 'zryna-playground-compiler',
      edges: ['normal', 'build'], features: 'default' }, packageCount: records.length,
    registryPackageCount: records.filter(pkg => pkg.source !== null).length,
    metadataProjection: 'fixed-cargo-tree-with-only-explicit-optional-edges-pruned',
    stdoutSha256: sha256(stdout), identitySetSha256: sha256(bytes(records)), packages: records };
  return { proof, packageIds };
}

export function validateCompilerTree(stdout, metadata, sourceRoot) {
  return compilerTreeSelection(stdout, metadata, sourceRoot).proof;
}
