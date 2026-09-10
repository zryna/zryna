import { spawnSync } from 'node:child_process';
import { readSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical } from './canonical.mjs';
import {
  closeReleaseFile, exactReleaseNames, hashReleaseFile, MAX_RELEASE_DOCUMENT, openReleaseFile,
  readReleaseFile, revalidateReleaseFile,
} from './release-files.mjs';
import { validateReleaseText } from './validate.mjs';
import { verifySignedRelease } from './verify-signed-release.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const API = 'https://api.github.com';
const UPLOADS = 'https://uploads.github.com';
const REPOSITORY = 'zryna/zryna';
const TAG = 'v0.2.0';
const NAME = 'Zryna 0.2.0 beta';
const MAX_RESPONSE = 2 * 1024 * 1024;
const MAX_TIMEOUT = 5 * 60 * 1000;
const CHUNK = 1024 * 1024;

function reject(message) {
  throw new Error(`R406-PUBLISHER: ${message}`);
}

function artifactDescriptors(envelope) {
  return new Map([
    ...envelope.subjects.flatMap((subject) => [
      subject.archive, subject.buildReceipt, subject.sbom, subject.provenance, subject.attestation,
    ]),
    envelope.releaseNotes,
    envelope.checksums.document,
    envelope.checksums.signature,
    envelope.signing.releaseNotesSignature,
    envelope.revocation.policy,
  ].map((value) => [value.path, value]));
}

function loadPublication(directory) {
  const envelopeName = 'zryna-release-envelope-v1.json';
  const envelopeDescriptor = (() => {
    const file = openReleaseFile(directory, envelopeName, MAX_RELEASE_DOCUMENT);
    try { return hashReleaseFile(file); } finally { closeReleaseFile(file); }
  })();
  const envelope = validateReleaseText(readReleaseFile(
    directory, envelopeName, MAX_RELEASE_DOCUMENT,
  ).toString('utf8'));
  exactReleaseNames(directory, envelope.assetAllowlist);
  const descriptors = artifactDescriptors(envelope);
  descriptors.set(envelopeName, envelopeDescriptor);
  const signature = (() => {
    const path = envelope.signing.envelopeSignaturePath;
    const file = openReleaseFile(directory, path, MAX_RELEASE_DOCUMENT);
    try { return hashReleaseFile(file); } finally { closeReleaseFile(file); }
  })();
  descriptors.set(signature.path, signature);
  for (const name of envelope.assetAllowlist) {
    const expected = descriptors.get(name);
    if (!expected) reject(`${name} lacks a publication descriptor`);
    const file = openReleaseFile(directory, name);
    try {
      if (canonical(hashReleaseFile(file)) !== canonical(expected)) {
        reject(`${name} differs from its publication descriptor`);
      }
    } finally {
      closeReleaseFile(file);
    }
  }
  const notes = readReleaseFile(directory, envelope.releaseNotes.path, MAX_RELEASE_DOCUMENT)
    .toString('utf8');
  const body = `${notes}\nRelease envelope: sha256:${envelopeDescriptor.sha256}\nWorkflow run: ${envelope.workflow.runId}/${envelope.workflow.runAttempt}\n`;
  return { envelope, descriptors, body };
}

function validateEnvironment(environment, publication) {
  const { envelope } = publication;
  if (!environment.GITHUB_TOKEN || environment.GITHUB_REPOSITORY !== REPOSITORY
    || environment.GITHUB_SERVER_URL !== 'https://github.com'
    || environment.GITHUB_REF !== `refs/tags/${TAG}`
    || environment.GITHUB_SHA !== envelope.source.commit
    || environment.GITHUB_WORKFLOW_SHA !== envelope.workflow.commit
    || environment.GITHUB_RUN_ID !== envelope.workflow.runId
    || Number(environment.GITHUB_RUN_ATTEMPT) !== envelope.workflow.runAttempt) {
    reject('exact release workflow identity and token are required');
  }
}

async function cancelReader(reader) {
  try { await reader.cancel(); } catch { /* The request still fails closed. */ }
}

