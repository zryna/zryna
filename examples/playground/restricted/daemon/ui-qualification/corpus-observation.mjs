// Browser DOM is evidence only after exact comparison with authenticated service responses.
import { exact, fail } from './wire.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';

const same = (left, right) => { if (JSON.stringify(left) !== JSON.stringify(right)) fail(); };
export function diagnosticChildren(record) {
  const location = record.byte_start === null ? 'No source span' :
    `${record.path}:${record.line_start}:${record.column_start}–${record.line_end}:${record.column_end} (UTF-8 bytes ${record.byte_start}–${record.byte_end})`;
  return [{ tag: 'P', text: `${record.severity}: ${record.code} — ${record.message}` },
    { tag: record.byte_start === null ? 'P' : 'BUTTON', text: location },
    ...(record.guidance ? [{ tag: 'P', text: record.guidance }] : [])];
}
function ax(nodes) {
  if (!Array.isArray(nodes) || !nodes.length || nodes.length > 512) fail();
  for (const node of nodes) {
    exact(node, ['role', 'name', 'ignored', 'properties']);
    if (typeof node.role !== 'string' || node.role.length > 128 || typeof node.name !== 'string' ||
        node.name.length > 4096 || typeof node.ignored !== 'boolean' || !Array.isArray(node.properties) || node.properties.length > 4) fail();
    for (const property of node.properties) {
      exact(property, ['name', 'value']);
      if (!['focused', 'live', 'invalid', 'disabled'].includes(property.name) ||
          !['string', 'boolean'].includes(typeof property.value) && property.value !== null) fail();
    }
  }
}
const accessible = (nodes, role, name) => nodes.find(row => !row.ignored && row.role === role && row.name === name);
const property = (node, name, expected) => node?.properties.some(row => row.name === name && row.value === expected);
export function inspectCorpus(value, actual) {
  exact(value, ['format', 'version', 'rows', 'keyboard', 'invalidArgument', 'revisionInvalidation', 'sourceBudget',
    'recovered', 'unicodeNavigation', 'argumentAX', 'invalidAX', 'diagnosticAX', 'inertSource', 'sessionIsolation', 'arbitraryInertMarkup', 'audibleAssistiveTechnology']);
  if (value.format !== 'zryna.ui-corpus-observation.v1' || value.version !== 1 ||
      value.sessionIsolation !== 'unrun' || value.arbitraryInertMarkup !== 'unrun' || value.audibleAssistiveTechnology !== 'unrun' ||
      !Array.isArray(value.rows) || value.rows.length !== qualificationCases.length || !actual.cleanupAcknowledged) fail();
  let evaluation = 0;
  for (const [index, expected] of qualificationCases.entries()) {
    const row = value.rows[index], compiled = actual.compilations[index];
    exact(row, ['id', 'source', 'status', 'identity', 'runDisabled', 'exports', 'diagnostics', 'unexpectedDiagnosticElements', 'observations']);
    if (row.id !== expected.id || row.source !== expected.source || compiled.id !== expected.id ||
        row.unexpectedDiagnosticElements !== 0 || row.runDisabled !== !expected.cases) fail();
    same(row.status, expected.cases ? 'Compiled. Choose an export and run it.' : 'Compilation rejected. Review the compiler diagnostics.');
    same(row.exports, compiled.exports.map(entry => ({ value: entry.logical, text: `${entry.logical} (${entry.arity} arguments)` })));
    same(row.diagnostics, compiled.report.diagnostics.map(diagnosticChildren));
    const identity = expected.cases ? `Source revision ${compiled.revision}; source SHA-256 ${compiled.sourceSha256}; component SHA-256 ${compiled.identity.componentSha256}` : '';
    same(row.identity, identity);
    const cases = scalarCases(expected);
    if (!Array.isArray(row.observations) || row.observations.length !== cases.length) fail();
    for (const [number, oracle] of cases.entries()) {
      const observed = row.observations[number], response = actual.evaluations[evaluation++];
      exact(observed, ['logical', 'args', 'value', 'text', 'identity']);
      same({ logical: observed.logical, args: observed.args, value: observed.value }, oracle);
      if (response.id !== expected.id || response.logical !== oracle.logical || response.value !== oracle.value ||
          response.revision !== compiled.revision || response.componentSha256 !== compiled.identity.componentSha256) fail();
      same(response.args, oracle.args);
      same(observed.text, `${oracle.logical}(${oracle.args.join(', ')}) = ${oracle.value} (signed i32)`);
      same(observed.identity, `Source revision ${compiled.revision}; source SHA-256 ${compiled.sourceSha256}; observed component SHA-256 ${compiled.identity.componentSha256}`);
    }
  }
  same(value.keyboard, { tabFromSource: 'compile' });
  same(value.invalidArgument, { focusedId: 'argument-0', invalid: 'true',
    error: 'Enter a whole decimal number from -2147483648 to 2147483647.',
    observation: 'Check the highlighted argument values before running.' });
  same(value.revisionInvalidation, { runDisabled: true, exports: 0, identity: '', observation: 'No observation.' });
  same(value.sourceBudget, { bytes: '4098 / 4096 UTF-8 bytes', invalid: 'true',
    error: 'Source exceeds 4096 UTF-8 bytes. Remove 2 bytes before compiling.', compileDisabled: true, runDisabled: true });
  same(value.recovered, { invalid: 'false', error: '', compileDisabled: false, runDisabled: true, identity: '' });
  exact(value.inertSource, ['sourceExact', 'imgElements', 'scriptElementsAfter', 'markerAbsent', 'scriptElementsBefore']);
  if (value.inertSource.sourceExact !== true || value.inertSource.imgElements !== 0 || value.inertSource.markerAbsent !== true ||
      value.inertSource.scriptElementsBefore !== 1 || value.inertSource.scriptElementsAfter !== 1) fail('CORPUS_DOM');
  exact(value.unicodeNavigation, ['diagnosticIndex', 'focusedId', 'start', 'end']);
  const unicode = qualificationCases.findIndex(row => row.id === 'unicode-navigation');
  const source = qualificationCases[unicode].source, diagnostics = actual.compilations[unicode].report.diagnostics;
  const index = diagnostics.findIndex(row => row.byte_start !== null);
  if (index === -1 || value.unicodeNavigation.diagnosticIndex !== index || value.unicodeNavigation.focusedId !== 'source') fail();
  const bytes = Buffer.from(source), record = diagnostics[index], decoder = new TextDecoder('utf-8', { fatal: true });
  if (record.byte_start <= Buffer.byteLength('// 😀 é\n') ||
      value.unicodeNavigation.start !== decoder.decode(bytes.subarray(0, record.byte_start)).length ||
      value.unicodeNavigation.end !== decoder.decode(bytes.subarray(0, record.byte_end)).length ||
      value.unicodeNavigation.start === record.byte_start) fail();
  for (const nodes of [value.argumentAX, value.invalidAX, value.diagnosticAX]) ax(nodes);
  if (!accessible(value.argumentAX, 'textbox', 'Argument 1 (i32)') || !accessible(value.argumentAX, 'textbox', 'Argument 2 (i32)') ||
      !property(accessible(value.invalidAX, 'textbox', 'Argument 1 (i32)'), 'focused', true) ||
      !property(accessible(value.invalidAX, 'textbox', 'Argument 1 (i32)'), 'invalid', 'true') ||
      !property(accessible(value.diagnosticAX, 'textbox', 'Source'), 'focused', true) ||
      !value.diagnosticAX.some(node => !node.ignored && node.role === 'status' && property(node, 'live', 'polite'))) fail();
  return structuredClone(value);
}
