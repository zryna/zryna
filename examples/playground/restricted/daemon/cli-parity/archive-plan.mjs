// Pure fixed-archive verification. No I/O, extraction, process execution or signature authority.
import { verifyArchive } from '../../../../../scripts/distribution/verify.mjs';
import { validateReleaseText } from '../../../../../scripts/distribution-release/validate.mjs';
import { release } from './plan.mjs';
import { fail, freeze, sha256 } from './records.mjs';

export async function fullReleaseImageFragment(archiveBytes, envelopeBytes) {
  if (!(archiveBytes instanceof Uint8Array) || archiveBytes.length !== release.archive.bytes ||
      !(envelopeBytes instanceof Uint8Array) || envelopeBytes.length !== 7253) fail();
  // Snapshot all accepted carriers before the first async boundary.
  const archive = Buffer.from(archiveBytes); const envelopeData = Buffer.from(envelopeBytes);
  if (sha256(archive) !== release.archive.sha256 || sha256(envelopeData) !== release.envelopeSha256) fail();
  const envelope = validateReleaseText(envelopeData.toString('utf8'));
  const subject = envelope.subjects.find(value => value.target === 'x86_64-unknown-linux-gnu');
  if (!subject || envelope.version !== release.version || envelope.source.commit !== release.sourceCommit ||
      envelope.source.tree !== release.sourceTree) fail();
  const source = Object.fromEntries(['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch']
    .map(key => [key, envelope.source[key]]));
  const verified = await verifyArchive(archive, { filename: subject.archive.path,
    size: subject.archive.size, sha256: subject.archive.sha256, version: envelope.version, source,
    target: { triple: subject.target, archiveFormat: 'tar-gzip', platformBaseline: subject.platformBaseline },
    recipe: envelope.recipe });
  if (verified.files.length !== 349 || verified.files.reduce((sum, file) => sum + file.data.length, 0) !== 154023815) fail();
  const files = verified.files.map(file => ({ path: `opt/zryna-release/${file.path}`, archivePath: file.path,
    bytes: file.data.length, sha256: sha256(file.data), mode: file.mode, uid: 0, gid: 0 }));
  const executables = files.filter(file => file.mode & 0o111).map(file => file.archivePath).sort();
  if (JSON.stringify(executables) !== JSON.stringify(['bin/zryna', 'runtime/node/bin/node']) ||
      files.some(file => ![0o644, 0o755].includes(file.mode))) fail();
  for (const pin of [release.compiler, release.node]) {
    const actual = files.find(file => file.archivePath === pin.path);
    if (!actual || ['bytes', 'sha256', 'mode'].some(key => actual[key] !== pin[key])) fail();
  }
  const directories = new Set(['opt', 'opt/zryna-release']);
  for (const file of files) {
    const parts = file.path.split('/');
    for (let count = 1; count < parts.length; count++) directories.add(parts.slice(0, count).join('/'));
  }
  return freeze({ format: 'zryna.playground-cli-parity-release-image-fragment.v1', version: 1,
    release, source: envelope.source, directories: [...directories].sort().map(path => ({ path, mode: 0o555, uid: 0, gid: 0 })),
    files, expandedFileBytes: 154023815, completeSignedArchiveInventory: true, originalModesRetained: true,
    acceptance: false, signatureEvidenceRequired: true, executionApproved: false,
    scope: 'pure-byte-verified-full349-image-plan-no-execution-authority' });
}
