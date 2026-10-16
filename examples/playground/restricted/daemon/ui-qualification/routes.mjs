import { binary, bounds, envelope, exact, fail, integer, origin } from './wire.mjs';

const prefix = '/examples/playground/restricted/';
const names = ['client.mjs', 'controller.mjs', 'corpus.mjs', 'evaluation.mjs', 'executable.mjs',
  'index.html', 'json.mjs', 'limits.mjs', 'main.mjs', 'presentation.mjs', 'protocol.mjs',
  'report.mjs', 'style.css', 'worker.mjs'];
export const routes = Object.freeze(Object.fromEntries([
  ...names.map(name => [`asset:${name}`, { path: prefix + name, method: 'GET', input: 0, output: 131072 }]),
  ['index', { path: prefix, method: 'GET', input: 0, output: 131072 }],
  ...[['session', 'GET', 0, 98304], ['compile', 'POST', 32768, 1277956],
    ['evaluate', 'POST', 8192, 2048], ['finish', 'POST', 1024, 1024], ['cancel', 'POST', 1024, 1024]]
    .map(([name, method, input, output]) => [name, { path: '/' + name, method, input, output }]),
].map(([name, value]) => [name, Object.freeze(value)])));
const requestNames = ['authorization', 'origin', 'sec-fetch-site', 'content-type'];
const security = ['content-type', 'cache-control', 'content-security-policy', 'x-content-type-options',
  'referrer-policy', 'cross-origin-opener-policy', 'cross-origin-resource-policy'];
const responseNames = [...security, 'x-zryna-job', 'x-zryna-receipt'];
const transportNames = ['content-length', 'date', 'connection', 'keep-alive', 'transfer-encoding'];

export function headerPairs(value, allowed) {
  if (!Array.isArray(value) || value.length > 16) fail('ROUTES_001');
  const headers = Object.create(null);
  let total = 0;
  for (const entry of value) {
    exact(entry, ['name', 'value']);
    if (!allowed.includes(entry.name) || Object.hasOwn(headers, entry.name) ||
        typeof entry.value !== 'string' || /[^\x20-\x7e]/.test(entry.value) ||
        (total += entry.name.length + entry.value.length) > bounds.headers) fail('ROUTES_002');
    headers[entry.name] = entry.value;
  }
  return headers;
}
export function selectBrowserHeaders(headers) {
  return requestNames.filter(name => Object.hasOwn(headers, name)).map(name => ({ name, value: headers[name] }));
}
export function selectNetworkHeaders(request) {
  const selected = Object.create(null);let count = 0;let hosts = 0;
  if (request.rawHeaders.length > 64) fail('INTERCEPT_HEADERS');
  for (let i = 0; i < request.rawHeaders.length; i += 2) {
    const name = request.rawHeaders[i].toLowerCase(), value = request.rawHeaders[i + 1];
    if (name === 'host' && ++hosts !== 1) fail('INTERCEPT_HEADERS');
    if (!['authorization', 'origin', 'sec-fetch-site', 'content-type'].includes(name)) continue;
    if (Object.hasOwn(selected, name) || /[^\x20-\x7e]/.test(value) || (count += name.length + value.length) > bounds.headers) fail('INTERCEPT_HEADERS');
    selected[name] = value;
  }
  return selectBrowserHeaders(selected);
}
export function routeFor(url, method, expectedOrigin) {
  origin(expectedOrigin);
  if (typeof url !== 'string' || url.length > 512) fail('ROUTES_003');
  const parsed = new URL(url);
  if (parsed.origin !== expectedOrigin || parsed.search || parsed.hash || parsed.username || parsed.password ||
      url !== expectedOrigin + parsed.pathname) fail('ROUTES_004');
  const found = Object.entries(routes).find(([, route]) => route.path === parsed.pathname && route.method === method);
  if (!found || found[0] === 'asset:worker.mjs') fail('ROUTES_005');
  return found[0];
}
export function requestRecord(value, expectedOrigin, capability) {
  envelope(value, 'request', ['id', 'route', 'method', 'headers', 'bodyBase64']);
  integer(value.id, bounds.requests);
  if (!Object.hasOwn(routes, value.route) || value.route === 'asset:worker.mjs') fail('ROUTES_006');
  const route = routes[value.route];
  if (value.method !== route.method) fail('ROUTES_007');
  const headers = headerPairs(value.headers, requestNames);
  if (headers.authorization !== undefined && headers.authorization !== `Bearer ${capability}` ||
      headers.origin !== undefined && headers.origin !== expectedOrigin ||
      headers['sec-fetch-site'] !== undefined && !['none', 'same-origin'].includes(headers['sec-fetch-site']) ||
      headers['content-type'] !== undefined && headers['content-type'] !== 'application/json') fail('ROUTES_008');
  return { route, headers, body: binary(value.bodyBase64, route.input) };
}
export function responseHeaders(raw) {
  if (!Array.isArray(raw) || raw.length % 2 || raw.length > 64) fail('ROUTES_009');
  const selected = []; const seen = new Set(); let total = 0;
  for (let index = 0; index < raw.length; index += 2) {
    const name = raw[index].toLowerCase(); const value = raw[index + 1];
    if (seen.has(name) || ![...responseNames, ...transportNames].includes(name) ||
        typeof value !== 'string' || /[^\x20-\x7e]/.test(value) ||
        (total += name.length + value.length) > bounds.headers) fail('ROUTES_010');
    seen.add(name);
    if (responseNames.includes(name)) selected.push({ name, value });
  }
  if (security.some(name => !seen.has(name))) fail('ROUTES_011');
  return selected;
}
export function responseRecord(value, route) {
  envelope(value, 'response', ['id', 'status', 'headers', 'bodyBase64']);
  integer(value.id, bounds.requests); integer(value.status, 599);
  if (value.status < 200 || value.status >= 300 && value.status < 400) fail('ROUTES_012');
  const headers = headerPairs(value.headers, responseNames);
  if (security.some(name => !Object.hasOwn(headers, name))) fail('ROUTES_013');
  return { headers, body: binary(value.bodyBase64, routes[route].output) };
}
