// Raw reconciliation only; caller separately authenticates source, host and paired oracle provenance.
import assert from 'node:assert/strict';
import { win32 } from 'node:path';
import { canonical, MAX_LOG, parse, portable, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { localAbsolute, validateWindowsAdmission, validateWindowsCapture } from './cli-performance-windows-binding.mjs';
import { validateWindowsOracles } from './cli-performance-windows-files.mjs';
import { windowsInvocation } from './cli-performance-windows-round.mjs';
import { checkedWindowsMetrics, MEMORY_METHOD, parseWindowsHelperMetadata, summarizeWindowsFive } from './cli-performance-windows-metrics.mjs';
function unqualified(value) {
  assert.equal(value.sourceAuthenticated, false); assert.equal(value.performanceQualified, false);
  assert.equal(value.baselineAuthority, null); assert.equal(value.thresholds, null);
}
export function replayWindowsRound({ document, capture, session, operator, oracles, read }) {
  const bytes = (path, maximum = MAX_LOG) => {
    portable(path); const result = read(path);
    assert(Buffer.isBuffer(result) && result.length <= maximum, 'bounded retained bytes'); return result;
  };
  const same = (path, value) => assert(bytes(path).equals(Buffer.from(canonical(value))), `retained binding: ${path}`);
  validateWindowsCapture(document, capture); validateWindowsOracles(document, capture, oracles);
  unqualified(capture); unqualified(session);
  same('operator-input.json', operator); same('round/capture.json', capture);
  same('round/catalogue.json', document); same('round/session.json', session);
  same('round/artifact-oracles.json', oracles);
  validateWindowsAdmission(capture, bytes('round/operator-admission.json'));
  localAbsolute(operator.root);
  assert.equal(operator.expectedHead, capture.sourceCommit); assert.equal(operator.expectedTree, capture.sourceTree);
  assert.equal(operator.round, capture.round); assert.equal(session.round, capture.round);
  assert.equal(session.format, 'zryna.cli-performance-windows-observations.draft.v1');
  assert.equal(session.status, 'collected-unqualified-observations'); assert(!Object.hasOwn(session, 'failure'));
  assert.equal(session.closureStatus, 'blocked'); assert.equal(session.memoryMethod, MEMORY_METHOD);
  assert.equal(session.memoryMeaning, 'actual PeakJobMemoryUsed committed bytes for the measured Job tree');
  assert.equal(session.wallMeaning, 'native QPC delta/frequency in exact decimal strings; spread is an exact integer ratio');
  assert.equal(session.captureSha256, sha256(canonical(capture)));
  assert.equal(session.catalogueSha256, capture.catalogueSha256);
  assert.equal(session.cliBuildClass, capture.cliBuildClass); assert.deepEqual(session.operatorAdmission, capture.operatorAdmission);
  assert.equal(win32.basename(capture.operatorAdmission.path), 'windows-operator-binding.json');
  const output = win32.join(win32.dirname(capture.operatorAdmission.path), 'round');
  const rows = matrixFor(document, 'windows-x86_64');
  assert.equal(session.cells.length, 20);
  const summaries = rows.map((row, cellIndex) => {
    const cell = session.cells[cellIndex]; assert.equal(cell.selector, row.id);
    assert.equal(cell.attempts.length, 6, 'one warmup and all five measured attempts');
    const attempts = cell.attempts.map((attempt, index) => {
      const id = `${row.id.replaceAll('/', '_')}_${index}`, role = index === 0 ? 'warmup' : 'measured';
      const prefix = `round/${id}`, call = windowsInvocation(row, capture, operator.root, win32.join(output, `${id}.sample`));
      const helperStreams = Object.fromEntries(['stdout', 'stderr'].map(name => {
        assert.equal(bytes(`${prefix}.helper.${name}`).length, 0, 'empty complete helper stream');
        return [name, { capturedBytes: 0, retainedBytes: 0, truncated: false,
          evidence: 'complete-captured-stream', retainedFile: `${id}.helper.${name}` }];
      }));
      assert.deepEqual(parse(bytes(`${prefix}.process.json`)), { id, selector: row.id, index, role, call,
        exitCode: 0, signal: null, error: null, stdoutBytes: 0, stderrBytes: 0, helperStreams });
      const raw = bytes(`${prefix}.metadata.raw`, 4096), stdout = bytes(`${prefix}.stdout`), stderr = bytes(`${prefix}.stderr`);
      for (const [name, retained] of [['result.json', raw], ['stdout.raw', stdout], ['stderr.raw', stderr]])
        assert(bytes(`${prefix}.sample/${name}`).equals(retained), 'original sample equals flattened retained bytes');
      assert.deepEqual(parse(bytes(`${prefix}.sample-read.json`)), { error: null,
        metadataBytes: raw.length, stdoutBytes: stdout.length, stderrBytes: stderr.length });
      const metrics = checkedWindowsMetrics(parseWindowsHelperMetadata(raw), capture, call.argv.length - 5, stdout, stderr);
      assert.deepEqual(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(stdout)), row.expectedResponse, 'original typed source outcome');
      assert(stdout.equals(Buffer.from(oracles[cellIndex].stdout)) && stderr.equals(Buffer.from(oracles[cellIndex].stderr)), 'exact paired oracle streams');
      const reconciled = { id, role, ...metrics };
      assert.deepEqual(attempt, reconciled, 'session attempt equals original helper metrics'); return reconciled;
    });
    const summary = summarizeWindowsFive(attempts.slice(1));
    assert.deepEqual(cell.summary, summary, 'all-five exact native summaries'); return { selector: row.id, summary };
  });
  return { rawMetricsReconciled: true, cells: 20, attempts: 120, warmups: 20, measuredAttempts: 100,
    summaries, sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
}
