import { createHash } from 'node:crypto';

const hash = chunks => createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
const count = value => {
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(BigInt(value));
  return bytes;
};
const field = value => {
  const bytes = Buffer.from(value);
  return Buffer.concat([count(bytes.length), bytes]);
};
const scalar = value => {
  if (value === 'bool') return Buffer.from([1]);
  if (value === 'i32') return Buffer.from([2]);
  throw new Error('unknown scalar signature type');
};

// Independent canonical binding check at the test transport boundary, before either host import.
export function verifyFixtureBinding(input, host) {
  if (input.host !== host || !['js-node', 'js-browser'].includes(host) ||
      !Array.isArray(input.requiredInterfaces) || input.requiredInterfaces.length !== 0 ||
      typeof input.source !== 'string' || Buffer.byteLength(input.source) > 1024 * 1024 ||
      !Array.isArray(input.exports) || input.exports.length > 16384) {
    throw new Error('invalid pure scalar fixture');
  }
  for (const digest of ['graphSha256', 'artifactSha256', 'interfaceSha256', 'bindingSha256']) {
    if (!/^[a-f0-9]{64}$/.test(input[digest])) throw new Error('invalid fixture digest');
  }
  if (hash([Buffer.from(input.source)]) !== input.artifactSha256) throw new Error('artifact changed');
  const interfaceChunks = [Buffer.from('ZRYNA-SCALAR-ADAPTER-INTERFACE\0'),
    field('zryna.scalar-adapter-interface.v1'), field('zryna-control-flow-v1'),
    field('javascript-esm'), field(host), Buffer.from([1, 0]), count(input.exports.length)];
  const names = new Set();
  for (const signature of input.exports) {
    if (!/^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(signature.name) ||
        names.has(signature.name.toLowerCase()) || !Array.isArray(signature.parameters) ||
        signature.parameters.length > 256) throw new Error('invalid fixture export');
    names.add(signature.name.toLowerCase());
    interfaceChunks.push(field(signature.name), count(signature.parameters.length),
      ...signature.parameters.map(scalar), scalar(signature.result));
  }
  if (hash(interfaceChunks) !== input.interfaceSha256) throw new Error('interface changed');
  const binding = hash([Buffer.from('ZRYNA-SCALAR-ADAPTER-BINDING\0'),
    Buffer.from(input.interfaceSha256, 'hex'), Buffer.from(input.graphSha256, 'hex'),
    Buffer.from(input.artifactSha256, 'hex'), count(Buffer.byteLength(input.source))]);
  if (binding !== input.bindingSha256) throw new Error('binding changed');
}
