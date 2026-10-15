import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { boundedResponse } from './hosted-api.mjs';
import { zipEntries } from './hosted-archive.mjs';
import { digest } from './registry-files.mjs';

// The only redirect authority is a fresh authenticated GitHub artifact response.
// The bearer credential is sent to GitHub only, never to the returned storage URL.
export async function downloadPublicationArchive(repository, artifact, {
  token = process.env.GITHUB_TOKEN, request = fetch,
} = {}) {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)
      || !Number.isSafeInteger(artifact.id) || artifact.id < 1
      || !Number.isSafeInteger(artifact.bytes) || artifact.bytes < 1 || artifact.bytes > 768 * 1024 * 1024
      || !/^[a-f0-9]{64}$/.test(artifact.sha256)) throw new Error('invalid archive expectation');
  const headers = { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' };
  if (token) headers.Authorization = `Bearer ${token}`;
  const initial = await request(`https://api.github.com/repos/${repository}/actions/artifacts/${artifact.id}/zip`, {
    headers, redirect: 'manual', signal: AbortSignal.timeout(30_000),
  });
  let response = initial;
  if (initial.status === 302) {
    const destination = new URL(initial.headers.get('location'));
    if (destination.protocol !== 'https:' || destination.username || destination.password
        || destination.port || !(/^[a-z0-9.-]+\.blob\.core\.windows\.net$/u.test(destination.hostname)
        || /^[a-z0-9.-]+\.actions\.githubusercontent\.com$/u.test(destination.hostname))) {
      throw new Error('unsupported authenticated artifact storage destination');
    }
    await initial.body?.cancel();
    response = await request(destination.href, { redirect: 'error', signal: AbortSignal.timeout(120_000) });
  }
  const bytes = await boundedResponse(response, artifact.bytes);
  if (bytes.length !== artifact.bytes || digest(bytes) !== artifact.sha256) {
    throw new Error('downloaded artifact archive identity differs');
  }
  return bytes;
}

export function retainPublicationArchive(directory, bytes) {
  // Validate the complete ZIP before creating output. The shared hostile-ZIP parser owns bounds.
  zipEntries(bytes);
  mkdirSync(directory, { recursive: false });
  writeFileSync(join(directory, 'artifact.zip'), bytes, { flag: 'wx', mode: 0o600 });
  mkdirSync(join(directory, 'evidence'));
  zipEntries(bytes, (entry, content) => {
    const file = join(directory, 'evidence', entry.path);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, content, { flag: 'wx', mode: 0o600 });
  });
}
