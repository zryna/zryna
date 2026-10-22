import assert from 'node:assert/strict';
import { exact, MAX_LOG, retained, reject, utf8 } from './evidence-input.mjs';

export const sourceCommands = Object.freeze({
  commit: ['git', 'rev-parse', 'HEAD'], tree: ['git', 'rev-parse', 'HEAD^{tree}'],
  state: ['git', 'status', '--porcelain=v1', '--untracked-files=all'],
});
export function sourceAccounting(sourceState, source, root) {
  exact(sourceState, ['before', 'after'], 'actual source state logs');
  for (const phase of ['before', 'after']) {
    exact(sourceState[phase], Object.keys(sourceCommands), 'source observation inventory');
    for (const [field, argv] of Object.entries(sourceCommands)) {
      const result = sourceState[phase][field];
      exact(result, ['argv', 'exitCode', 'signal', 'error', 'stdout', 'stderr'], 'source command');
      assert.deepEqual(result.argv, argv, 'actual source observation command');
      if (result.exitCode !== 0 || result.signal !== null || result.error !== null) reject('source inspection failed');
      const prefix = `source/${phase}-${field}`;
      const stdout = utf8(retained(result.stdout, root, MAX_LOG, `${prefix}.stdout.log`)).replace(/\r\n/g, '\n');
      retained(result.stderr, root, MAX_LOG, `${prefix}.stderr.log`);
      assert.equal(stdout, field === 'state' ? '' : `${source[field]}\n`, 'actual source identity or cleanliness');
    }
  }
}
