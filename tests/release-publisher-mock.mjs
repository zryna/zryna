import assert from 'node:assert/strict';
import { sha256 } from '../scripts/distribution-release/canonical.mjs';

export function githubServer({ decoyCount = 0 } = {}) {
  let release = null;
  let mutations = 0;
  let interruptAfter = null;
  const uploadNames = [];
  const decoys = Array.from({ length: decoyCount }, (_, index) => ({
    id: 1_000 + index,
    tag_name: `v0.0.${index}`,
  }));
  const additional = [];
  const response = (status, value) => {
    const body = Buffer.from(JSON.stringify(value));
    return new Response(body, { status, headers: {
      'content-length': String(body.length), 'content-type': 'application/json',
    } });
  };
  const fetchImpl = async (url, options) => {
    const parsed = new URL(url);
    if (options.method === 'GET' && parsed.pathname.endsWith('/releases')) {
      assert.equal(parsed.searchParams.get('per_page'), '100');
      const page = Number(parsed.searchParams.get('page'));
      const releases = [...decoys, ...additional, ...(release === null ? [] : [release])];
      return response(200, releases.slice((page - 1) * 100, page * 100));
    }
    if (options.method === 'POST' && parsed.pathname.endsWith('/releases')) {
      mutations += 1;
      const input = JSON.parse(options.body);
      release = {
        id: 41, url: 'https://api.github.com/repos/zryna/zryna/releases/41',
        tag_name: input.tag_name, name: input.name, body: input.body,
        draft: input.draft, prerelease: input.prerelease, assets: [],
      };
      return response(201, release);
    }
    if (options.method === 'POST' && parsed.hostname === 'uploads.github.com') {
      if (release.assets.length === interruptAfter) throw new Error('simulated upload interruption');
      mutations += 1;
      const name = parsed.searchParams.get('name');
      const bytes = Buffer.from(await new Response(options.body).arrayBuffer());
      const id = 100 + release.assets.length;
      const asset = {
        id,
        url: `https://api.github.com/repos/zryna/zryna/releases/assets/${id}`,
        name,
        state: 'uploaded',
        size: bytes.length,
        digest: `sha256:${sha256(bytes)}`,
        browser_download_url: `https://github.com/zryna/zryna/releases/download/untagged-0123456789abcdefabcd/${encodeURIComponent(name)}`,
      };
      release.assets.push(asset);
      uploadNames.push(name);
      return response(201, asset);
    }
    if (options.method === 'PATCH' && parsed.pathname.endsWith('/releases/41')) {
      mutations += 1;
      const input = JSON.parse(options.body);
      release = {
        ...release,
        draft: input.draft,
        prerelease: input.prerelease,
        assets: release.assets.map((asset) => ({
          ...asset,
          browser_download_url: `https://github.com/zryna/zryna/releases/download/v0.2.3/${encodeURIComponent(asset.name)}`,
        })),
      };
      return response(200, release);
    }
    throw new Error(`unexpected request ${options.method} ${url}`);
  };
  return {
    fetchImpl,
    get release() { return release; },
    get mutations() { return mutations; },
    get uploadNames() { return [...uploadNames]; },
    interruptUploadsAfter(count) { interruptAfter = count; },
    mutateRelease(mutate) { mutate(release); },
    addRelease(value) { additional.push(value); },
  };
}
