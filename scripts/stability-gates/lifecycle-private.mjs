// Fixed manual roles. Untrusted JSON supplies data, never argv or source code.
import assert from 'node:assert/strict';
import { mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dispatchData, selection } from './lifecycle-private-io.mjs';
import { capture } from './lifecycle-private-capture.mjs';
import { run } from './lifecycle-private-execute.mjs';

if (resolve(process.argv[1] || '') === fileURLToPath(import.meta.url)) {
  process.umask(0o077);
  const operation = process.env.LIFECYCLE_OPERATION;
  const input = selection(dispatchData(process.env.LIFECYCLE_SELECTION), operation);
  const base = join(process.env.RUNNER_TEMP, 'lifecycle');
  if (operation === 'capture') {
    mkdirSync(base, { mode: 0o700 });
    await capture(base, input);
  } else {
    assert.equal(process.getuid(), 0);
    await run(base, input);
  }
}
