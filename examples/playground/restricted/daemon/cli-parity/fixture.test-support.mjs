// Synthetic parser/recorder controls only: no CLI, provider, component or generated module executes.
import { frame, witSha256 } from '../../../../../tests/playground-fixtures/service.mjs';
import { fixedCorpus, findOperation } from './plan.mjs';
import { sha256 } from './records.mjs';
export { witSha256 };
export const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64),
  policySha256: 'c'.repeat(64), bindingTemplateSha256: 'd'.repeat(64) };
export function diagnostic(entry) {
  return { code: entry.code ?? (entry.provider ? 'TS1001' : 'ZRYNA-M1001'), severity: 'error',
    path: 'src/main.zry', byte_start: 0, byte_end: 1, line_start: 1, column_start: 1,
    line_end: 1, column_end: 2, message: 'Synthetic fixture only', guidance: 'Never actual execution evidence' };
}
export function cliCapture(operation) {
  const entry = fixedCorpus.find(value => value.id === operation.corpusId);
  const value = { version: 1, command: operation.kind, ok: Boolean(entry.cases),
    manifest: entry.cases ? `.zryna/out/parity.${operation.kind}/zryna-manifest-v1.json` : null,
    results: operation.kind === 'run' ? [{ target: 'javascript', outcome: { kind: 'returned',
      value: { type: 'i32', value: operation.expected.value } } }] : [],
    diagnostics: entry.cases ? [] : [((d) => ({ code: d.code, severity: d.severity,
      primary: { kind: 'source', span: { file: 0, start: 0, end: 1 } },
      message: d.message, guidance: d.guidance }))(diagnostic(entry))] };
  return { exitCode: value.ok ? 0 : 3, stdout: Buffer.from(JSON.stringify(value)), stderr: Buffer.alloc(0) };
}
export async function compilation(corpusId, revision = fixedCorpus.findIndex(value => value.id === corpusId) + 1) {
  const entry = fixedCorpus.find(value => value.id === corpusId);
  const input = { version: 1, revision, source: entry.source };
  const exports = entry.cases ? [entry.export, ...(entry.extra ? [entry.extra.export] : [])].map(logical => ({
    logical, component: 'zryna-export-' + Buffer.from(logical).toString('hex'), core: logical,
    arity: logical === entry.extra?.export ? entry.extra.args.length : entry.cases[0].args.length })) : [];
  const result = await frame(input, { exports });
  let bytes = result.bytes;
  if (!entry.cases) {
    Object.assign(result.metadata, { status: 'rejected', identity: null, artifacts: [],
      report: { schema_version: 1, diagnostics: [diagnostic(entry)] } });
    const header = Buffer.from(JSON.stringify(result.metadata)); bytes = Buffer.alloc(4 + header.length);
    bytes.writeUInt32LE(header.length); bytes.set(header, 4);
  }
  return { revision, frameBytes: bytes, receipt: { revision, sourceSha256: sha256(Buffer.from(entry.source)),
    frameSha256: sha256(bytes), compilerSha256: authority.compilerSha256, toolkitSha256: authority.toolkitSha256,
    policySha256: authority.policySha256 } };
}
export function observation(id, componentSha256) {
  const operation = findOperation(id); const revision = fixedCorpus.findIndex(value => value.id === operation.corpusId) + 1;
  return { selection: { revision, sourceSha256: operation.sourceSha256, componentSha256,
    logical: operation.logical, args: [...operation.args] }, observation: { revision,
    logical: operation.logical, componentSha256, value: operation.expected.value } };
}
