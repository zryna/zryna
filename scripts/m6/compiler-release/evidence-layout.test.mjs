import assert from 'node:assert/strict';
import test from 'node:test';
import { gzipSync, gunzipSync } from 'node:zlib';
import { encodeTar, decodeTar } from '../../distribution/archive-tar.mjs';
import { bytes, describe, sha256 } from './common.mjs';
import { encodePublicationEvidence, decodePublicationEvidence, evidenceLayoutPath, evidenceRoot,
  replicaEvidence, evidenceMaximumBytes } from './evidence-layout.mjs';

const source = { repository: 'https://github.com/zryna/zryna', commit: 'a'.repeat(40),
  tree: 'b'.repeat(40), sourceDateEpoch: 1 };
const file = (path, data = Buffer.from('INERT EVIDENCE FIXTURE\n')) => ({ path, mode: 0o644, data });
function fixture(licenses = 304) {
  const admission = [file('admission/admission.json')];
  const replicas = [1, 2].map(replica => [file('replica.json', bytes({ fixture: true, replica })),
    file('cargo-metadata.json', bytes({ fixture: true, distinctRoot: replica })),
    ...Array.from({ length: licenses }, (_, index) => file(`licenses/crate-${String(index).padStart(3, '0')}/LICENSE`,
      Buffer.from(`INERT LICENSE FIXTURE ${index}\n`)))].sort((a, b) => a.path < b.path ? -1 : 1));
  return { admission, replicas, reproduction: file('assembly/reproduction.json') };
}
async function packet(f) {
  return encodePublicationEvidence(f.admission, f.replicas, f.reproduction, source);
}
async function alteredPacket(original, change) {
  const files = await decodeTar(original, evidenceRoot, 1);
  const layoutFile = files.find(value => value.path === evidenceLayoutPath);
  const layout = JSON.parse(layoutFile.data);
  await change(files, layout);
  layoutFile.data = bytes(layout);
  files.sort((a, b) => a.path < b.path ? -1 : 1);
  return encodeTar(evidenceRoot, files, 1);
}
function reindexOuter(files, layout) {
  layout.files = files.filter(value => value.path !== evidenceLayoutPath).map(value => describe(value.path, value.data));
}
async function replaceReplica(files, layout, captured, raw) {
  const selected = replicaEvidence[0], subject = files.find(value => value.path === selected.path);
  subject.data = raw ?? await encodeTar(selected.root, captured, 1);
  layout.replicas[0].archive = describe(selected.path, subject.data);
  layout.replicas[0].files = captured.map(value => describe(value.path, value.data));
  reindexOuter(files, layout);
}

test('304 licenses in EACH replica roundtrip independently without raising shared 512-file bounds', async () => {
  const f = fixture(), encoded = await packet(f), decoded = await decodePublicationEvidence(encoded, source);
  assert.equal(decoded.files.length, 5); assert.equal(decoded.layout.version, 2);
  for (const [index, actual] of decoded.replicas.entries()) {
    assert.equal(actual.length, 306); assert.equal(actual.filter(value => value.path.startsWith('licenses/')).length, 304);
    assert.deepEqual(actual.map(value => describe(value.path, value.data)),
      f.replicas[index].map(value => describe(value.path, value.data)));
  }
  assert.notEqual(sha256(decoded.replicas[0][0].data), sha256(decoded.replicas[1][0].data));
  assert(encoded.equals(await packet(f)));
});

test('per-replica400 inclusive and first extra reject without dropping any original bytes', async () => {
  const inclusive = fixture(398);
  assert.equal((await decodePublicationEvidence(await packet(inclusive), source)).replicas[0].length, 400);
  await assert.rejects(packet(fixture(399)), /evidence file count/);
});

test('v1 flat layout and an unrelated source cannot masquerade as v2 evidence', async () => {
  const f = fixture(1), encoded = await packet(f);
  await assert.rejects(decodePublicationEvidence(encoded, { ...source, tree: 'c'.repeat(40) }));
  const flat = await encodeTar(evidenceRoot, [file('replica1/replica.json')], 1);
  await assert.rejects(decodePublicationEvidence(flat, source), /required v2 layout/);
});

test('declared aggregate expansion rejects first extra before nested decompression', async () => {
  const original = await packet(fixture(1));
  const changed = await alteredPacket(original, async (files, layout) => {
    for (const replica of layout.replicas) replica.files[0].bytes = 48 * 1024 * 1024 + 1;
  });
  await assert.rejects(decodePublicationEvidence(changed, source), /aggregate expanded bytes/);
});

