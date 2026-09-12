import { requireValue, sha256 } from './canonical.mjs';
import { targetPaths } from './inventory.mjs';
import { NODE_TARGETS } from './materials.mjs';

const LINUX_TAR = Object.freeze({
  size: 203161600,
  sha256: 'd9bd21210a6aaf6d02d35f739d02bb05ecda8c63deb1abb0150c288c3263fb48',
});

function members(target) {
  const windows = target === 'x86_64-pc-windows-msvc';
  const root = windows ? 'node-v22.22.1-win-x64' : 'node-v22.22.1-linux-x64';
  const record = NODE_TARGETS[target];
  const paths = targetPaths(target);
  return [
    { sourcePath: `${root}/${windows ? 'node.exe' : 'bin/node'}`, path: paths.node,
      type: 'ordinary', mode: 0o755, size: record.executable[0], sha256: record.executable[1] },
    { sourcePath: `${root}/LICENSE`, path: 'licenses/node-LICENSE', type: 'ordinary', mode: 0o644,
      size: record.license[0], sha256: record.license[1] },
  ];
}

// The protected #406 layer supplies a capability created only after validating qualification
// nativeTools or the accepted production recipe. This layer never accepts or executes tool paths.
export async function captureNodeMaterials(target, archive, capability) {
  const record = NODE_TARGETS[target];
  requireValue(record && Buffer.isBuffer(archive) && archive.length === record.archiveSize
    && sha256(archive) === record.archiveSha256, 'Node archive identity');
  requireValue(capability && typeof capability.readOrdinaryMember === 'function',
    'Node archive capability');
  let container = archive;
  let format = 'zip';
  if (target === 'x86_64-unknown-linux-gnu') {
    requireValue(typeof capability.expandXz === 'function', 'Node XZ capability');
    container = await capability.expandXz({ archive, expected: LINUX_TAR });
    requireValue(Buffer.isBuffer(container) && container.length === LINUX_TAR.size
      && sha256(container) === LINUX_TAR.sha256, 'Node expanded tar identity');
    format = 'tar';
  }
  const files = [];
  for (const expected of members(target)) {
    const data = await capability.readOrdinaryMember({ format, container,
      containerSha256: sha256(container), expected: structuredClone(expected) });
    requireValue(Buffer.isBuffer(data) && data.length === expected.size
      && sha256(data) === expected.sha256, 'Node selected member');
    files.push({ path: expected.path, mode: expected.mode, data: Buffer.from(data) });
  }
  return files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
}

export function nodeMaterialMembers(target) {
  requireValue(Object.hasOwn(NODE_TARGETS, target), 'Node material target');
  return structuredClone(members(target));
}

export function nodeExpandedArchive(target) {
  requireValue(Object.hasOwn(NODE_TARGETS, target), 'Node material target');
  return target === 'x86_64-unknown-linux-gnu' ? structuredClone(LINUX_TAR) : null;
}
