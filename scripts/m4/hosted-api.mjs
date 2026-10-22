const API = 'https://api.github.com';
const MAX_JSON = 4 * 1024 * 1024;

export async function boundedResponse(response, maximum) {
  if (!response.ok) throw new Error(`M4 hosted: GitHub HTTP ${response.status}`);
  const declared = response.headers.get('content-length');
  if (declared !== null && (!/^\d+$/.test(declared) || Number(declared) > maximum)) {
    throw new Error('M4 hosted: response exceeds byte bound');
  }
  const chunks = [];
  let length = 0;
  const reader = response.body.getReader();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      length += value.byteLength;
      if (length > maximum) throw new Error('M4 hosted: response exceeds byte bound');
      chunks.push(Buffer.from(value));
    }
  } finally { await reader.cancel(); }
  return Buffer.concat(chunks, length);
}

export function githubApi(token = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN) {
  return async function get(endpoint) {
    if (!/^\/repos\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_./?=&-]+$/.test(endpoint)
        || endpoint.includes('..')) throw new Error('M4 hosted: invalid API endpoint');
    const headers = { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' };
    if (token) headers.Authorization = `Bearer ${token}`;
    const response = await fetch(`${API}${endpoint}`, {
      headers, redirect: 'error', signal: AbortSignal.timeout(30_000),
    });
    const bytes = await boundedResponse(response, MAX_JSON);
    return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  };
}
