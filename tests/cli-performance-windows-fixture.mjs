// Synthetic controller IO only; no child, helper, compiler, guest or measurement execution.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { canonical, MAX_LOG, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
export const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
export const rows = matrixFor(document, 'windows-x86_64');
export function metadata(stdout, stderr, childArgc, value = 3) {
  const start = 9007199254740993000n;
  return { format: 'zryna.stability.windows-job-metrics.v1', metric: 'job-peak-committed-bytes.v1',
    measurementStatus: 'complete', timeoutMs: 1000, cleanupReserveMs: 10000, combinedStreamLimitBytes: MAX_LOG,
    childArgc, commandUtf16Units: 123, childCreated: true, childPid: 1234, mainExitKnown: true, mainExitCode: '0',
    stdoutBytes: stdout.length, stderrBytes: stderr.length, stdoutEof: true, stderrEof: true,
    jobEmptyConfirmed: true, forcedTermination: false, cleanupConfirmed: true, peakQuerySucceeded: true,
    peakJobMemoryUsedBytes: String(value), qpcFrequency: '1000000000', qpcStart: String(start),
    qpcEnd: String(start + BigInt(value)), wallTicks: String(value),
    primaryStage: 0, primaryWin32Error: 0, cleanupStage: 0, cleanupWin32Error: 0 };
}
export function fixture(change = () => {}) {
  const tool = name => ({ path: `C:\\synthetic\\${name}`, bytes: 1, sha256: '0'.repeat(64) });
  const capture = { format: 'zryna.cli-performance-windows-capture.draft.v1',
    catalogueSha256: sha256(canonical(document)), round: 'baseline-round-1', cliBuildClass: 'release',
    sourceCommit: '0'.repeat(40), sourceTree: '1'.repeat(40), host: { platform: 'windows-x86_64', synthetic: true },
    tools: { helper: tool('helper.exe'), cli: tool('zryna.exe'), node: tool('node.exe') },
    operatorAdmission: tool('operator.json'), hostWallGuardMs: 1000, cleanupGuardMs: 10000,
    combinedOutputBytes: MAX_LOG, artifactOracleSha256: '2'.repeat(64), reviewRecordSha256: '3'.repeat(64), environment: {} };
  const saved = new Map(), events = []; let calls = 0, pending;
  const io = { root: 'C:\\synthetic\\source', output: 'C:\\synthetic\\capture',
    retain: (name, bytes) => { assert(!saved.has(name)); saved.set(name, Buffer.from(bytes)); events.push(`retain:${name}`); },
    checkMaterials: () => events.push('material'), beforeAttempt: row => events.push(`before:${row.id}`),
    checkArtifactsAndStreams: row => events.push(`artifacts:${row.id}`), afterAttempt: row => events.push(`after:${row.id}`),
    finishRound: () => events.push('finishRound'),
    parseHelper: (_directory, bytes) => { events.push('parseHelper'); return JSON.parse(bytes); },
    readSample: () => { events.push('readSample'); assert(pending, 'synthetic missing helper files'); return pending; },
    execute: async (file, argv, guard) => {
      const index = calls++, row = rows[Math.floor(index / 6)]; events.push('execute');
      assert.equal(file, capture.tools.helper.path); assert.equal(guard, 12000);
      const result = { exitCode: 0, signal: null, error: null, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
      const raw = { stdout: Buffer.from(JSON.stringify(row.expectedResponse)), stderr: Buffer.alloc(0), error: null };
      const h = metadata(raw.stdout, raw.stderr, argv.length - 5, [1000, 5, 1, 3, 2, 4][index % 6]);
      const context = { result, raw, metadata: h, index, row, argv, capture, events };
      change(context); raw.metadata ??= Buffer.from(`${JSON.stringify(h)}\n`); pending = raw; return result;
    } };
  return { capture, io, saved, events, calls: () => calls };
}
