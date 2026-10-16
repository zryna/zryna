// Closed inert evaluation input; executable templates and compiler identity belong to the image.
import { constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { decodeJson } from '../json.mjs';
import { decodeCompilation, decodeSourceRequest } from '../protocol.mjs';
import { verifyExecutable } from '../executable.mjs';
import { exact, fail, isHash, limits, sha256 } from '../limits.mjs';
import { validateSelection } from '../evaluation.mjs';

export const evaluationInputLimit = 2 * 1024 * 1024;

export async function readImageFile(path, maximum) {
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  try {
    const before = await file.stat({ bigint: true });
    if (!before.isFile() || before.size < 1n || before.size > BigInt(maximum)) fail('IMAGE-FILE');
    const buffer = Buffer.alloc(Number(before.size) + 1);
    let used = 0;
    while (used < buffer.length) {
      const { bytesRead } = await file.read(buffer, used, buffer.length - used, null);
      if (!bytesRead) break;
      used += bytesRead;
    }
    const after = await file.stat({ bigint: true });
    if (used !== Number(before.size) || ['dev', 'ino', 'size', 'mtimeNs', 'ctimeNs', 'mode', 'uid', 'gid']
      .some(key => before[key] !== after[key])) fail('IMAGE-FILE');
    return new Uint8Array(buffer.subarray(0, used));
  } finally { await file.close(); }
}

export function decodeBase64Frame(value) {
  if (typeof value !== 'string' || value.length > Math.ceil(limits.frame / 3) * 4 ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) {
    fail('EVALUATION-FRAME');
  }
  const frame = Buffer.from(value, 'base64');
  if (frame.length < 4 || frame.length > limits.frame || frame.toString('base64') !== value) {
    fail('EVALUATION-FRAME');
  }
  return new Uint8Array(frame);
}

export async function admitEvaluation(inputBytes, identityBytes, templateBytes) {
  const input = decodeJson(inputBytes, evaluationInputLimit);
  exact(input, ['version', 'sourceRequest', 'frameBase64', 'selection', 'receipt', 'authority']);
  if (input.version !== 1) fail('EVALUATION-VERSION');
  const sourceRequest = decodeSourceRequest(new TextEncoder().encode(JSON.stringify(input.sourceRequest)));
  const identity = decodeJson(identityBytes, 4096);
  exact(identity, ['version', 'compilerSha256', 'bindingTemplateSha256', 'witSha256', 'browserArchiveSha256']);
  if (identity.version !== 1 || Object.entries(identity).some(([key, value]) => key !== 'version' && !isHash(value))) {
    fail('IMAGE-IDENTITY');
  }
  exact(input.authority, ['compilerSha256', 'toolkitSha256', 'policySha256', 'bindingTemplateSha256']);
  if (input.authority.compilerSha256 !== identity.compilerSha256 ||
      input.authority.bindingTemplateSha256 !== identity.bindingTemplateSha256 ||
      await sha256(templateBytes) !== identity.bindingTemplateSha256) fail('IMAGE-AUTHORITY');
  const frame = decodeBase64Frame(input.frameBase64);
  const expected = { revision: sourceRequest.revision, source: sourceRequest.source, witSha256: identity.witSha256 };
  const compiled = await decodeCompilation(frame, expected);
  if (compiled.metadata.status !== 'compiled') fail('EVALUATION-NOT-COMPILED');
  validateSelection(input.selection, compiled.metadata);
  await verifyExecutable(frame, compiled, input.receipt, input.authority, templateBytes);
  return { input, expected };
}
