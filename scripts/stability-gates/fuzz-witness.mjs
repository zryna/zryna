import assert from 'node:assert/strict';
import { exact, MAX_LOG, parse } from './input.mjs';
import { acceptedDTO } from './fuzz-dto.mjs';
import { range as span } from './fuzz-ranges.mjs';

export function* originalInputs(campaign) {
  let state = BigInt(campaign.seed);
  const mask = (1n << BigInt(campaign.wordBits)) - 1n;
  for (let index = 0; index < campaign.cases; index += 1) {
    let text = campaign.id === 'lexer-generated' ? '' : '// π\r\n';
    const count = campaign.id === 'lexer-generated' ? 32 : 4 + index % 47;
    for (let fragment = 0; fragment < count; fragment += 1) {
      state = (state * BigInt(campaign.multiplier) + BigInt(campaign.increment)) & mask;
      text += campaign.corpus[Number(state % BigInt(campaign.corpus.length))];
      if (campaign.id !== 'lexer-generated') text += ' ';
    }
    yield { event: 'input', case: index, path: 'src/main.zry', text };
  }
}

function lexical(outcome, text) {
  exact(outcome, ['result'], 'completed lexical result');
  exact(outcome.result, ['files', 'diagnostics'], 'lexical project');
  assert(Array.isArray(outcome.result.files) && outcome.result.files.length === 1);
  const file = outcome.result.files[0];
  exact(file, ['id', 'path', 'lexemes'], 'lexical file');
  assert.equal(file.id, 0);
  assert.equal(file.path, 'src/main.zry');
  const value = { lexemes: file.lexemes, diagnostics: outcome.result.diagnostics };
  assert(Array.isArray(value.lexemes) && Array.isArray(value.diagnostics));
  assert(value.lexemes.length <= 65536 && value.diagnostics.length <= 256);
  let end = 0;
  const slices = [];
  const bytes = Buffer.from(text);
  for (const lexeme of value.lexemes) {
    exact(lexeme, ['kind', 'start', 'end'], 'lexeme');
    assert.match(lexeme.kind, /^(token|trivia):[A-Za-z][A-Za-z0-9]*(?:\([A-Za-z]+\))?$/);
    span({ start: lexeme.start, end: lexeme.end }, text, false);
    assert.equal(lexeme.start, end, 'lossless ordering');
    assert(lexeme.end > end, 'nonempty lexeme');
    slices.push(bytes.subarray(end, lexeme.end));
    end = lexeme.end;
  }
  assert.equal(end, bytes.length);
  assert.deepEqual(Buffer.concat(slices), bytes);
  for (const diagnostic of value.diagnostics) {
    exact(diagnostic, ['code', 'message', 'span', 'severity', 'guidance'], 'lexical diagnostic');
    assert(['error', 'warning'].includes(diagnostic.severity));
    assert.equal(typeof diagnostic.guidance, 'string');
    assert.match(diagnostic.code, /^ZRYNA-[A-Z][0-9]{4}$/);
    assert.equal(typeof diagnostic.message, 'string');
    assert(diagnostic.span !== null, 'source-bound lexical diagnostic');
    span(diagnostic.span, text, true);
  }
  return { lexemes: value.lexemes.length, diagnostics: value.diagnostics.length };
}

function parser(value, input, version) {
  if (Object.hasOwn(value, 'result')) {
    exact(value, ['result'], 'accepted raw candidate');
    acceptedDTO(value.result, input, version);
    return 'accepted';
  }
  exact(value, ['error'], 'rejected raw candidate');
  exact(value.error, ['code', 'message', 'span'], 'parser rejection');
  assert.match(value.error.code, /^ZRYNA-[A-Z][0-9]{4}$/);
  assert.equal(typeof value.error.message, 'string');
  if (value.error.span !== null) span(value.error.span, input.text, true);
  return 'rejected';
}

export function replayWitness(bytes, campaign) {
  assert(bytes.length > 0 && bytes.length <= MAX_LOG);
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  assert(text.endsWith('\n'), 'complete JSONL line');
  const lines = text.slice(0, -1).split('\n');
  const versions = campaign.id === 'lexer-generated' ? [null] : [2, 3, 4];
  assert.equal(campaign.eventCounts.total, 2 + campaign.cases * (1 + 2 * versions.length));
  assert.equal(lines.length, campaign.eventCounts.total);
  let cursor = 0;
  const next = () => parse(Buffer.from(`${lines[cursor++]}\n`));
  assert.deepEqual(next(), campaign.header);
  const totals = { inputs: 0, outputs: 0, lexemes: 0, diagnostics: 0, versions: {} };
  for (const input of originalInputs(campaign)) {
    assert.deepEqual(next(), input, 'retained original generated input');
    totals.inputs += 1;
    for (const version of versions) {
      const outcomes = [];
      for (let execution = 0; execution < 2; execution += 1) {
        const output = next();
        exact(output, version === null ? ['event', 'case', 'execution', 'outcome'] :
          ['event', 'case', 'version', 'execution', 'outcome'], 'completed execution');
        assert.equal(output.event, 'output');
        assert.equal(output.case, input.case);
        assert.equal(output.execution, execution);
        if (version !== null) assert.equal(output.version, version);
        outcomes.push(output.outcome);
        totals.outputs += 1;
      }
      assert.deepEqual(outcomes[0], outcomes[1], 'actual deterministic outputs');
      if (version === null) {
        const count = lexical(outcomes[0], input.text);
        totals.lexemes += count.lexemes;
        totals.diagnostics += count.diagnostics;
      } else {
        const outcome = parser(outcomes[0], input, version);
        totals.versions[version] ??= { accepted: 0, rejected: 0 };
        totals.versions[version][outcome] += 1;
      }
    }
  }
  assert.deepEqual(next(), { event: 'complete', cases: campaign.cases });
  assert.equal(cursor, lines.length);
  return totals;
}
