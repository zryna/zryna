// Strict isolated-target libtest replay. No source or process authenticity is inferred.
import assert from 'node:assert/strict';
const number = value => { const n = Number(value); assert(Number.isSafeInteger(n) && n >= 0, 'bounded Rust count'); return n; };
export function replayTarget(text) {
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 16 * 1024 * 1024, 'bounded Rust transcript');
  assert(!text.includes('\u001b') && !text.replaceAll('\r\n', '\n').includes('\r'), 'unambiguous Rust transcript');
  const rows = [], seen = new Set(); let running = null, counts = null;
  for (const line of text.replaceAll('\r\n', '\n').split('\n')) {
    let match = /^running (\d+) tests?$/u.exec(line);
    if (match) { assert.equal(running, null, 'one isolated target running block'); assert.equal(counts, null); running = number(match[1]); continue; }
    match = /^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored(?:, [^\n]*)?)$/u.exec(line);
    if (match) {
      assert(running !== null && counts === null, 'named row inside target block');
      assert(!seen.has(match[1]) && rows.length < 65536, 'unique bounded actual test names');
      seen.add(match[1]); rows.push({ name: match[1], outcome: match[2].startsWith('ignored') ? 'ignored' : match[2] }); continue;
    }
    match = /^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9]+(?:\.[0-9]+)?s$/u.exec(line);
    if (match) {
      assert(running !== null && counts === null, 'one complete target footer');
      counts = Object.fromEntries(['passed', 'failed', 'ignored', 'measured', 'filtered'].map((key, i) => [key, number(match[i + 2])]));
      assert.equal(match[1], 'ok', 'failed suite'); continue;
    }
    if (/^(?:running |test result:|test .* \.\.\. )/u.test(line)) {
      assert(/^test [A-Za-z0-9_:]+ has been running for over [0-9]+ seconds$/u.test(line), 'unparsed/listed/truncated Rust accounting line');
    }
  }
  assert(counts && running !== null, 'missing actual target block/footer');
  assert.equal(counts.failed, 0); assert.equal(counts.measured, 0);
  assert.equal(rows.length, running, 'complete named target inventory');
  assert.equal(counts.passed + counts.failed + counts.ignored + counts.measured, running, 'running/footer census');
  assert.equal(rows.filter(row => row.outcome === 'ok').length, counts.passed, 'raw passed census');
  assert.equal(rows.filter(row => row.outcome === 'ignored').length, counts.ignored, 'raw ignored census');
  return { counts, rows };
}
