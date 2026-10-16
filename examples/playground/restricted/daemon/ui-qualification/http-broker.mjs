// Trusted operator-only fixed-origin HTTP transport. No target or command comes from the UI.
import { request as httpRequest } from 'node:http';
import { bounds, fail, hash, integer, origin, sha256, tagTransport } from './wire.mjs';
import { requestRecord, responseHeaders } from './routes.mjs';

export function createHttpBroker({ origin: selectedOrigin, capability, holdCompileProbe = false, onEvent = () => {} }) {
  origin(selectedOrigin);
  if (!hash(capability) || typeof holdCompileProbe !== 'boolean') fail('BROKER_001');
  const target = new URL(selectedOrigin);
  const jobs = new Map(); const finished = new Map(); const sockets = new Set(); const queue = []; const audit = [];
  let nextId = 1; let live = 0; let closed = false; let recovery = null; let held = false; let closing = null;

  function complete(job, reply, error) {
    if (job.state === 'done') return;
    if (job.state === 'queued') {
      const index = queue.indexOf(job);
      if (index !== -1) queue.splice(index, 1);
    }
    job.state = 'done'; clearTimeout(job.timer); jobs.delete(job.id);
    if (!error) finished.set(job.id, { reply: reply.type, abortReceived: job.abortReceived });
    if (error) job.reject(tagTransport(error, 'HTTP_ERROR')); else job.resolve(reply);
    pump();
  }
  function dispatch(job) {
    job.state = 'live'; live++;
    const chunks = []; let size = 0; let selectedHeaders;
    const request = httpRequest({ protocol: 'http:', hostname: '127.0.0.1', port: target.port,
      path: job.route.path, method: job.method, agent: false,
      headers: { ...job.headers, 'content-length': job.body.length } }, response => {
      try {
        if (!Number.isInteger(response.statusCode) || response.statusCode < 200 || response.statusCode > 599 ||
            response.statusCode >= 300 && response.statusCode < 400) fail('BROKER_002');
        selectedHeaders = responseHeaders(response.rawHeaders);
      } catch (error) { request.destroy(error); return; }
      response.on('data', chunk => {
        size += chunk.length;
        if (size > job.route.output) request.destroy(new Error('PLAYGROUND-UI-TRANSPORT'));
        else chunks.push(chunk);
      });
      response.on('error', error => complete(job, null, error));
      response.on('aborted', () => complete(job, null, new Error('PLAYGROUND-UI-TRANSPORT')));
      response.on('end', () => {
        if (job.state === 'done') return;
        const body = Buffer.concat(chunks);
        audit.push(Object.freeze({ id: job.id, route: job.name, method: job.method,
          requestBytes: job.body.length, requestSha256: sha256(job.body), status: response.statusCode,
          responseBytes: body.length, responseSha256: sha256(body),
          securityHeaders: selectedHeaders.filter(entry => !entry.name.startsWith('x-zryna-')) }));
        complete(job, { version: 1, type: 'response', id: job.id, status: response.statusCode,
          headers: selectedHeaders, bodyBase64: body.toString('base64') });
      });
    });
    job.request = request;
    sockets.add(request);
    request.once('close', () => { sockets.delete(request); live--; pump(); });
    job.reaped = new Promise(resolve => request.once('close', resolve));
    request.once('error', error => complete(job, null, error));
    job.timer = setTimeout(() => request.destroy(new Error('PLAYGROUND-UI-TRANSPORT')),
      job.name === 'compile' ? 12000 : job.name === 'evaluate' ? 7000 : 5000);
    request.end(job.body);
  }
  function pump() {
    while (!closed && live < bounds.live && queue.length) {
      const job = queue.shift();
      if (job.state === 'queued') dispatch(job);
    }
  }
  return Object.freeze({
    request(record) {
      const waiting = [...jobs.values()].filter(job => job.state === 'queued' || job.state === 'held').length;
      if (closed || jobs.size >= bounds.live + bounds.queued || waiting >= bounds.queued) fail('BROKER_003');
      const checked = requestRecord(record, selectedOrigin, capability);
      if (record.id !== nextId++) fail('BROKER_004');
      const job = { id: record.id, name: record.route, method: record.method, ...checked, abortReceived: false,
        state: 'queued', request: null, reaped: Promise.resolve(), timer: null };
      const result = new Promise((resolve, reject) => { job.resolve = resolve; job.reject = reject; });
      jobs.set(job.id, job);
      if (holdCompileProbe && !held && job.name === 'compile') {
        held = true; job.state = 'held';
        job.timer = setTimeout(() => complete(job, null, new Error('PLAYGROUND-UI-ABORT-UNPROVED')), 1500);
        try { onEvent(Object.freeze({ version: 1, type: 'probe-paused', id: job.id })); }
        catch (error) { complete(job, null, error); }
      } else { queue.push(job); pump(); }
      return result;
    },
    abort(id) {
      integer(id, bounds.requests);
      const job = jobs.get(id);
      if (closed) fail('BROKER_005');
      if (!job) {
        const terminal = finished.get(id);
        if (!terminal || terminal.reply !== 'response' || terminal.abortReceived) fail('BROKER_006');
        terminal.abortReceived = true;
        return { version: 1, type: 'aborted', id };
      }
      job.abortReceived = true;
      job.request?.destroy();
      audit.push(Object.freeze({ id, route: job.name, aborted: true, dispatched: job.state === 'live',
        requestBytes: job.body.length, requestSha256: sha256(job.body) }));
      complete(job, { version: 1, type: 'aborted', id });
      return null;
    },
    audit() { return structuredClone(audit); },
    activeIds() { return [...jobs.keys()]; },
    ownedSockets() { return sockets.size; },
    recovery() { return recovery; },
    close() {
      if (closing) return closing;
      closed = true;
      closing = (async () => {
        const owned = [...jobs.values()];
        const reaping = [...sockets].map(request => new Promise(resolve => request.once('close', resolve)));
        for (const request of sockets) request.destroy();
        for (const job of owned) {
          job.request?.destroy(); complete(job, null, new Error('PLAYGROUND-UI-STOPPED'));
        }
        let timeout;
        try {
          await Promise.race([Promise.all(reaping), new Promise((_, reject) => {
            timeout = setTimeout(() => reject(new Error('PLAYGROUND-UI-CLEANUP')), 2000);
          })]);
        } catch (error) { recovery = error; throw error; }
        finally { clearTimeout(timeout); }
      })();
      return closing;
    },
  });
}
