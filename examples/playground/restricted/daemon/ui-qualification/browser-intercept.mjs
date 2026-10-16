// Trusted UI-only tcp4 loopback bridge. The browser supplies actual network headers.
import { createServer } from 'node:http';
import { bounds, envelope, fail, integer, tagTransport, transportBranch, origin, sha256 } from './wire.mjs';
import { routeFor, routes, responseRecord, selectNetworkHeaders, loopbackResponseHeaders } from './routes.mjs';
import { observeBrowserRequests, requestFailureBranch } from './browser-diagnostics.mjs';

export async function interceptBrowser(context, selectedOrigin, send) {
  origin(selectedOrigin);const target = new URL(selectedOrigin);const maximum = bounds.live + bounds.queued;
  let serial = 0;let admitted = 0;let reading = 0;let stopped = false;let firstFailure = null;let rejectFailure;let stopping;
  let compileRequest = null;let compileJob = null;let totalIn = 0;let totalOut = 0;let corpusEnabled = false;
  const pending = new Map(), completed = new Map(), sockets = new Set(), operations = new Set(), transmissions = new Set(), aborts = [], pages = new Set();
  const requestObservations = observeBrowserRequests(context, selectedOrigin);
  const failures = new Promise((_, reject) => { rejectFailure = reject; });failures.catch(() => {});
  const broken = (error, branch = 'INTERCEPT_EVENT') => { firstFailure ??= tagTransport(error instanceof Error ? error : new Error('PLAYGROUND-UI-TRANSPORT'), branch);rejectFailure(firstFailure); };
  const pageEvents = Object.fromEntries(['worker', 'websocket', 'download', 'popup'].map(name => [name, () => broken(null, 'INTERCEPT_PAGE_' + name.toUpperCase())]));
  const serviceWorker = () => broken(null, 'INTERCEPT_SERVICE_WORKER');
  const transmit = record => {
    const operation = Promise.resolve().then(() => { if (stopped) throw new Error('PLAYGROUND-UI-STOPPED');return send(record); });
    transmissions.add(operation);operation.then(() => transmissions.delete(operation), error => { transmissions.delete(operation);broken(error, 'INTERCEPT_SEND'); });return operation;
  };
  function associated(job) {
    return compileRequest && job === compileJob && compileRequest.bytes === job.bytes && compileRequest.sha256 === job.sha256;
  }
  function abortIfProved(job) {
    if (stopped || !job || job.abortSent || !job.paused || !job.httpClosed || !associated(job) || !compileRequest.failed) return;
    job.abortSent = true;job.state = 'aborting';transmit({ version: 1, type: 'abort', id: job.id }).catch(() => {});
  }
  const requestStarted = request => {
    if (stopped) return;
    try {
      const name = routeFor(request.url(), request.method(), selectedOrigin);
      if (!['document', 'script', 'stylesheet', 'fetch'].includes(request.resourceType())) fail('INTERCEPT_002');
      if (name !== 'compile' || corpusEnabled) return;
      const body = request.postDataBuffer() ?? Buffer.alloc(0);
      if (compileRequest || body.length > routes.compile.input) fail('INTERCEPT_003');
      compileRequest = { request, bytes: body.length, sha256: sha256(body), failed: false };
      if (compileJob && !associated(compileJob)) fail('INTERCEPT_006');abortIfProved(compileJob);
    } catch (error) { broken(error, 'INTERCEPT_REQUEST_EVENT'); }
  };
  const requestFailed = request => {
    if (stopped) return;
    if (!compileRequest || request !== compileRequest.request) { broken(requestObservations.failed(request, tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'), requestFailureBranch(request))));return; }
    compileRequest.failed = true;abortIfProved(compileJob);
  };
  const pageStarted = page => {
    if (pages.size) { broken(null, 'INTERCEPT_SECOND_PAGE');return; }pages.add(page);
    for (const [name, handler] of Object.entries(pageEvents)) page.on(name, handler);
  };
  async function serve(request, response) {
    let job = null;let slot = false;let closed = false;
    const closedByBrowser = () => {
      if (response.writableEnded || stopped) return;
      closed = true;
      if (!job) return;
      job.httpClosed = true;
      if (job !== compileJob) { job.reject(new Error('PLAYGROUND-UI-TRANSPORT'));broken(null, 'INTERCEPT_RESPONSE_CLOSE'); }
      else abortIfProved(job);
    };
    response.on('close', closedByBrowser);request.on('aborted', closedByBrowser);
    try {
      if (stopped || ++admitted > bounds.requests || reading + pending.size >= maximum) fail('INTERCEPT_001');
      reading++;slot = true;
      if (request.socket.localAddress !== '127.0.0.1' || request.socket.remoteAddress !== '127.0.0.1' ||
          request.socket.localPort !== Number(target.port) || request.socket.remoteFamily !== 'IPv4' || request.headers.host !== target.host) fail('INTERCEPT_001');
      const name = routeFor(selectedOrigin + request.url, request.method, selectedOrigin), route = routes[name];
      const headers = selectNetworkHeaders(request);const declared = request.headers['content-length'];
      if (request.headers['content-encoding'] || declared !== undefined && (!/^(0|[1-9][0-9]*)$/.test(declared) || Number(declared) > route.input)) fail('INTERCEPT_003');
      request.socket.setTimeout(name === 'compile' ? 12000 : name === 'evaluate' ? 7000 : 5000);
      const chunks = [];let bytes = 0;
      for await (const chunk of request) {
        bytes += chunk.length;totalIn += chunk.length;
        if (bytes > route.input || totalIn > bounds.aggregate) fail('INTERCEPT_003');chunks.push(chunk);
      }
      if (declared !== undefined && Number(declared) !== bytes || stopped || closed || serial >= bounds.requests || pending.size >= maximum) fail('INTERCEPT_004');
      const body = Buffer.concat(chunks);reading--;slot = false;
      job = { id: ++serial, name, response, bytes, sha256: sha256(body), state: 'waiting', paused: false,
        httpClosed: false, abortSent: false, responseSeen: false, resolve: null, reject: null };
      if (name === 'compile' && !corpusEnabled) { if (compileJob) fail('INTERCEPT_006');compileJob = job;if (compileRequest && !associated(job)) fail('INTERCEPT_006'); }
      const reply = new Promise((resolve, reject) => { job.resolve = resolve;job.reject = reject; });reply.catch(() => {});pending.set(job.id, job);
      await transmit({ version: 1, type: 'request', id: job.id, route: name, method: request.method, headers, bodyBase64: body.toString('base64') });
      const record = await reply;
      if (record.type === 'aborted') return;
      const actual = responseRecord(record, name);totalOut += actual.body.length;
      if (totalOut > bounds.aggregate || closed) fail('INTERCEPT_010');
      response.writeHead(record.status, loopbackResponseHeaders(record.status, actual));
      await new Promise((resolve, reject) => { response.once('error', error => reject(tagTransport(error, 'INTERCEPT_RESPONSE_ERROR')));response.once('close', () => response.writableFinished ? resolve() : reject(tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'), 'INTERCEPT_RESPONSE_INCOMPLETE')));response.end(actual.body, resolve); });
    } catch (error) { if (!stopped) broken(error, 'INTERCEPT_SERVE');response.destroy(); }
    finally { if (slot) reading--;if (job) pending.delete(job.id); }
  }
  const server = createServer({ maxHeaderSize: bounds.headers }, (request, response) => {
    const operation = serve(request, response);operations.add(operation);operation.finally(() => operations.delete(operation));
  });
  server.maxConnections = maximum;server.maxRequestsPerSocket = 1;server.headersTimeout = 5000;server.requestTimeout = 12000;server.keepAliveTimeout = 1;
  server.on('connection', socket => {
    if (stopped || sockets.size >= maximum) { socket.destroy();broken(null, 'INTERCEPT_SOCKET_LIMIT');return; }
    sockets.add(socket);socket.setTimeout(5000);socket.on('error', () => {});socket.on('timeout', () => { socket.destroy();if (!stopped) broken(null, 'INTERCEPT_SOCKET_TIMEOUT'); });socket.once('close', () => sockets.delete(socket));
  });
  server.on('drop', () => broken(null, 'INTERCEPT_DROP'));
  server.on('clientError', (_error, socket) => { socket.destroy();if (!stopped) broken(null, 'INTERCEPT_CLIENT_ERROR'); });server.on('error', error => broken(error, 'INTERCEPT_SERVER_ERROR'));
  try {
    await new Promise((resolve, reject) => { server.once('error', reject);server.listen({ host: '127.0.0.1', port: Number(target.port), exclusive: true }, () => { server.off('error', reject);resolve(); }); });
    const actual = server.address();if (!actual || actual.address !== '127.0.0.1' || actual.family !== 'IPv4' || actual.port !== Number(target.port)) fail('INTERCEPT_001');
  } catch (error) { requestObservations.stop();for (const socket of sockets) socket.destroy();server.close(() => {});throw error; }
  context.on('request', requestStarted);context.on('requestfailed', requestFailed);context.on('serviceworker', serviceWorker);context.on('page', pageStarted);
  return Object.freeze({
    failures, failureBranch() { return transportBranch(firstFailure); },invalidate() { broken(null, 'INTERCEPT_INVALIDATE'); },
    accept(record) {
      if (stopped) fail('INTERCEPT_005');
      if (record?.type === 'probe-paused') {
        envelope(record, 'probe-paused', ['id']);integer(record.id, bounds.requests);const job = pending.get(record.id);
        if (!job || job !== compileJob || job.paused || compileRequest && !associated(job)) fail('INTERCEPT_006');
        job.paused = true;abortIfProved(job);return;
      }
      if (record?.type === 'aborted') envelope(record, 'aborted', ['id']);else if (record?.type !== 'response') fail('INTERCEPT_007');
      integer(record.id, bounds.requests);const job = pending.get(record.id);
      if (!job) {
        const terminal = completed.get(record.id);
        if (!terminal?.aborted || record.type !== 'response' || terminal.responseSeen) fail('INTERCEPT_008');
        responseRecord(record, terminal.name);terminal.responseSeen = true;return;
      }
      if (!['waiting', 'aborting'].includes(job.state) || record.type === 'aborted' && job.state !== 'aborting') fail('INTERCEPT_009');
      if (record.type === 'response') { if (job.responseSeen) fail('INTERCEPT_010');responseRecord(record, job.name);job.responseSeen = true;if (job.state === 'aborting') return; }
      else {
        if (!associated(job) || !job.paused || !job.httpClosed || !compileRequest.failed) fail('INTERCEPT_009');
        aborts.push(Object.freeze({ id: job.id, route: 'compile', paused: true, event: 'requestfailed', acknowledged: true,
          httpResponseClosed: true, requestBytes: job.bytes, requestSha256: job.sha256 }));
      }
      completed.set(job.id, { name: job.name, aborted: record.type === 'aborted', responseSeen: job.responseSeen });job.state = 'terminal';job.resolve(record);
    },
    probeState() { return compileJob?.paused && associated(compileJob) ? Object.freeze({ id: compileJob.id, state: compileJob.state }) : null; },
    abortProof() { return structuredClone(aborts); },
    beginCorpus() {
      if (stopped || corpusEnabled || aborts.length !== 1 || compileJob?.state !== 'terminal' || pending.has(compileJob.id)) fail('INTERCEPT_006');
      corpusEnabled = true;
      return transmit({ version: 1, type: 'corpus-begin', id: compileJob.id });
    },
    stop() {
      if (stopping) return stopping;stopped = true;requestObservations.stop();
      context.off('request', requestStarted);context.off('requestfailed', requestFailed);context.off('serviceworker', serviceWorker);context.off('page', pageStarted);
      for (const page of pages) for (const [name, handler] of Object.entries(pageEvents)) page.off(name, handler);
      const closed = new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
      for (const job of pending.values()) job.reject(new Error('PLAYGROUND-UI-STOPPED'));
      for (const socket of sockets) socket.destroy();server.closeAllConnections();
      stopping = (async () => { let timer;try {
        await Promise.race([Promise.all([closed, ...operations, ...transmissions]), new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('PLAYGROUND-UI-CLEANUP')), 2000); })]);
      } finally { clearTimeout(timer); } })();return stopping;
    },
  });
}
