// Private scalar boundary shared by the Node consumer and real-browser conformance harness.
function scalarCall(exports, module, name, args) {
  const signature = exports.find(candidate => candidate.name === name);
  if (!signature) throw new TypeError('ZRYNA-B2101');
  if (args.length !== signature.parameters.length) throw new TypeError('ZRYNA-B2102');
  function check(type, value) {
    if (type === 'bool') {
      if (typeof value !== 'boolean') throw new TypeError('ZRYNA-B2001');
    } else if (type === 'i32') {
      if (typeof value !== 'number') throw new TypeError('ZRYNA-B2001');
      if (value !== (value | 0) || Object.is(value, -0)) throw new RangeError('ZRYNA-B2002');
    } else {
      throw new TypeError('ZRYNA-D3871');
    }
  }
  for (let index = 0; index < args.length; index++) check(signature.parameters[index], args[index]);
  const invoke = module[name];
  if (typeof invoke !== 'function') throw new TypeError('ZRYNA-D3871');
  const value = invoke(...args);
  check(signature.result, value);
  return { type: signature.result, value };
}
