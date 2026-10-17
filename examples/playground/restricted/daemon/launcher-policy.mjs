// Pretrusted bootstrap policy: matching adjacent metadata never establishes operator approval.
import { createHash } from 'node:crypto';
import { isAbsolute, posix } from 'node:path';

export const prefix = 'examples/playground/restricted/';
export const uiNames = Object.freeze(['client.mjs', 'controller.mjs', 'corpus.mjs', 'evaluation.mjs',
  'executable.mjs', 'index.html', 'json.mjs', 'limits.mjs', 'main.mjs', 'presentation.mjs',
  'protocol.mjs', 'report.mjs', 'style.css', 'worker.mjs']);
export const sourcePaths = Object.freeze([...uiNames.map(name => prefix + name), prefix + 'service.mjs',
  ...['bridge.mjs', 'bridge-material.mjs', 'compiler-publication-capability.mjs', 'launcher.mjs', 'launcher-adapter.mjs', 'launcher-capture.mjs',
    'launcher-core.mjs', 'launcher-policy.mjs', 'launcher-snapshot.mjs'].map(name => prefix + 'daemon/' + name)].sort());
export const fail = code => { throw new Error(`PLAYGROUND-${code}`); };
export const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export function exact(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) fail('SOURCE-POLICY');
}
export function descriptor(value, maximum) {
  exact(value, ['path', 'bytes', 'sha256', 'ownerUid']);
  if (typeof value.path !== 'string' || !isAbsolute(value.path) || value.path.length > 4096 ||
      posix.normalize(value.path) !== value.path || !/^\/[A-Za-z0-9._/-]+$/.test(value.path) ||
      value.path.split('/').length > 64 || !Number.isSafeInteger(value.bytes) || value.bytes < 1 ||
      value.bytes > maximum || !hash(value.sha256) || !Number.isInteger(value.ownerUid) ||
      value.ownerUid < 0 || value.ownerUid > 4294967295) fail('SOURCE-POLICY');
}
export function closedJson(bytes, maximum) {
  if (!(bytes instanceof Uint8Array) || bytes.length < 1 || bytes.length > maximum) fail('SOURCE-POLICY');
  let text; let value;
  try { text = new TextDecoder('utf-8', { fatal: true }).decode(bytes); value = JSON.parse(text); }
  catch { fail('SOURCE-POLICY'); }
  let compact = ''; let quoted = false; let escaped = false;
  for (const character of text) {
    if (quoted) {
      compact += character;
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
    } else if (character === '"') { quoted = true; compact += character; }
    else if (![' ', '\t', '\r', '\n'].includes(character)) compact += character;
  }
  // Canonical tokens reject duplicate keys and alternate spellings, permitting pretty JSON.
  if (JSON.stringify(value) !== compact) fail('SOURCE-POLICY');
  return value;
}
export function validatePolicy(reviewed, publication = 'qualification-candidate') {
  exact(reviewed, ['format', 'version', 'compositionKind', 'compositionSha256', 'source', 'runtime',
    'compiler', 'approval', 'template', 'witSha256', 'compilerPublication']);
  if (reviewed.format !== 'zryna.playground-source-launcher.v2' || reviewed.version !== 2 ||
      reviewed.compositionKind !== 'reviewed-source-v1' || !hash(reviewed.compositionSha256) ||
      !hash(reviewed.witSha256)) fail('SOURCE-POLICY');
  exact(reviewed.source, ['kind', 'repository', 'commit', 'tree', 'root', 'files']);
  if (reviewed.source.repository !== 'https://github.com/zryna/zryna') fail('SOURCE-POLICY');
  if (reviewed.source.kind === 'operator-reviewed-working-snapshot') {
    if (publication !== 'qualification-candidate' || reviewed.source.commit !== null || reviewed.source.tree !== null) fail('SOURCE-POLICY');
  } else if (reviewed.source.kind !== 'git-revision' || typeof reviewed.source.commit !== 'string' ||
      typeof reviewed.source.tree !== 'string' || !/^[a-f0-9]{40}$/.test(reviewed.source.commit) ||
      !/^[a-f0-9]{40}$/.test(reviewed.source.tree)) fail('SOURCE-POLICY');
  exact(reviewed.source.root, ['path', 'ownerUid']);
  descriptor({ ...reviewed.source.root, bytes: 1, sha256: '0'.repeat(64) }, 1);
  const files = reviewed.source.files;
  if (!Array.isArray(files) || files.length !== sourcePaths.length || files.length > 64) fail('SOURCE-POLICY');
  let total = 0;
  for (const [index, file] of files.entries()) {
    exact(file, ['path', 'bytes', 'sha256', 'mode']);
    if (file.path !== sourcePaths[index] || file.path.length > 240 || file.path.split('/').length > 16 ||
        !Number.isSafeInteger(file.bytes) || file.bytes < 1 || file.bytes > 131072 ||
        (total += file.bytes) > 1048576 || !hash(file.sha256) || ![0o400, 0o444, 0o600, 0o644].includes(file.mode)) fail('SOURCE-POLICY');
  }
  exact(reviewed.runtime, ['nodeVersion', 'node']);
  if (reviewed.runtime.nodeVersion !== '22.22.1') fail('SOURCE-RUNTIME');
  descriptor(reviewed.runtime.node, 268435456); descriptor(reviewed.compiler, 268435456);
  descriptor(reviewed.approval, 262144); descriptor(reviewed.template, 65536);
  exact(reviewed.compilerPublication, ['status', 'type', 'receipt']);
  const publicationType = publication === 'authenticated' ?
    'zryna.playground-compiler-publication.v1' : 'zryna.playground-compiler-candidate.v1';
  if (reviewed.compilerPublication.type !== publicationType) fail('SOURCE-PUBLICATION');
  descriptor(reviewed.compilerPublication.receipt, 262144);
  // Publication verification must come from its separate authenticated product verifier.
  if (!['qualification-candidate', 'authenticated'].includes(publication) ||
      reviewed.compilerPublication.status !== publication) fail('PUBLICATION-VERIFIER-REQUIRED');
  const identity = composition(reviewed);
  if (sha256(Buffer.from(JSON.stringify(identity))) !== reviewed.compositionSha256) fail('SOURCE-COMPOSITION');
  return structuredClone(reviewed);
}
export function composition(reviewed) {
  return { format: 'zryna.playground-source-composition.v2', version: 2,
    compositionKind: 'reviewed-source-v1', source: { kind: reviewed.source.kind, repository: reviewed.source.repository,
      commit: reviewed.source.commit, tree: reviewed.source.tree, files: reviewed.source.files },
    runtime: { nodeVersion: reviewed.runtime.nodeVersion,
      node: { bytes: reviewed.runtime.node.bytes, sha256: reviewed.runtime.node.sha256 } },
    compiler: { bytes: reviewed.compiler.bytes, sha256: reviewed.compiler.sha256 },
    template: { bytes: reviewed.template.bytes, sha256: reviewed.template.sha256 }, witSha256: reviewed.witSha256,
    compilerPublication: { status: reviewed.compilerPublication.status, type: reviewed.compilerPublication.type,
      receipt: { bytes: reviewed.compilerPublication.receipt.bytes, sha256: reviewed.compilerPublication.receipt.sha256 } } };
}
