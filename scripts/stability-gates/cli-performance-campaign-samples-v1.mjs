// Pure native-sample projection only; this helper issues no evidence or admission key.
import assert from 'node:assert/strict';
import { u64 } from './windows-job-metrics-v1/calibration/protocol.mjs';
export function retainNumericArchiveSamples(host, session) {
  assert(['linux-x86_64', 'windows-x86_64'].includes(host));
  assert(Array.isArray(session.cells) && session.cells.length === (host === 'linux-x86_64' ? 24 : 20));
  return session.cells.map(cell => {
    assert.equal(typeof cell.selector, 'string'); assert.equal(cell.attempts.length, 6);
    return { selector: cell.selector, attempts: cell.attempts.map((attempt, index) => {
      assert.equal(typeof attempt.id, 'string'); assert.equal(attempt.role, index === 0 ? 'warmup' : 'measured');
      const identity = { id: attempt.id, index, role: attempt.role };
      if (host === 'linux-x86_64') {
        assert(Number.isSafeInteger(attempt.wallNs) && attempt.wallNs > 0 && attempt.wallNs <= 15000000000);
        assert(Number.isSafeInteger(attempt.peakRssKiB) && attempt.peakRssKiB > 0 && attempt.peakRssKiB <= 999999999999999);
        return { ...identity, wallNs: String(attempt.wallNs), peakRssKiB: String(attempt.peakRssKiB) };
      }
      const keys = ['qpcStart', 'qpcEnd', 'wallTicks', 'qpcFrequency', 'peakJobMemoryUsedBytes'];
      for (const key of keys) { assert.equal(typeof attempt[key], 'string'); u64(attempt[key]); }
      return { ...identity, ...Object.fromEntries(keys.map(key => [key, attempt[key]])) };
    }) };
  });
}
