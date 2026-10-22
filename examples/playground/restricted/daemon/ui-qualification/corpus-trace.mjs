// Independent host-side inspection of actual service bytes. Never executes acquired artifacts.
import { decodeCompilation, decodeSourceRequest } from '../../protocol.mjs';
import { decodeJson } from '../../json.mjs';
import { verifyReceipt } from '../../executable.mjs';
import { validateSelection, validateObservation } from '../../evaluation.mjs';
import { bounds, exact, fail, hash, integer, sha256 } from './wire.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';

export function createCorpusTrace() {
  let session = null, latest = null, active = null, sessionRequestId = null, cancelRequestId = null;
  const requestIds = new Set();
  const rows = [], evaluations = [];
  function finished() { if (active !== null) fail(); }
  return Object.freeze({
    async observe({ id, name, request, body, status, headers }) {
      if (!['session', 'compile', 'evaluate', 'finish', 'cancel'].includes(name)) return;
      integer(id, bounds.requests);if (requestIds.has(id)) fail();requestIds.add(id);
      if (status !== 200) fail();
      if (name === 'session') {
        if (session !== null) fail();
        const value = decodeJson(body, 98304);
        exact(value, ['version', 'evaluationMode', 'authority', 'witSha256', 'template']);
        exact(value.authority, ['compilerSha256', 'toolkitSha256', 'policySha256', 'bindingTemplateSha256']);
        if (value.version !== 2 || value.evaluationMode !== 'supervised-host' || !hash(value.witSha256) ||
            Object.values(value.authority).some(item => !hash(item)) || typeof value.template !== 'string' ||
            !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value.template) ||
            sha256(Buffer.from(value.template, 'base64')) !== value.authority.bindingTemplateSha256) fail();
        session = { authority: value.authority, witSha256: value.witSha256 };sessionRequestId = id;return;
      }
      if (!session) fail();
      if (name === 'cancel') {
        if (cancelRequestId !== null) fail();
        finished();exact(decodeJson(request, 1024), ['cancelled']);
        if (decodeJson(request, 1024).cancelled !== true) fail();
        exact(decodeJson(body, 1024), ['finished']);if (decodeJson(body, 1024).finished !== true) fail();cancelRequestId = id;return;
      }
      if (name === 'finish') {
        const input = decodeJson(request, 1024), reply = decodeJson(body, 1024);
        exact(input, ['token', 'cancelled']);exact(reply, ['finished']);
        if (!active || input.token !== active.token || input.cancelled !== false || reply.finished !== true) fail();
        active.row.finishId = id;
        active = null;return;
      }
      finished();
      if (name === 'compile') {
        const input = decodeSourceRequest(request), expected = qualificationCases[rows.length];
        if (!expected || input.source !== expected.source || rows.length && input.revision <= rows.at(-1).revision) fail('CORPUS_SOURCE');
        if (rows.length && evaluations.length !== qualificationCases.slice(0, rows.length).reduce((n, row) => n + scalarCases(row).length, 0)) fail();
        const { metadata } = await decodeCompilation(body, { revision: input.revision, source: input.source, witSha256: session.witSha256 });
        const receiptText = headers.find(row => row.name === 'x-zryna-receipt')?.value;
        if (typeof receiptText !== 'string' || receiptText.length > 2048) fail();
        const receipt = decodeJson(Buffer.from(receiptText, 'base64'), 1024);
        verifyReceipt(receipt, metadata, session.authority);
        if (receipt.frameSha256 !== sha256(body)) fail('CORPUS_RECEIPT');
        const token = headers.find(row => row.name === 'x-zryna-job')?.value;
        if (!hash(token)) fail();
        if (expected.cases ? metadata.status !== 'compiled' : metadata.status !== 'rejected') fail('CORPUS_STATUS');
        const exports = [...new Map(scalarCases(expected).map(row => [row.logical, row.args.length]))]
          .map(([logical, arity]) => ({ logical, arity })).sort((a, b) => a.logical.localeCompare(b.logical));
        if (JSON.stringify(metadata.exports.map(({ logical, arity }) => ({ logical, arity }))
          .sort((a, b) => a.logical.localeCompare(b.logical))) !== JSON.stringify(exports)) fail('CORPUS_STATUS');
        if (expected.code && !metadata.report.diagnostics.some(row => row.code === expected.code) ||
            expected.provider && !metadata.report.diagnostics.some(row => row.code.startsWith('TS'))) fail();
        rows.push({ id: expected.id, requestId: id, finishId: null, revision: input.revision, sourceSha256: metadata.sourceSha256,
          frameSha256: receipt.frameSha256, status: metadata.status, report: metadata.report,
          identity: metadata.identity, exports: metadata.exports });
        latest = metadata;active = { token, row: rows.at(-1) };return;
      }
      if (name === 'evaluate') {
        const input = decodeJson(request, 8192), reply = decodeJson(body, 2048);
        const selection = validateSelection(input, latest);
        exact(reply, ['token', 'observation']);if (!hash(reply.token)) fail();
        const observation = validateObservation(reply.observation, selection);
        const expected = qualificationCases.flatMap(row => scalarCases(row).map(value => ({ id: row.id, ...value })))[evaluations.length];
        if (!expected || expected.id !== rows.at(-1)?.id || selection.logical !== expected.logical ||
            JSON.stringify(selection.args) !== JSON.stringify(expected.args) || observation.value !== expected.value) fail('CORPUS_SCALAR');
        evaluations.push({ id: expected.id, requestId: id, finishId: null, ...observation, args: [...selection.args] });
        active = { token: reply.token, row: evaluations.at(-1) };
      }
    },
    snapshot() { return structuredClone({ authority: session?.authority ?? null, witSha256: session?.witSha256 ?? null, sessionRequestId, cancelRequestId,
      compilations: rows, evaluations, cleanupAcknowledged: active === null }); },
    complete() {
      finished();
      if (rows.length !== qualificationCases.length || evaluations.length !== 10) fail();
      return structuredClone({ authority: session.authority, witSha256: session.witSha256, sessionRequestId, cancelRequestId,
        compilations: rows, evaluations, cleanupAcknowledged: true });
    },
  });
}
