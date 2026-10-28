// Actual publisher dependency regression; URL construction only, never an HTTP client.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';
import { parse } from 'yaml';

const root = createRequire(new URL('../package.json', import.meta.url));
const publisher = createRequire(root.resolve('@vscode/vsce/package.json'));
const consumer = createRequire(publisher.resolve('typed-rest-client/package.json'));
const qs = consumer('qs'), { getUrl } = consumer('./Util.js');

test('publisher consumes the fixed qs lock entry through its actual typed-rest-client dependency', () => {
  assert.equal(publisher('./package.json').version, '4.0.0');
  assert.equal(consumer('./package.json').version, '1.8.11');
  assert.equal(consumer('qs/package.json').version, '6.16.1');
  const lock = parse(readFileSync(new URL('../pnpm-lock.yaml', import.meta.url), 'utf8'), { uniqueKeys: true, maxAliasCount: 0 });
  assert(lock.packages['qs@6.16.1']); assert.equal(lock.packages['qs@6.16.0'], undefined);
  assert.equal(lock.snapshots['typed-rest-client@1.8.11'].dependencies.qs, '6.16.1');
});

test('non-callable own toString in comma arrays does not throw the advisory TypeError', () => {
  for (const value of [42, null, false, {}, 'non-callable']) {
    const params = { a: [{ toString: value }] };
    assert.equal(qs.stringify(params, { arrayFormat: 'comma' }), 'a=%5Bobject%20Object%5D');
    assert.equal(getUrl('/api', 'https://example.invalid/', {
      params, options: { arrayFormat: 'comma' },
    }), 'https://example.invalid/api?a=%5Bobject%20Object%5D');
  }
});

test('ordinary scalar and comma-array query construction remains unchanged', () => {
  const params = { a: [1, 2], value: 'safe' };
  assert.equal(qs.stringify(params, { arrayFormat: 'comma' }), 'a=1%2C2&value=safe');
  assert.equal(getUrl('/api', 'https://example.invalid/', {
    params, options: { arrayFormat: 'comma' },
  }), 'https://example.invalid/api?a=1,2&value=safe');
});
