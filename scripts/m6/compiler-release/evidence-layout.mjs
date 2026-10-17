// Retained evidence only. Decoding these bytes grants no execution or publication authority.
import { gunzipSync } from 'node:zlib';
import { decodeTar, encodeTar } from '../../distribution/archive-tar.mjs';
import { bytes, describe, exactKeys, orderedPaths, parseCanonical, requireValue, SHA } from './common.mjs';

export const evidenceRoot = 'm6-compiler-publication-evidence';
export const evidenceLayoutPath = 'evidence-layout.json';
export const evidenceMaximumBytes = 96 * 1024 * 1024;
export const replicaEvidence = Object.freeze([1, 2].map(replica => Object.freeze({ replica,
  path: `replica${replica}-evidence.tar.gz`, root: `m6-compiler-replica${replica}-evidence` })));
const replicaNames = new Set(['job-observation.json', 'cargo-metadata.json', 'tree-identities.json',
  'cargo-metadata-after.json', 'tree-identities-after.json',
  'rust-materials.json', 'material-closure.json', 'replica.json',
  ...Array.from({ length: 7 }, (_, index) => String(index + 1).padStart(2, '0'))
    .flatMap(number => [`command-${number}.stdout`, `command-${number}.stderr`])]);

function inventory(files, maximum, maximumBytes = evidenceMaximumBytes) {
  requireValue(files.length > 0 && files.length <= maximum, 'compiler evidence file count');
  orderedPaths(files.map(file => file.path));
  requireValue(files.every(file => file.mode === 0o644 && Buffer.isBuffer(file.data)) &&
    files.reduce((total, file) => total + file.data.length, 0) <= maximumBytes,
  'compiler evidence ordinary finite bytes');
  return files.map(file => describe(file.path, file.data));
}

function replicaInventory(files) {
  const result = inventory(files, 400);
  requireValue(files.every(file => replicaNames.has(file.path) || file.path.startsWith('licenses/')) &&
    files.some(file => file.path === 'replica.json'), 'compiler evidence closed replica paths');
  // Only the two fixed outer subjects are opened as archives. Replica files are never recursed.
  return result;
}

async function boundedDecode(packet, root, epoch, maximumFiles, maximumBytes = evidenceMaximumBytes) {
  requireValue(Buffer.isBuffer(packet) && packet.length > 0 && packet.length <= evidenceMaximumBytes,
    'compiler evidence bounded archive');
  // Bound expansion before the shared canonical decoder allocates its broader distribution limit.
  gunzipSync(packet, { maxOutputLength: maximumBytes + maximumFiles * 12 * 1024 + 1024 });
  const files = await decodeTar(packet, root, epoch);
  inventory(files, maximumFiles, maximumBytes);
  return files;
}

export async function encodePublicationEvidence(admission, replicas, reproduction, source) {
  inventory(admission, 400);
  requireValue(admission.every(file => file.path.startsWith('admission/')) &&
    Array.isArray(replicas) && replicas.length === 2 && reproduction.path === 'assembly/reproduction.json',
  'compiler evidence fixed subjects');
  const rawBytes = [...admission, reproduction, ...replicas.flat()]
    .reduce((total, file) => total + file.data.length, 0);
  requireValue(rawBytes <= evidenceMaximumBytes,
  'compiler evidence aggregate expanded bytes');
  const files = [...admission, reproduction], nested = [];
  for (const [index, selected] of replicaEvidence.entries()) {
    const captured = replicas[index], listed = replicaInventory(captured);
    const data = await encodeTar(selected.root, captured, source.sourceDateEpoch);
    requireValue(data.length <= evidenceMaximumBytes, 'compiler evidence finite replica archive');
    files.push({ path: selected.path, mode: 0o644, data });
    nested.push({ replica: selected.replica, archive: describe(selected.path, data), files: listed });
  }
  files.sort((a, b) => a.path < b.path ? -1 : 1);
  const layout = bytes({ format: 'zryna.playground-compiler-publication-evidence-layout.v2', version: 2,
    source, files: inventory(files, 499), replicas: nested });
  requireValue(layout.length <= 262144, 'compiler evidence layout record bound');
  requireValue(rawBytes + layout.length <= evidenceMaximumBytes,
    'compiler evidence aggregate expanded bytes');
  files.push({ path: evidenceLayoutPath, mode: 0o644, data: layout });
  files.sort((a, b) => a.path < b.path ? -1 : 1); inventory(files, 500);
  const packet = await encodeTar(evidenceRoot, files, source.sourceDateEpoch);
  requireValue(packet.length > 0 && packet.length <= evidenceMaximumBytes,
    'compiler evidence bounded archive');
  return packet;
}

