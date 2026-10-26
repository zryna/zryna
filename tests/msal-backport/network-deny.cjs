'use strict';
// Loaded before SDK/packager modules. No live network or auth/UI subprocess may escape.
const { syncBuiltinESMExports } = require('node:module');
const events = [];
function denied(kind) { return function () { events.push(kind); throw new Error(`M418_DENIED_${kind}`); }; }
for (const name of ['node:http', 'node:https']) {
  const module = require(name); module.request = denied(name); module.get = denied(name);
}
const net = require('node:net');
net.connect = denied('net'); net.createConnection = denied('net');
net.Socket.prototype.connect = denied('socket');
const tls = require('node:tls'); tls.connect = denied('tls');
const dns = require('node:dns');
for (const name of Object.keys(dns)) if (/^(lookup|resolve|reverse)/.test(name) && typeof dns[name] === 'function') dns[name] = denied('dns');
for (const name of Object.keys(dns.promises)) if (typeof dns.promises[name] === 'function') dns.promises[name] = denied('dns');
require('node:dgram').createSocket = denied('dgram');
globalThis.fetch = denied('fetch'); globalThis.WebSocket = denied('websocket');
const cp = require('node:child_process');
// SDK-only persistent packet: no packaging/auth/UI subprocess exception exists.
for (const name of ['spawn', 'spawnSync', 'exec', 'execSync', 'execFile', 'execFileSync', 'fork'])
  cp[name] = denied('child_process');
globalThis.__m418Denied = events;
syncBuiltinESMExports();