for (const mutation of ['unlisted outer', 'unknown outer', 'missing replica', 'duplicate replica identity',
  'changed nested bytes', 'unlisted nested', 'unknown nested', 'recursive archive', 'nested first extra']) {
  test(`closed evidence rejects ${mutation}`, async () => {
    const f = fixture(1), original = await packet(f);
    const changed = await alteredPacket(original, async (files, layout) => {
      if (mutation === 'unlisted outer' || mutation === 'unknown outer') {
        files.push(file('unknown.txt')); files.sort((a, b) => a.path < b.path ? -1 : 1);
        if (mutation === 'unknown outer') reindexOuter(files, layout);
      } else if (mutation === 'missing replica') {
        files.splice(files.findIndex(value => value.path === replicaEvidence[1].path), 1); reindexOuter(files, layout);
      } else if (mutation === 'duplicate replica identity') layout.replicas[1].replica = 1;
      else {
        const captured = f.replicas[0].map(value => ({ ...value, data: Buffer.from(value.data) }));
        if (mutation === 'changed nested bytes') {
          captured[0].data = Buffer.from('ALTERED');
          const subject = files.find(value => value.path === replicaEvidence[0].path);
          subject.data = await encodeTar(replicaEvidence[0].root, captured, 1);
          layout.replicas[0].archive = describe(subject.path, subject.data); reindexOuter(files, layout);
        } else {
          if (mutation === 'nested first extra') {
            captured.push(...Array.from({ length: 398 }, (_, index) => file(`licenses/extra-${index}/LICENSE`)));
          } else captured.push(file(mutation === 'recursive archive' ? 'replica3-evidence.tar.gz' :
            mutation === 'unlisted nested' ? 'licenses/extra/LICENSE' : 'unexpected.json'));
          captured.sort((a, b) => a.path < b.path ? -1 : 1);
          await replaceReplica(files, layout, captured);
          if (mutation === 'unlisted nested') layout.replicas[0].files.pop();
        }
      }
    });
    await assert.rejects(decodePublicationEvidence(changed, source));
  });
}

for (const mutation of ['symlink', 'trailing bytes', 'noncanonical gzip', 'traversal path']) {
  test(`nested canonical archive rejects ${mutation}`, async () => {
    const f = fixture(1), original = await packet(f);
    const changed = await alteredPacket(original, async (files, layout) => {
      const selected = replicaEvidence[0];
      let raw = await encodeTar(selected.root, f.replicas[0], 1);
      if (mutation === 'trailing bytes') raw = Buffer.concat([raw, Buffer.from('extra')]);
      else if (mutation === 'noncanonical gzip') raw = gzipSync(gunzipSync(raw), { level: 1 });
      else {
        const expanded = gunzipSync(raw);
        if (mutation === 'symlink') expanded[156] = 50;
        else { expanded.fill(0, 0, 100); expanded.write('../outside', 0, 'ascii'); }
        expanded.fill(32, 148, 156);
        const sum = expanded.subarray(0, 512).reduce((total, value) => total + value, 0);
        expanded.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
        raw = gzipSync(expanded, { level: 9 }); raw[9] = 255;
      }
      await replaceReplica(files, layout, f.replicas[0], raw);
    });
    await assert.rejects(decodePublicationEvidence(changed, source));
  });
}


test('96 MiB logical aggregate includes generated layout in both producer and pre-inflate admission', async () => {
  const payload = Buffer.alloc(48 * 1024 * 1024 - 4);
  const f = { admission: [file('admission/a.json', Buffer.from('{}'))],
    reproduction: file('assembly/reproduction.json', Buffer.from('{}')),
    replicas: [1, 2].map(() => [file('licenses/payload', payload), file('replica.json', Buffer.from('{}'))]) };
  const rawBytes = [...f.admission, f.reproduction, ...f.replicas.flat()]
    .reduce((total, entry) => total + entry.data.length, 0);
  assert.equal(rawBytes, evidenceMaximumBytes);
  await assert.rejects(packet(f), /aggregate expanded bytes/);

  // Independently construct the previously admitted canonical packet: raw exactly at cap,
  // layout additional. Both nested archives are valid, so rejection must precede inflation.
  const outer = [...f.admission, f.reproduction], nested = [];
  for (const [index, selected] of replicaEvidence.entries()) {
    const data = await encodeTar(selected.root, f.replicas[index], 1);
    outer.push(file(selected.path, data));
    nested.push({ replica: selected.replica, archive: describe(selected.path, data),
      files: f.replicas[index].map(entry => describe(entry.path, entry.data)) });
  }
  outer.sort((a, b) => a.path < b.path ? -1 : 1);
  const layout = bytes({ format: 'zryna.playground-compiler-publication-evidence-layout.v2', version: 2,
    source, files: outer.map(entry => describe(entry.path, entry.data)), replicas: nested });
  assert(layout.length > 0 && layout.length < 4096);
  outer.push(file(evidenceLayoutPath, layout)); outer.sort((a, b) => a.path < b.path ? -1 : 1);
  const oversizedLogical = await encodeTar(evidenceRoot, outer, 1);
  await assert.rejects(decodePublicationEvidence(oversizedLogical, source), /aggregate expanded bytes/);

  // Keep all evidence and leave a finite margin for the layout: ordinary bytes roundtrip intact.
  f.replicas[0][0] = file('licenses/payload', payload.subarray(0, payload.length - 4096));
  const accepted = await decodePublicationEvidence(await packet(f), source);
  const actualLayout = accepted.files.find(entry => entry.path === evidenceLayoutPath);
  assert(rawBytes - 4096 + actualLayout.data.length <= evidenceMaximumBytes);
  for (const [index, entries] of accepted.replicas.entries())
    assert.deepEqual(entries, f.replicas[index]);
});