async function responseBytes(response, url) {
  const encoding = response.headers?.get?.('content-encoding');
  if (encoding !== null && encoding !== undefined && encoding.toLowerCase() !== 'identity') {
    reject(`${url} returned encoded content`);
  }
  const lengthText = response.headers?.get?.('content-length');
  let declared;
  if (lengthText !== null && lengthText !== undefined) {
    if (!/^(0|[1-9][0-9]*)$/.test(lengthText)) reject(`${url} returned invalid content length`);
    declared = Number(lengthText);
    if (!Number.isSafeInteger(declared) || declared > MAX_RESPONSE) {
      reject(`${url} response exceeds ${MAX_RESPONSE} bytes`);
    }
  }
  if (!response.body || typeof response.body.getReader !== 'function') {
    if (declared === 0) return Buffer.alloc(0);
    reject(`${url} did not return a readable stream`);
  }
  const reader = response.body.getReader();
  const chunks = [];
  let size = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    if (!(value instanceof Uint8Array)) {
      await cancelReader(reader);
      reject(`${url} returned invalid bytes`);
    }
    size += value.byteLength;
    if (size > MAX_RESPONSE) {
      await cancelReader(reader);
      reject(`${url} response exceeds ${MAX_RESPONSE} bytes`);
    }
    chunks.push(Buffer.from(value));
  }
  if (declared !== undefined && declared !== size) reject(`${url} response length differs`);
  return Buffer.concat(chunks, size);
}

