import { bytes, id, requireValue, sha256 } from './common.mjs';
const API = 'https://api.github.com';
const DOWNLOAD_HOSTS = new Set(['api.github.com','release-assets.githubusercontent.com','objects.githubusercontent.com']);
export async function readResponse(response, maximum) {
  const encoding = response.headers.get('content-encoding');
  requireValue(!encoding || encoding === 'identity', 'compiler release encoded API response');
  const length = response.headers.get('content-length');
  requireValue(length === null || (/^(0|[1-9][0-9]*)$/.test(length) && Number(length) <= maximum),
    'compiler release API declared size');
  const chunks = []; let total = 0;
  requireValue(response.body?.getReader, 'compiler release API readable body');
  const reader = response.body.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read(); if (done) break;
      requireValue(value instanceof Uint8Array, 'compiler release API byte carrier');
      total += value.length; requireValue(total <= maximum, 'compiler release API byte limit');
      chunks.push(Buffer.from(value));
    }
  } catch (error) { try { await reader.cancel(); } catch {} throw error; }
  requireValue(length === null || Number(length) === total, 'compiler release API streamed size');
  return Buffer.concat(chunks, total);
}
export function github({ token = process.env.GITHUB_TOKEN, fetchImpl = fetch } = {}) {
  requireValue(typeof token === 'string' && token.length > 0, 'compiler release GitHub credential');
  const captures = [];
  async function request(path, { method = 'GET', body, maximum = 2*1024*1024,
    status = 200, binary = false, upload = false } = {}) {
    requireValue(typeof path === 'string' && /^\/repos\/zryna\/zryna\//.test(path) &&
      !/[\r\n#]/.test(path) && !path.includes('..') && !/%(?:2e|2f|5c)/i.test(path), 'compiler release fixed repository API');
    let url = `${upload ? 'https://uploads.github.com' : API}${path}`;
    const signal = AbortSignal.timeout(120000);
    for (let redirects = 0; redirects <= 4; redirects++) {
      const parsed = new URL(url);
      const headers = { Accept: binary && !upload ? 'application/octet-stream' : 'application/vnd.github+json',
        'Accept-Encoding': 'identity', 'X-GitHub-Api-Version': '2026-03-10' };
      if (parsed.hostname === 'api.github.com' || parsed.hostname === 'uploads.github.com') {
        headers.Authorization = `Bearer ${token}`;
      }
      if (body !== undefined) headers['Content-Type'] = binary ? 'application/octet-stream' : 'application/json';
      const response = await fetchImpl(url, { method, body: body === undefined ? undefined :
        (binary ? body : bytes(body)), headers, signal, redirect: 'manual' });
      if ([301,302,303,307,308].includes(response.status)) {
        requireValue(method === 'GET' && binary && redirects < 4, 'compiler release API redirect');
        const next = new URL(response.headers.get('location'), url);
        requireValue(next.protocol === 'https:' && DOWNLOAD_HOSTS.has(next.hostname) &&
          !next.username && !next.password && !next.port, 'compiler release approved download redirect');
        url = next.href; continue;
      }
      const data = await readResponse(response, maximum);
      captures.push({ method, path, status: response.status, bytes: data.length, sha256: sha256(data), data });
      requireValue(response.status === status, `compiler release API status ${response.status}`);
      return binary ? data : JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(data));
    }
    throw new Error('M6-COMPILER-RELEASE-REDIRECT');
  }
  return { request, captures,
    asset: assetId => request(`/repos/zryna/zryna/releases/assets/${id(assetId)}`,
      { binary:true, maximum:64*1024*1024 }),
  };
}
