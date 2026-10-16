import { checkedText, exact, fail, freeze, jsonOutput, sha256 } from './records.mjs';
import { findOperation, fixedCorpus } from './plan.mjs';

function diagnostic(value, source) {
  exact(value, ['code', 'severity', 'primary', 'message', 'guidance']);
  if (!/^(?:ZRYNA-[A-Z][0-9]{4}|TS[0-9]{1,10})$/.test(value.code) ||
      !['error', 'warning'].includes(value.severity)) fail();
  checkedText(value.message); checkedText(value.guidance);
  let path = null; let byteStart = null; let byteEnd = null;
  if (value.primary?.kind === 'global') exact(value.primary, ['kind']);
  else if (value.primary?.kind === 'workspace-path') {
    exact(value.primary, ['kind', 'path']);
    if (value.primary.path !== 'src/main.zry') fail();
    path = value.primary.path;
  } else if (value.primary?.kind === 'source') {
    exact(value.primary, ['kind', 'span']); exact(value.primary.span, ['file', 'start', 'end']);
    const { file, start, end } = value.primary.span;
    const bytes = Buffer.from(source);
    if (file !== 0 || !Number.isSafeInteger(start) || !Number.isSafeInteger(end) ||
        start < 0 || end < start || end > bytes.length) fail();
    const decoder = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
    try { decoder.decode(bytes.subarray(0, start)); decoder.decode(bytes.subarray(0, end)); }
    catch { fail(); }
    path = 'src/main.zry'; byteStart = start; byteEnd = end;
  } else fail();
  return { code: value.code, severity: value.severity, path, byte_start: byteStart,
    byte_end: byteEnd, message: value.message, guidance: value.guidance };
}
export function decodeCliOutput(id, capture) {
  const operation = findOperation(id);
  const entry = fixedCorpus.find(value => value.id === operation.corpusId);
  exact(capture, ['exitCode', 'stdout', 'stderr']);
  if (!(capture.stderr instanceof Uint8Array) || capture.stderr.length !== 0 ||
      ![0, 3].includes(capture.exitCode) || !(capture.stdout instanceof Uint8Array) ||
      capture.stdout.length < 1 || capture.stdout.length > 65536) fail();
  const stdout = new Uint8Array(capture.stdout);
  const value = jsonOutput(stdout);
  exact(value, ['version', 'ok', 'command', 'manifest', 'results', 'diagnostics']);
  if (value.version !== 1 || typeof value.ok !== 'boolean' || value.command !== operation.kind ||
      !Array.isArray(value.results) || !Array.isArray(value.diagnostics) || value.diagnostics.length > 256) fail();
  const diagnostics = value.diagnostics.map(item => diagnostic(item, entry.source));
  let returned = null;
  if (value.ok) {
    if (capture.exitCode !== 0 || value.manifest !== `.zryna/out/parity.${operation.kind}/zryna-manifest-v1.json` ||
        diagnostics.some(item => item.severity === 'error')) fail();
    if (operation.kind === 'build') { if (value.results.length) fail(); }
    else {
      if (value.results.length !== 1) fail();
      const item = value.results[0]; exact(item, ['target', 'outcome']);
      exact(item.outcome, ['kind', 'value']); exact(item.outcome.value, ['type', 'value']);
      returned = item.outcome.value.value;
      if (item.target !== 'javascript' || item.outcome.kind !== 'returned' || item.outcome.value.type !== 'i32' ||
          !Number.isInteger(returned) || returned < -2147483648 || returned > 2147483647 || Object.is(returned, -0)) fail();
    }
  } else if (capture.exitCode !== 3 || value.manifest !== null || value.results.length ||
      !diagnostics.some(item => item.severity === 'error')) fail();
  const status = value.ok ? 'compiled' : 'rejected';
  if (status !== operation.expected.status || operation.kind === 'run' && returned !== operation.expected.value ||
      operation.expected.code !== null && !diagnostics.some(item => item.code === operation.expected.code) ||
      entry.provider && !diagnostics.some(item => /^TS[0-9]+$/.test(item.code))) fail();
  return freeze({ operationId: id, corpusId: operation.corpusId, sourceSha256: operation.sourceSha256,
    status, value: returned, diagnostics, capture: { exitCode: capture.exitCode,
      stdoutBytes: stdout.length, stdoutSha256: sha256(stdout), stderrBytes: 0, stderrSha256: sha256(Buffer.alloc(0)) } });
}
