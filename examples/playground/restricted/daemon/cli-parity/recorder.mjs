import { decodeCompilation } from '../../protocol.mjs';
import { verifyReceipt } from '../../executable.mjs';
import { validateObservation, validateSelection } from '../../evaluation.mjs';
import { canonical, exact, fail, freeze, hash, sha256 } from './records.mjs';
import { createParityPlan, findOperation, fixedCorpus, parityPlanSha256 } from './plan.mjs';
import { decodeCliOutput } from './cli-output.mjs';
import { limits } from '../../limits.mjs';

export function createParityRecorder({ authority, witSha256 }) {
  exact(authority, ['compilerSha256', 'toolkitSha256', 'policySha256', 'bindingTemplateSha256']);
  if (Object.values(authority).some(value => !hash(value)) || !hash(witSha256)) fail();
  const trustedAuthority = Object.freeze({ ...authority });
  const cli = new Map(); const compiled = new Map(); const observations = new Map(); const pending = new Set();
  let terminal = false;
  const open = () => { if (terminal) fail(); };
  return Object.freeze({
    cli(id, capture) {
      open(); if (cli.has(id)) fail();
      cli.set(id, decodeCliOutput(id, capture));
    },
    async compilation(corpusId, capture) {
      open();
      const entry = fixedCorpus.find(value => value.id === corpusId);
      if (!entry || compiled.has(corpusId) || pending.has(corpusId)) fail();
      exact(capture, ['revision', 'frameBytes', 'receipt']);
      const revision = fixedCorpus.findIndex(value => value.id === corpusId) + 1;
      if (capture.revision !== revision || !(capture.frameBytes instanceof Uint8Array) ||
          capture.frameBytes.length < 4 || capture.frameBytes.length > limits.frame) fail();
      exact(capture.receipt, ['revision', 'sourceSha256', 'frameSha256', 'compilerSha256', 'toolkitSha256', 'policySha256']);
      if (capture.receipt.revision !== revision || Object.entries(capture.receipt)
        .some(([key, value]) => key !== 'revision' && !hash(value))) fail();
      const frame = new Uint8Array(capture.frameBytes); const receipt = { ...capture.receipt };
      pending.add(corpusId);
      try {
        const result = await decodeCompilation(frame, { revision: capture.revision, source: entry.source, witSha256 });
        verifyReceipt(receipt, result.metadata, trustedAuthority);
        if (sha256(frame) !== receipt.frameSha256 ||
            result.metadata.status !== (entry.cases ? 'compiled' : 'rejected') ||
            entry.code && !result.metadata.report.diagnostics.some(value => value.code === entry.code) ||
            entry.provider && !result.metadata.report.diagnostics.some(value => /^TS[0-9]+$/.test(value.code))) fail();
        open();
        const diagnostics = result.metadata.report.diagnostics.map(value => ({ code: value.code, severity: value.severity,
          path: value.path, byte_start: value.byte_start, byte_end: value.byte_end, message: value.message, guidance: value.guidance }));
        compiled.set(corpusId, freeze({ metadata: result.metadata, diagnostics,
          frameSha256: receipt.frameSha256, frameBytes: frame.length }));
      } finally { pending.delete(corpusId); }
    },
    abort() { terminal = true; },
    observation(id, capture) {
      open();
      const operation = findOperation(id);
      const compilation = compiled.get(operation.corpusId);
      if (operation.kind !== 'run' || observations.has(id) || !compilation) fail();
      exact(capture, ['selection', 'observation']);
      const selection = validateSelection(capture.selection, compilation.metadata);
      if (selection.logical !== operation.logical || canonical(selection.args) !== canonical(operation.args)) fail();
      const observed = validateObservation(capture.observation, selection);
      if (observed.value !== operation.expected.value) fail();
      observations.set(id, freeze({ ...observed, selection }));
    },
    finish() {
      open();
      if (pending.size || cli.size !== 19 || compiled.size !== 9 || observations.size !== 10) fail();
      const rows = createParityPlan().operations.map(operation => {
        const command = cli.get(operation.id); const compilation = compiled.get(operation.corpusId);
        const observation = observations.get(operation.id);
        if (!command || !compilation || command.status !== compilation.metadata.status ||
            canonical(command.diagnostics) !== canonical(compilation.diagnostics) ||
            operation.kind === 'run' && (!observation || command.value !== observation.value)) fail();
        return { operationId: operation.id, corpusId: operation.corpusId, sourceSha256: operation.sourceSha256,
          kind: operation.kind, status: command.status, expectedValue: operation.expected.value,
          cliValue: command.value, playgroundValue: observation?.value ?? null,
          logical: operation.logical, args: operation.args, diagnostics: command.diagnostics,
          cliCapture: command.capture, playgroundFrameSha256: compilation.frameSha256,
          playgroundComponentSha256: observation?.componentSha256 ?? compilation.metadata.identity?.componentSha256 ?? null };
      });
      terminal = true;
      return freeze({ format: 'zryna.playground-cli-parity-comparison.v1', version: 1,
        planSha256: parityPlanSha256(), comparisonStatus: 'matched', comparedOperations: 19,
        acceptedSourceCases: 4, rejectedSourceCases: 5, comparedScalarObservations: 10,
        authority: trustedAuthority, witSha256, rows,
        // Bytes and callbacks alone cannot authenticate execution, cleanup, or publication.
        acceptance: false, executionEvidenceRequired: true, source: 'caller-captured-streams',
        scope: 'comparison-only-no-runtime-or-publication-attestation' });
    },
  });
}
