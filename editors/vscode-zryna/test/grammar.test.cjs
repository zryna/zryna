'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const grammar = require('../syntaxes/zryna.tmLanguage.json');
const manifest = require('../package.json');

test('registered grammar has valid patterns for comments, strings and scalar declarations', () => {
  assert.equal(manifest.contributes.grammars[0].scopeName, grammar.scopeName);
  for (const rule of grammar.patterns) {
    for (const key of ['match', 'begin', 'end']) if (rule[key]) assert.doesNotThrow(() => new RegExp(rule[key]));
  }
  const samples = { 'keyword.control.zryna': 'export function return', 'storage.type.zryna': 'i32',
    'constant.numeric.zryna': '42', 'entity.name.function.zryna': 'sum(', 'keyword.operator.zryna': '+' };
  for (const [scope, sample] of Object.entries(samples)) {
    assert.ok(new RegExp(grammar.patterns.find(rule => rule.name === scope).match).test(sample));
  }
  assert.equal(grammar.patterns[0].name, 'comment.line.double-slash.zryna');
  assert.equal(grammar.patterns[1].name, 'comment.block.zryna');
});
