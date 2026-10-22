'use strict';
// Trusted fixed bootstrap; source bytes are data and are never evaluated here.
const fs = require('node:fs');
const crypto = require('node:crypto');
const ROOT = '/work/project';
const names = ['src/main.zry', 'zryna.lock.json', 'zryna.package.json'];
function requireValue(value) { if (!value) throw new Error('CLI-PROJECT-STAGE-REJECTED'); }
function exact(value, keys) {
  requireValue(value && typeof value === 'object' && !Array.isArray(value) &&
    JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...keys].sort()));
}
function sha(value) { return crypto.createHash('sha256').update(value).digest('hex'); }
function identity(value) {
  return { dev: value.dev, ino: value.ino, mode: value.mode, uid: value.uid, gid: value.gid,
    nlink: value.nlink, size: value.size, mtimeMs: value.mtimeMs, ctimeMs: value.ctimeMs };
}
function directory(api, path, device) {
  const value = api.lstatSync(path);
  requireValue(value.isDirectory() && value.uid === 65534 && value.gid === 65534 &&
    (value.mode & 0o7777) === 0o700 && (device === undefined || value.dev === device));
  return value;
}
function inventories(api) {
  requireValue(JSON.stringify(api.readdirSync(ROOT).sort()) === JSON.stringify(['src', 'zryna.lock.json', 'zryna.package.json']));
  requireValue(JSON.stringify(api.readdirSync(ROOT + '/src')) === JSON.stringify(['main.zry']));
}
function validated(request) {
  exact(request, ['version', 'operationId', 'files']);
  requireValue(request.version === 1 && typeof request.operationId === 'string' &&
    /^(?:build|run)-[a-z0-9-]{1,100}$/.test(request.operationId) && Array.isArray(request.files) && request.files.length === 3);
  requireValue(JSON.stringify(request.files.map(x => x.path).sort()) === JSON.stringify(names));
  let total = 0;
  return request.files.map(file => {
    exact(file, ['path', 'bytes', 'mode', 'sha256', 'bodyBase64']);
    requireValue(file.mode === 0o600 && Number.isSafeInteger(file.bytes) && file.bytes > 0 && file.bytes <= 16384 &&
      typeof file.bodyBase64 === 'string' && file.bodyBase64.length <= 21848 && /^[a-f0-9]{64}$/.test(file.sha256));
    const data = Buffer.from(file.bodyBase64, 'base64'); total += data.length;
    requireValue(data.toString('base64') === file.bodyBase64 && data.length === file.bytes &&
      sha(data) === file.sha256 && total <= 32768);
    return { file, data };
  });
}
function capture(api, file, device) {
  const fd = api.openSync(ROOT + '/' + file.path, api.constants.O_RDONLY | api.constants.O_NOFOLLOW | api.constants.O_NONBLOCK);
  try {
    const before = api.fstatSync(fd);
    requireValue(before.isFile() && before.dev === device && before.uid === 65534 && before.gid === 65534 &&
      before.nlink === 1 && (before.mode & 0o7777) === 0o600 && before.size === file.bytes);
    const data = Buffer.alloc(file.bytes + 1); let count = 0;
    while (count < data.length) { const got = api.readSync(fd, data, count, data.length - count, null); if (!got) break; count += got; }
    requireValue(count === file.bytes && sha(data.subarray(0, count)) === file.sha256 &&
      JSON.stringify(identity(api.fstatSync(fd))) === JSON.stringify(identity(before)) &&
      JSON.stringify(identity(api.lstatSync(ROOT + '/' + file.path))) === JSON.stringify(identity(before)));
    return { path: file.path, bytes: count, mode: 0o600, uid: 65534, gid: 65534, nlink: 1, sha256: file.sha256 };
  } finally { api.closeSync(fd); }
}
function stage(request, api = fs) {
  const files = validated(request); const root = directory(api, ROOT);
  requireValue(api.statfsSync(ROOT).type === 0x01021994 && api.readdirSync(ROOT).length === 0);
  const rootIdentity = { dev: root.dev, ino: root.ino, uid: root.uid, gid: root.gid, mode: root.mode & 0o7777 };
  api.mkdirSync(ROOT + '/src', { mode: 0o700 }); directory(api, ROOT + '/src', root.dev);
  for (const { file, data } of files) {
    const fd = api.openSync(ROOT + '/' + file.path, api.constants.O_WRONLY | api.constants.O_CREAT | api.constants.O_EXCL | api.constants.O_NOFOLLOW, 0o600);
    try { let offset = 0; while (offset < data.length) { const count = api.writeSync(fd, data, offset, data.length - offset); requireValue(count > 0); offset += count; } }
    finally { api.closeSync(fd); }
  }
  inventories(api); const captured = files.map(({ file }) => capture(api, file, root.dev)).sort((a,b) => a.path < b.path ? -1 : 1);
  const after = directory(api, ROOT); directory(api, ROOT + '/src', root.dev); inventories(api);
  requireValue(after.dev === root.dev && after.ino === root.ino && api.statfsSync(ROOT).type === 0x01021994);
  return { version: 1, operationId: request.operationId, sourceStaged: true, filesystem: 'tmpfs',
    before: { empty: true, root: rootIdentity }, after: { root: rootIdentity, src: { mode: 0o700, uid: 65534, gid: 65534, sameDevice: true }, files: captured } };
}
module.exports = { stage };
