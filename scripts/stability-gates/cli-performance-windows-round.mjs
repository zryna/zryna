// Finite fixed-workload controller. Process status precedes bounded stream evidence and validation.
import assert from 'node:assert/strict';
import { win32 } from 'node:path';
import { canonical, MAX_LOG, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { localAbsolute, validateWindowsCapture } from './cli-performance-windows-binding.mjs';
import { checkedWindowsMetrics, MEMORY_METHOD, summarizeWindowsFive } from './cli-performance-windows-metrics.mjs';
export function windowsInvocation(row, capture, root, sampleDirectory) {
  const args = row.argv.map(value => value === '$SOURCE_ROOT' ? root : value === '$NODE' ? capture.tools.node.path : value);
  return { executable: capture.tools.helper.path, sampleDirectory,
    argv: ['--timeout-ms', String(capture.hostWallGuardMs), '--output', sampleDirectory, '--', capture.tools.cli.path, ...args] };
}
function helperEvidence(result, id) {
  let remaining = MAX_LOG;
  return ['stdout', 'stderr'].map(name => {
    const buffer = result[name], retainedBytes = Math.min(buffer.length, remaining);
    remaining -= retainedBytes;
    const truncated = retainedBytes !== buffer.length;
    return { bytes: buffer.subarray(0, retainedBytes), name, record: { capturedBytes: buffer.length,
      retainedBytes, truncated, evidence: truncated ? 'bounded-prefix-ineligible' : 'complete-captured-stream',
      retainedFile: `${id}.helper.${name}${truncated ? '.prefix' : ''}` } };
  });
}
export async function collectWindowsRound(document, capture, io) {
  validateWindowsCapture(document, capture); localAbsolute(io.root); localAbsolute(io.output);
  const rows = matrixFor(document, 'windows-x86_64');
  const session = { format: 'zryna.cli-performance-windows-observations.draft.v1',
    captureSha256: sha256(canonical(capture)), catalogueSha256: capture.catalogueSha256,
    round: capture.round, cliBuildClass: capture.cliBuildClass, operatorAdmission: capture.operatorAdmission,
    memoryMethod: MEMORY_METHOD, memoryMeaning: 'actual PeakJobMemoryUsed committed bytes for the measured Job tree',
    wallMeaning: 'native QPC delta/frequency in exact decimal strings; spread is an exact integer ratio',
    status: 'invalid-or-incomplete', closureStatus: 'blocked', sourceAuthenticated: false,
    performanceQualified: false, baselineAuthority: null, thresholds: null, cells: [] };
  await io.retain('capture.json', Buffer.from(canonical(capture)));
  await io.retain('catalogue.json', Buffer.from(canonical(document)));
  try {
    await io.checkMaterials();
    for (const row of rows) {
      const attempts = [];
      for (let index = 0; index < 6; ++index) {
        await io.checkMaterials(); await io.beforeAttempt(row);
        const id = `${row.id.replaceAll('/', '_')}_${index}`;
        const call = windowsInvocation(row, capture, io.root, win32.join(io.output, `${id}.sample`));
        let result;
        try { result = await io.execute(call.executable, call.argv, capture.hostWallGuardMs + capture.cleanupGuardMs + 1000); }
        catch (error) { result = { exitCode: null, signal: null, error: String(error.message).slice(0, 4096), stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }; }
        let metrics;
        try {
          const streams = helperEvidence(result, id);
          // Always record actual captured lengths/status before any bounded stream write can fail.
          await io.retain(`${id}.process.json`, Buffer.from(canonical({ id, selector: row.id, index,
            role: index === 0 ? 'warmup' : 'measured', call, exitCode: result.exitCode,
            signal: result.signal, error: result.error, stdoutBytes: result.stdout.length, stderrBytes: result.stderr.length,
            helperStreams: Object.fromEntries(streams.map(stream => [stream.name, stream.record])) })));
          for (const stream of streams) await io.retain(stream.record.retainedFile, stream.bytes);
          let raw;
          try { raw = await io.readSample(call.sampleDirectory); }
          catch (error) { raw = { metadata: Buffer.alloc(0), stdout: Buffer.alloc(0), stderr: Buffer.alloc(0), error: String(error.message).slice(0, 4096) }; }
          // Original create-only helper sample remains retained, including files unreadable under a bound.
          await io.retain(`${id}.metadata.raw`, raw.metadata);
          await io.retain(`${id}.stdout`, raw.stdout); await io.retain(`${id}.stderr`, raw.stderr);
          await io.retain(`${id}.sample-read.json`, Buffer.from(canonical({ error: raw.error,
            metadataBytes: raw.metadata.length, stdoutBytes: raw.stdout.length, stderrBytes: raw.stderr.length })));
          assert(streams.every(stream => !stream.record.truncated), 'untruncated helper streams required');
          assert.equal(result.error, null, 'no helper timeout/spawn/overflow/unknown cleanup');
          assert.equal(result.signal, null); assert.equal(result.exitCode, 0, 'helper process successful');
          assert(Buffer.isBuffer(result.stdout) && Buffer.isBuffer(result.stderr));
          assert(result.stdout.length + result.stderr.length <= MAX_LOG);
          assert.equal(result.stdout.length, 0); assert.equal(result.stderr.length, 0);
          assert.equal(raw.error, null, 'complete retained helper files');
          const metadata = await io.parseHelper(call.sampleDirectory, raw.metadata);
          metrics = checkedWindowsMetrics(metadata, capture, call.argv.length - 5, raw.stdout, raw.stderr);
          const response = JSON.parse(raw.stdout.toString('utf8')); canonical(response);
          assert.deepEqual(response, row.expectedResponse, 'original typed source outcome');
          await io.checkArtifactsAndStreams(row, raw.stdout, raw.stderr);
        } finally {
          await io.checkMaterials(); // runs after every attempted command, including writer/parser/validation failures
        }
        await io.afterAttempt(row);
        attempts.push({ id, role: index === 0 ? 'warmup' : 'measured', ...metrics });
      }
      session.cells.push({ selector: row.id, attempts, summary: summarizeWindowsFive(attempts.slice(1)) });
    }
    assert.equal(session.cells.length, 20); await io.finishRound();
    session.status = 'collected-unqualified-observations';
  } catch (error) {
    session.failure = String(error.message).slice(0, 4096);
    await io.retain('session.json', Buffer.from(canonical(session))); throw error;
  }
  await io.retain('session.json', Buffer.from(canonical(session))); return session;
}