export async function decodePublicationEvidence(packet, source) {
  const files = await boundedDecode(packet, evidenceRoot, source.sourceDateEpoch, 500);
  const layoutFile = files.find(file => file.path === evidenceLayoutPath);
  requireValue(layoutFile, 'compiler evidence required v2 layout');
  const layout = parseCanonical(layoutFile.data);
  exactKeys(layout, ['format', 'version', 'source', 'files', 'replicas']);
  requireValue(layout.format === 'zryna.playground-compiler-publication-evidence-layout.v2' &&
    layout.version === 2 && bytes(layout.source).equals(bytes(source)), 'compiler evidence selected layout/source');
  const subjects = files.filter(file => file !== layoutFile);
  requireValue(bytes(layout.files).equals(bytes(inventory(subjects, 499))) &&
    subjects.every(file => file.path.startsWith('admission/') || file.path === 'assembly/reproduction.json' ||
      replicaEvidence.some(selected => selected.path === file.path)) &&
    subjects.some(file => file.path === 'assembly/reproduction.json') &&
    Array.isArray(layout.replicas) && layout.replicas.length === 2, 'compiler evidence exact outer inventory');
  const replicas = [];
  const budgets = layout.replicas.map(record => {
    exactKeys(record, ['replica', 'archive', 'files']);
    requireValue(Array.isArray(record.files) && record.files.length > 0 && record.files.length <= 400,
      'compiler evidence declared replica count');
    orderedPaths(record.files.map(file => file.path));
    for (const file of record.files) {
      exactKeys(file, ['path', 'bytes', 'sha256']);
      requireValue(Number.isSafeInteger(file.bytes) && file.bytes >= 0 && file.bytes <= 64 * 1024 * 1024 &&
        SHA(file.sha256), 'compiler evidence declared replica bytes');
    }
    return record.files.reduce((total, file) => total + file.bytes, 0);
  });
  const outerRaw = subjects.filter(file => !replicaEvidence.some(selected => selected.path === file.path));
  requireValue(layoutFile.data.length + outerRaw.reduce((total, file) => total + file.data.length, 0) +
    budgets.reduce((total, size) => total + size, 0) <= evidenceMaximumBytes,
  'compiler evidence aggregate expanded bytes');
  for (const [index, selected] of replicaEvidence.entries()) {
    const record = layout.replicas[index], archive = files.find(file => file.path === selected.path);
    exactKeys(record, ['replica', 'archive', 'files']);
    requireValue(archive && record.replica === selected.replica &&
      bytes(record.archive).equals(bytes(describe(selected.path, archive.data))), 'compiler evidence exact replica subject');
    const captured = await boundedDecode(archive.data, selected.root, source.sourceDateEpoch, 400, budgets[index]);
    requireValue(bytes(record.files).equals(bytes(replicaInventory(captured))), 'compiler evidence exact nested inventory');
    replicas.push(captured);
  }
  requireValue([layoutFile, ...subjects.filter(file => !replicaEvidence.some(selected => selected.path === file.path)),
    ...replicas.flat()].reduce((total, file) => total + file.data.length, 0) <= evidenceMaximumBytes,
  'compiler evidence aggregate expanded bytes');
  return { files, layout, replicas };
}
