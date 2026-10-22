// This original, pretrusted module instance owns publication authority across acquisition/startup.
// No issuer/map/mint function is exported. Receipt dictionaries cannot activate public execution.
const retained = new WeakMap();

function state(capability) {
  const value = retained.get(capability);
  if (!value) throw new Error('PLAYGROUND-COMPILER-PUBLICATION-CAPABILITY');
  return value;
}

function frozenCopy(value) {
  const copy = structuredClone(value);
  function freeze(entry) {
    if (entry && typeof entry === 'object') {
      Object.values(entry).forEach(freeze);
      Object.freeze(entry);
    }
  }
  freeze(copy);
  return copy;
}

function record(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) {
    throw new Error('PLAYGROUND-COMPILER-PUBLICATION-INPUT');
  }
  return Object.fromEntries(keys.map(key => [key, value[key]]));
}

function snapshot(input) {
  const own = record(input, ['envelope', 'bundle', 'archive', 'policy', 'verifierPolicy', 'trustedRoot']);
  own.policy = record(own.policy, ['version', 'source', 'envelopeSha256', 'archive', 'binary', 'provenanceSha256']);
  own.policy.source = record(own.policy.source,
    ['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch', 'tagObject', 'tagType']);
  own.policy.archive = record(own.policy.archive, ['path', 'bytes', 'sha256']);
  own.policy.binary = record(own.policy.binary, ['path', 'bytes', 'sha256']);
  own.verifierPolicy = record(own.verifierPolicy,
    ['version', 'path', 'bytes', 'sha256', 'ownerUid', 'trustedRootBytes', 'trustedRootSha256']);
  for (const [name, maximum] of [['envelope', 262144], ['bundle', 1048576],
    ['archive', 513 * 1024 * 1024], ['trustedRoot', 262144]]) {
    if (!(own[name] instanceof Uint8Array) || own[name].length < 1 || own[name].length > maximum) {
      throw new Error('PLAYGROUND-COMPILER-PUBLICATION-CARRIER');
    }
  }
  if (own.archive.length !== own.policy.archive.bytes ||
      own.trustedRoot.length !== own.verifierPolicy.trustedRootBytes) {
    throw new Error('PLAYGROUND-COMPILER-PUBLICATION-CARRIER-SIZE');
  }
  for (const name of ['envelope', 'bundle', 'archive', 'trustedRoot']) own[name] = new Uint8Array(own[name]);
  return own;
}

export async function authenticateCompilerPublication(input) {
  const owned = snapshot(input);
  // A mere getter import has no verifier/archive imports. The fixed verifier is never injectable.
  const { verifyCompilerPublication } = await import('./compiler-publication-verify.mjs');
  const verified = await verifyCompilerPublication(owned);
  const capability = Object.freeze({});
  retained.set(capability, verified);
  return capability;
}

export function compilerPublicationReceipt(capability) {
  return frozenCopy(state(capability).receipt);
}

export function compilerPublicationBinary(capability) {
  return new Uint8Array(state(capability).binary);
}

export function compilerPublicationFiles(capability) {
  return state(capability).files.map(file => ({ path: file.path, mode: file.mode,
    data: new Uint8Array(file.data) }));
}

export function compilerPublicationVerification(capability) {
  const verification = state(capability).verification;
  return { command: frozenCopy(verification.command), result: frozenCopy(verification.result),
    stdout: new Uint8Array(verification.stdout), stderr: new Uint8Array(verification.stderr) };
}
