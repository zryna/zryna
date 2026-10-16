import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { bytes, portablePath, requireValue, sha256 } from '../../distribution/canonical.mjs';
import { compilerLockPackages, identity, REGISTRY } from './material-lock.mjs';
import { compilerTargetMatches, COMPILER_TARGET } from './material-target.mjs';

export { COMPILER_TARGET };
const APP = 'zryna-playground-compiler';

function localPath(sourceRoot, path) {
  requireValue(typeof path === 'string' && isAbsolute(path) && resolve(path) === path,
    'compiler metadata absolute source path');
  const selected = relative(sourceRoot, path);
  portablePath(selected);
  requireValue(!selected.startsWith('../') && !isAbsolute(selected), 'compiler metadata source escape');
  return selected;
}

export function compilerMetadataSelection(metadata, sourceRoot) {
  requireValue(typeof sourceRoot === 'string' && isAbsolute(sourceRoot)
    && resolve(sourceRoot) === sourceRoot && metadata?.version === 1
    && metadata.workspace_root === sourceRoot, 'compiler metadata source root');
  requireValue(Array.isArray(metadata.packages) && metadata.packages.length > 0
    && metadata.packages.length <= 2048 && Array.isArray(metadata.resolve?.nodes)
    && metadata.resolve.nodes.length <= 2048 && Array.isArray(metadata.workspace_members)
    && new Set(metadata.workspace_members).size === metadata.workspace_members.length,
  'compiler metadata graph budget');
  const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  requireValue(packages.size === metadata.packages.length && nodes.size === metadata.resolve.nodes.length
    && metadata.workspace_members.every(id => packages.has(id)), 'compiler metadata duplicate identities');
  const roots = metadata.packages.filter(pkg => pkg.name === APP
    && metadata.workspace_members.includes(pkg.id));
  requireValue(roots.length === 1 && roots[0].source === null && roots[0].version === '0.1.0'
    && localPath(sourceRoot, roots[0].manifest_path) === `apps/${APP}/Cargo.toml`
    && Array.isArray(roots[0].targets) && roots[0].targets.some(target =>
      target.name === APP && target.kind?.includes('bin')), 'compiler metadata sole app root');
  const pending = [roots[0].id];
  const selected = new Set();
  const edges = [];
  while (pending.length) {
    const id = pending.pop();
    if (selected.has(id)) continue;
    selected.add(id);
    requireValue(selected.size <= 512, 'compiler metadata selected graph budget');
    const node = nodes.get(id);
    requireValue(packages.has(id) && node && Array.isArray(node.deps)
      && node.deps.length <= 1024 && Array.isArray(node.features), 'compiler metadata selected node');
    const duplicate = new Set();
    for (const dep of node.deps) {
      requireValue(typeof dep.pkg === 'string' && packages.has(dep.pkg) && !duplicate.has(dep.pkg)
        && Array.isArray(dep.dep_kinds) && dep.dep_kinds.length > 0 && dep.dep_kinds.length <= 16,
      'compiler metadata dependency identity');
      duplicate.add(dep.pkg);
      for (const kind of dep.dep_kinds) {
        requireValue([null, 'build', 'dev'].includes(kind.kind)
          && (kind.target === null || typeof kind.target === 'string'),
        'compiler metadata dependency kind');
      }
      const kinds = dep.dep_kinds.filter(kind => kind.kind !== 'dev'
        && compilerTargetMatches(kind.target));
      if (kinds.length) {
        pending.push(dep.pkg);
        edges.push({ from: id, to: dep.pkg, kinds: kinds.map(kind => ({ ...kind })) });
        requireValue(edges.length <= 8192, 'compiler metadata edge budget');
      }
    }
  }
  for (const id of selected) {
    const pkg = packages.get(id);
    requireValue(typeof pkg.name === 'string' && /^[A-Za-z0-9_-]+$/.test(pkg.name)
      && typeof pkg.version === 'string' && /^[0-9][A-Za-z0-9+._-]*$/.test(pkg.version),
    'compiler metadata selected identity');
    if (pkg.source === null) {
      requireValue(metadata.workspace_members.includes(id), 'compiler metadata nonworkspace path');
      localPath(sourceRoot, pkg.manifest_path);
    } else {
      requireValue(typeof pkg.manifest_path === 'string' && isAbsolute(pkg.manifest_path)
        && resolve(pkg.manifest_path) === pkg.manifest_path
        && pkg.manifest_path.endsWith(`/${pkg.name}-${pkg.version}/Cargo.toml`),
      'compiler metadata selected registry path');
    }
  }
  return { selected, packages, edgeCount: edges.length };
}

export function selectCompilerGraph({ metadata, sourceRoot, cargoLock, approved }) {
  const { selected, packages, edgeCount } = compilerMetadataSelection(metadata, sourceRoot);
  const locked = compilerLockPackages(cargoLock);
  const identities = new Set();
  const external = [];
  const workspace = [];
  for (const id of selected) {
    const pkg = packages.get(id);
    requireValue(typeof pkg.name === 'string' && typeof pkg.version === 'string'
      && !identities.has(identity(pkg)), 'compiler metadata ambiguous package');
    identities.add(identity(pkg));
    const lock = locked.get(identity(pkg));
    requireValue(lock && (lock.source ?? null) === pkg.source, 'compiler metadata lock identity');
    if (pkg.source === null) {
      requireValue(metadata.workspace_members.includes(id), 'compiler metadata nonworkspace path package');
      workspace.push({ name: pkg.name, version: pkg.version,
        manifestPath: localPath(sourceRoot, pkg.manifest_path) });
    } else {
      const pin = approved.get(identity(pkg));
      requireValue(pkg.source === REGISTRY && pin && lock.checksum === pin.crateSha256,
        'compiler metadata unknown crate or lock checksum');
      requireValue(typeof pkg.manifest_path === 'string' && isAbsolute(pkg.manifest_path)
        && resolve(pkg.manifest_path) === pkg.manifest_path
        && pkg.manifest_path.endsWith(`/${pkg.name}-${pkg.version}/Cargo.toml`),
      'compiler metadata crate cache manifest');
      external.push({ pin, cacheRoot: dirname(pkg.manifest_path) });
    }
  }
  external.sort((a, b) => identity(a.pin) < identity(b.pin) ? -1 : 1);
  workspace.sort((a, b) => identity(a) < identity(b) ? -1 : 1);
  requireValue(external.length > 0 && external.length <= 256, 'compiler metadata crate count');
  return { external, proof: { format: 'zryna.playground-compiler-cargo-closure.v1', version: 1,
    target: COMPILER_TARGET, selection: { package: APP, features: 'default', edges: ['normal', 'build'] },
    metadataCanonicalSha256: sha256(bytes(metadata)), cargoLockSha256: sha256(cargoLock), workspace,
    packages: external.map(({ pin }) => ({ name: pin.name, version: pin.version,
      crateSha256: pin.crateSha256 })), selectedNodeCount: selected.size, selectedEdgeCount: edgeCount } };
}
