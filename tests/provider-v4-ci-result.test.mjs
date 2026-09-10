import assert from 'node:assert/strict';
import test from 'node:test';

import { verifyProviderV4Result } from '../scripts/verify-provider-v4-ci-result.mjs';

test('provider-v4 aggregate accepts only required success or authenticated skip', () => {
  assert.doesNotThrow(() => verifyProviderV4Result('true', 'success'));
  assert.doesNotThrow(() => verifyProviderV4Result('false', 'skipped'));
  for (const [required, result] of [
    ['true', 'failure'],
    ['true', 'cancelled'],
    ['true', 'skipped'],
    ['false', 'success'],
    ['false', 'failure'],
    ['', 'skipped'],
    ['true', ''],
  ]) {
    assert.throws(
      () => verifyProviderV4Result(required, result),
      /invalid provider-v4 CI result/,
      `${required}/${result}`,
    );
  }
});
