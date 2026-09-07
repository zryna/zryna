// Fixed host-carrier cases; neither these expectations nor sentinel functions produce artifacts.
function scalarCorpus(module, exports, call) {
  const returned = [];
  const rejected = [];
  let sentinelEntries = 0;
  const sentinel = {
    add() { sentinelEntries++; throw new Error('target entered'); },
    identity() { sentinelEntries++; throw new Error('target entered'); },
  };
  const positive = [
    ['add', [20, 22], { type: 'i32', value: 42 }],
    ['add', [2147483647, 1], { type: 'i32', value: -2147483648 }],
    ['add', [-2147483648, 0], { type: 'i32', value: -2147483648 }],
    ['add', [0, 0], { type: 'i32', value: 0 }],
    ['add', [2147483647, 0], { type: 'i32', value: 2147483647 }],
    ['identity', [false], { type: 'bool', value: false }],
    ['identity', [true], { type: 'bool', value: true }],
  ];
  for (let rerun = 0; rerun < 2; rerun++) {
    for (const [name, args, expected] of positive) {
      const actual = call(exports, module, name, args);
      if (actual.type !== expected.type || !Object.is(actual.value, expected.value)) {
        throw new Error('typed scalar result disagrees with fixed oracle');
      }
      returned.push(actual);
    }
  }
  const negatives = [
    ['unknown', 'missing', [], 'ZRYNA-B2101'],
    ['prototype-name', 'toString', [], 'ZRYNA-B2101'],
    ['arity-short', 'add', [20], 'ZRYNA-B2102'],
    ['arity-extra', 'add', [20, 22, 0], 'ZRYNA-B2102'],
    ...[
      ['negative-zero', -0, 'ZRYNA-B2002'], ['fraction', 0.5, 'ZRYNA-B2002'],
      ['nan', NaN, 'ZRYNA-B2002'], ['positive-infinity', Infinity, 'ZRYNA-B2002'],
      ['negative-infinity', -Infinity, 'ZRYNA-B2002'],
      ['too-large', 2147483648, 'ZRYNA-B2002'], ['too-small', -2147483649, 'ZRYNA-B2002'],
      ['bigint', 20n, 'ZRYNA-B2001'], ['string', '20', 'ZRYNA-B2001'],
      ['boxed-number', new Number(20), 'ZRYNA-B2001'], ['bool', true, 'ZRYNA-B2001'],
      ['null', null, 'ZRYNA-B2001'], ['undefined', undefined, 'ZRYNA-B2001'],
    ].flatMap(([label, value, code]) => [
      [`i32-left-${label}`, 'add', [value, 22], code],
      [`i32-right-${label}`, 'add', [20, value], code],
    ]),
    ...[0, 1, null, undefined, {}, 'true', new Boolean(true), 1n, () => true]
      .map((value, index) => [`bool-${index}`, 'identity', [value], 'ZRYNA-B2001']),
  ];
  function rejects(body, code) {
    try { body(); } catch (error) {
      if (error.message === code || error.message.startsWith(`${code}:`)) return;
      throw error;
    }
    throw new Error('malformed scalar carrier was accepted');
  }
  for (const [label, name, args, code] of negatives) {
    rejects(() => call(exports, sentinel, name, args), code);
    rejects(() => call(exports, module, name, args), code);
    if (exports.some(signature => signature.name === name)) {
      rejects(() => module[name](...args), code);
    }
    rejected.push({ label, code });
  }
  if (sentinelEntries !== 0) throw new Error('rejected input reached target entry');
  return { returned, rejected, sentinelEntries, positiveCount: returned.length,
    negativeCount: rejected.length, requiredInterfaces: [] };
}
