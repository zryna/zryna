// Executes only the independently reviewed, pinned static verifier; compiler bytes remain inert.
import { spawnSync } from 'node:child_process';
import { closeSync, constants, fstatSync, mkdtempSync, openSync, readSync,
  rmdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { isAbsolute, join, posix } from 'node:path';
import { validateStaticVerifier } from '../signature.mjs';
import { certificateIdentity, exactKeys, gitId, issuer, requireValue, sha256,
  sourceRef, verifierPins } from './compiler-publication-constants.mjs';

function release(descriptors, primary) {
  const errors = primary ? [primary] : [];
  for (const descriptor of descriptors.reverse()) {
    try { closeSync(descriptor); } catch (error) { errors.push(error); }
  }
  if (errors.length === 1) throw errors[0];
  if (errors.length) throw new AggregateError(errors, 'PLAYGROUND-COMPILER-PUBLICATION-VERIFIER-CLEANUP');
}

export function captureCompilerPublicationVerifier(policy) {
  exactKeys(policy, ['version', 'path', 'bytes', 'sha256', 'ownerUid', 'trustedRootBytes', 'trustedRootSha256']);
  requireValue(process.platform === 'linux' && process.arch === 'x64' &&
    Object.entries(verifierPins).every(([key, value]) => policy[key] === value) &&
    Number.isSafeInteger(policy.ownerUid) && policy.ownerUid >= 0 && policy.ownerUid <= 0xffffffff &&
    typeof policy.path === 'string' && isAbsolute(policy.path) && posix.normalize(policy.path) === policy.path &&
    /^\/[A-Za-z0-9._/-]+$/.test(policy.path), 'compiler publication independently approved verifier');
  const descriptors = []; let snapshot; let primary;
  try {
    let parent = openSync('/', constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
    descriptors.push(parent);
    const parts = policy.path.slice(1).split('/');
    for (const part of parts.slice(0, -1)) {
      parent = openSync(`/proc/self/fd/${parent}/${part}`,
        constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
      descriptors.push(parent);
    }
    const file = openSync(`/proc/self/fd/${parent}/${parts.at(-1)}`,
      constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    descriptors.push(file);
    const before = fstatSync(file, { bigint: true });
    requireValue(before.isFile() && before.uid === BigInt(policy.ownerUid) &&
      before.size === BigInt(policy.bytes) && !(before.mode & 0o6022n), 'compiler publication verifier material');
    snapshot = Buffer.alloc(policy.bytes + 1); let offset = 0;
    while (offset < snapshot.length) {
      const count = readSync(file, snapshot, offset, Math.min(65536, snapshot.length - offset), offset);
      if (!count) break;
      offset += count;
    }
    const after = fstatSync(file, { bigint: true });
    requireValue(offset === policy.bytes && ['dev', 'ino', 'size', 'mode', 'uid', 'gid', 'mtimeNs', 'ctimeNs']
      .every(key => before[key] === after[key]), 'compiler publication stable verifier snapshot');
    snapshot = snapshot.subarray(0, offset);
    requireValue(sha256(snapshot) === policy.sha256, 'compiler publication verifier byte identity');
    validateStaticVerifier(snapshot);
  } catch (error) { primary = error; }
  release(descriptors, primary);
  return snapshot;
}

export function compilerPublicationVerificationArguments(sourceCommit, root) {
  requireValue(gitId(sourceCommit), 'compiler publication signature source');
  return ['verify-blob', '--offline', '--new-bundle-format=true', '--insecure-ignore-tlog=false',
    '--private-infrastructure=false', '--insecure-ignore-sct=false',
    '--trusted-root', join(root, 'trusted-root.json'), '--bundle', join(root, 'bundle.json'),
    '--certificate-identity', certificateIdentity, '--certificate-oidc-issuer', issuer,
    '--certificate-github-workflow-sha', sourceCommit,
    '--certificate-github-workflow-repository', 'zryna/zryna',
    '--certificate-github-workflow-ref', sourceRef,
    '--certificate-github-workflow-trigger', 'push', join(root, 'envelope.json')];
}

export function verifyCompilerPublicationSignature(envelope, bundle, policy, trustedRoot, sourceCommit) {
  requireValue(envelope instanceof Uint8Array && envelope.length > 0 && envelope.length <= 262144 &&
    bundle instanceof Uint8Array && bundle.length > 0 && bundle.length <= 1048576 &&
    trustedRoot instanceof Uint8Array && trustedRoot.length === verifierPins.trustedRootBytes &&
    sha256(trustedRoot) === verifierPins.trustedRootSha256, 'compiler publication signature materials');
  compilerPublicationVerificationArguments(sourceCommit, '/tmp');
  const executable = captureCompilerPublicationVerifier(policy);
  const root = mkdtempSync('/tmp/zryna-compiler-publication-');
  const owned = []; let primary; let verification;
  try {
    for (const [name, data, mode] of [['cosign', executable, 0o500], ['envelope.json', envelope, 0o400],
      ['bundle.json', bundle, 0o400], ['trusted-root.json', trustedRoot, 0o400]]) {
      const path = join(root, name);
      const descriptor = openSync(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW, mode);
      owned.push(path); let writeError;
      try { writeFileSync(descriptor, data); } catch (error) { writeError = error; }
      release([descriptor], writeError);
    }
    const argv = [join(root, 'cosign'), ...compilerPublicationVerificationArguments(sourceCommit, root)];
    const started = performance.now();
    const actual = spawnSync(argv[0], argv.slice(1), { shell: false, cwd: root,
      env: { LANG: 'C.UTF-8', HOME: root }, timeout: 20000, killSignal: 'SIGKILL', maxBuffer: 65536 });
    const stdout = Buffer.from(actual.stdout ?? []), stderr = Buffer.from(actual.stderr ?? []);
    verification = { command: { argv, cwd: root, environment: { LANG: 'C.UTF-8', HOME: root },
      timeoutMs: 20000, combinedOutputLimit: 65536 }, stdout, stderr,
    result: { status: actual.status, signal: actual.signal, errorCode: actual.error?.code ?? null,
      elapsedMs: Math.ceil(performance.now() - started), stdoutBytes: stdout.length, stderrBytes: stderr.length,
      stdoutSha256: sha256(stdout), stderrSha256: sha256(stderr) } };
    requireValue(!actual.error && actual.status === 0 && actual.signal === null &&
      stdout.length + stderr.length <= 65536, 'compiler publication actual signature verification');
  } catch (error) { primary = error; }
  const errors = primary ? [primary] : [];
  for (const path of owned.reverse()) {
    try { unlinkSync(path); } catch (error) { errors.push(error); }
  }
  try { rmdirSync(root); } catch (error) { errors.push(error); }
  if (errors.length) {
    const failure = errors.length === 1 ? errors[0] :
      new AggregateError(errors, 'PLAYGROUND-COMPILER-PUBLICATION-SIGNATURE-CLEANUP');
    if (verification) failure.verification = verification;
    throw failure;
  }
  return verification;
}
