import assert from 'node:assert/strict';
import test from 'node:test';
import { fullReleaseImageFragment } from './archive-plan.mjs';
import { release } from './plan.mjs';

test('full349 plan refuses wrong carrier lengths and a same-length substituted archive before parsing', async () => {
  for (const [archive, envelope] of [[{}, {}], [new Uint8Array(), new Uint8Array(7253)],
    [new Uint8Array(release.archive.bytes), new Uint8Array(7252)]]) {
    await assert.rejects(fullReleaseImageFragment(archive, envelope), /CLI-PARITY/);
  }
  await assert.rejects(fullReleaseImageFragment(new Uint8Array(release.archive.bytes), new Uint8Array(7253)), /CLI-PARITY/);
});