async function request({ url, token, fetchImpl, timeoutMs, method = 'GET', body, headers = {},
  statuses = [200] }) {
  if (!url.startsWith(`${API}/`) && !url.startsWith(`${UPLOADS}/`)) reject('unexpected API origin');
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl(url, {
      method,
      headers: {
        Accept: 'application/vnd.github+json',
        'Accept-Encoding': 'identity',
        Authorization: `Bearer ${token}`,
        'X-GitHub-Api-Version': '2026-03-10',
        ...headers,
      },
      body,
      ...(body instanceof ReadableStream ? { duplex: 'half' } : {}),
      redirect: 'error',
      signal: controller.signal,
    });
    const bytes = await responseBytes(response, url);
    if (!statuses.includes(response.status)) reject(`${url} returned ${response.status}`);
    if (response.status === 404) return null;
    if (bytes.length === 0) return null;
    try { return JSON.parse(bytes.toString('utf8')); } catch { reject(`${url} did not return JSON`); }
  } catch (error) {
    if (error?.name === 'AbortError') reject(`${url} timed out`);
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function numericId(value, label) {
  const text = String(value);
  if (!/^[1-9][0-9]{0,19}$/.test(text)) reject(`${label} is not a bounded numeric ID`);
  return text;
}

function releaseIdentity(release, publication, expectedDraft) {
  const id = numericId(release?.id, 'release ID');
  if (release?.url !== `${API}/repos/${REPOSITORY}/releases/${id}`
    || release.tag_name !== TAG || release.name !== NAME || release.body !== publication.body
    || release.draft !== expectedDraft || release.prerelease !== true
    || !Array.isArray(release.assets)) {
    reject('GitHub release identity or state differs');
  }
  return id;
}

function verifyAssets(release, publication, expectedNames) {
  const expected = [...expectedNames].sort();
  const actual = release.assets.map(({ name }) => name).sort();
  if (canonical(actual) !== canonical(expected)) reject('GitHub release asset inventory differs');
  for (const asset of release.assets) {
    const descriptor = publication.descriptors.get(asset.name);
    numericId(asset.id, `${asset.name} asset ID`);
    if (!descriptor || asset.state !== 'uploaded' || asset.size !== descriptor.size
      || asset.digest !== `sha256:${descriptor.sha256}`
      || asset.browser_download_url !== `https://github.com/${REPOSITORY}/releases/download/${TAG}/${encodeURIComponent(asset.name)}`) {
      reject(`${asset.name} server-side identity or digest differs`);
    }
  }
}

function contentType(name) {
  if (name.endsWith('.zip')) return 'application/zip';
  if (name.endsWith('.tar.gz')) return 'application/gzip';
  if (name.endsWith('.json') || name.endsWith('.jsonl')) return 'application/json';
  if (name.endsWith('.md')) return 'text/markdown; charset=utf-8';
  return 'text/plain; charset=utf-8';
}

function retainedStream(directory, expected) {
  const file = openReleaseFile(directory, expected.path);
  let observed;
  try {
    observed = hashReleaseFile(file);
  } catch (error) {
    closeReleaseFile(file);
    throw error;
  }
  if (canonical(observed) !== canonical(expected)) {
    closeReleaseFile(file);
    reject(`${expected.path} changed before upload`);
  }
  let offset = 0;
  let closed = false;
  const close = () => {
    if (!closed) { closeReleaseFile(file); closed = true; }
  };
  const stream = new ReadableStream({
    pull(controller) {
      if (offset === file.metadata.size) {
        revalidateReleaseFile(file);
        close();
        controller.close();
        return;
      }
      const buffer = Buffer.allocUnsafe(Math.min(CHUNK, file.metadata.size - offset));
      const count = readSync(file.descriptor, buffer, 0, buffer.length, offset);
      if (count !== buffer.length) {
        close();
        controller.error(new Error(`R406-PUBLISHER: ${expected.path} ended during upload`));
        return;
      }
      offset += count;
      controller.enqueue(buffer);
    },
    cancel() { close(); },
  });
  return { stream, close };
}

async function getRelease(token, fetchImpl, timeoutMs) {
  return request({
    url: `${API}/repos/${REPOSITORY}/releases/tags/${TAG}`,
    token, fetchImpl, timeoutMs, statuses: [200, 404],
  });
}

export async function publishDraftRelease({ directory, phase, environment = process.env,
  fetchImpl = fetch, requestTimeoutMs = MAX_TIMEOUT,
  verifyImpl = verifySignedRelease, signatureSpawn = spawnSync }) {
  if (!isAbsolute(directory) || resolve(directory) !== directory
    || !['create-draft', 'upload', 'verify-draft', 'publish'].includes(phase)
    || !Number.isInteger(requestTimeoutMs) || requestTimeoutMs < 1 || requestTimeoutMs > MAX_TIMEOUT) {
    reject('exact directory, phase, and bounded timeout are required');
  }
  verifyImpl({ directory, spawn: signatureSpawn });
  const publication = loadPublication(directory);
  validateEnvironment(environment, publication);
  const token = environment.GITHUB_TOKEN;
  if (phase === 'create-draft') {
    const existing = await getRelease(token, fetchImpl, requestTimeoutMs);
    if (existing !== null) reject('a release already exists for the immutable tag');
    const created = await request({
      url: `${API}/repos/${REPOSITORY}/releases`, token, fetchImpl, timeoutMs: requestTimeoutMs,
      method: 'POST', statuses: [201], headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        tag_name: TAG, name: NAME, body: publication.body, draft: true, prerelease: true,
        generate_release_notes: false, make_latest: 'false',
      }),
    });
    releaseIdentity(created, publication, true);
    verifyAssets(created, publication, []);
    return created;
  }
  const release = await getRelease(token, fetchImpl, requestTimeoutMs);
  if (release === null) reject('the expected draft release is missing');
  const releaseId = releaseIdentity(release, publication, true);
  if (phase === 'upload') {
    verifyAssets(release, publication, []);
    for (const name of publication.envelope.assetAllowlist) {
      const expected = publication.descriptors.get(name);
      const retained = retainedStream(directory, expected);
      try {
        const asset = await request({
          url: `${UPLOADS}/repos/${REPOSITORY}/releases/${releaseId}/assets?name=${encodeURIComponent(name)}`,
          token, fetchImpl, timeoutMs: requestTimeoutMs, method: 'POST', statuses: [201],
          headers: { 'Content-Type': contentType(name), 'Content-Length': String(expected.size) },
          body: retained.stream,
        });
        verifyAssets({ assets: [asset] }, publication, [name]);
      } finally {
        retained.close();
      }
    }
    return release;
  }
  verifyAssets(release, publication, publication.envelope.assetAllowlist);
  if (phase === 'verify-draft') return release;
  const published = await request({
    url: `${API}/repos/${REPOSITORY}/releases/${releaseId}`,
    token, fetchImpl, timeoutMs: requestTimeoutMs, method: 'PATCH', statuses: [200],
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ draft: false, prerelease: true, make_latest: 'false' }),
  });
  releaseIdentity(published, publication, false);
  verifyAssets(published, publication, publication.envelope.assetAllowlist);
  return published;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 6 || process.argv[2] !== '--directory'
      || process.argv[4] !== '--phase') reject('expected --directory and --phase once');
    await publishDraftRelease({ directory: resolve(process.argv[3]), phase: process.argv[5] });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
