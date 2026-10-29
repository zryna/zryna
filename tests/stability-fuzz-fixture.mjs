// Synthetic validator fixtures only; never compiler execution or published evidence.
import { canonical } from '../scripts/stability-gates/input.mjs';
import { loadFuzzRegistry } from '../scripts/stability-gates/fuzz-registry.mjs';
import { originalInputs } from '../scripts/stability-gates/fuzz-witness.mjs';

export const registry = loadFuzzRegistry().registry;
export const encode = records => Buffer.from(records.map(record => canonical(record)).join(''));

export function witnessRecords(campaign) {
  const records = [structuredClone(campaign.header)];
  for (const input of originalInputs(campaign)) {
    records.push(input);
    if (campaign.id === 'lexer-generated') {
      const outcome = { result: { files: [{ id: 0, path: input.path,
        lexemes: [{ kind: 'token:Invalid', start: 0, end: Buffer.byteLength(input.text) }] }], diagnostics: [] } };
      for (const execution of [0, 1]) records.push({ event: 'output', case: input.case, execution,
        outcome: structuredClone(outcome) });
    } else for (const version of [2, 3, 4]) {
      const outcome = { error: { code: 'ZRYNA-F2002', message: 'synthetic rejection', span: null } };
      for (const execution of [0, 1]) records.push({ event: 'output', case: input.case, version, execution,
        outcome: structuredClone(outcome) });
    }
  }
  records.push({ event: 'complete', cases: campaign.cases });
  return records;
}

export function rustLog(gate) {
  const names = gate.test ? [gate.test] : registry.namedBoundaries;
  return Buffer.from(names.map(name => `test tests::${name} ... ok\n`).join('') +
    `test result: ok. ${names.length} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s\n`);
}
