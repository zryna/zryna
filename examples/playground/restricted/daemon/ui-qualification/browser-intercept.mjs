// Executes inside the separately approved trusted UI-test image, not the EVAL image.
import { bounds, envelope, fail, integer, tagTransport, transportBranch } from './wire.mjs';
import { routeFor, routes, responseRecord, selectBrowserHeaders } from './routes.mjs';

export async function interceptBrowser(context, selectedOrigin, send) {
  let serial = 0; let stopped = false; let failure;let firstFailure = null;
  const pending = new Map(); const completed = new Map(); const aborts = []; const byRequest = new WeakMap();
  const failures = new Promise((_, reject) => { failure = reject; });
  // The driver always races this promise. Events must never become unhandled rejections.
  failures.catch(() => {});
  const broken = error => {
    firstFailure ??= tagTransport(error instanceof Error ? error : new Error('PLAYGROUND-UI-TRANSPORT'), 'INTERCEPT_EVENT');
    failure(firstFailure);
  };
  const requestFailed = request => {
    const id = byRequest.get(request); const job = pending.get(id);
    if (job && job.state === 'waiting') {
      job.state = 'aborting';
      Promise.resolve().then(() => send({ version: 1, type: 'abort', id })).catch(broken);
    }
  };
  context.on('requestfailed', requestFailed);
  context.on('serviceworker', broken);
  context.on('page', page => {
    page.on('worker', broken); page.on('websocket', broken); page.on('download', broken);
    page.on('popup', broken);
  });
  await context.route('**/*', async route => {
    const request = route.request();
    let job;let boundary = 'INTERCEPT_EVENT';
    try {
      if (stopped || pending.size >= bounds.live + bounds.queued) fail('INTERCEPT_001');
      const name = routeFor(request.url(), request.method(), selectedOrigin);
      if (!['document', 'script', 'stylesheet', 'fetch'].includes(request.resourceType())) fail('INTERCEPT_002');
      const body = request.postDataBuffer() ?? Buffer.alloc(0);
      if (body.length > routes[name].input) fail('INTERCEPT_003');
      boundary = 'INTERCEPT_HEADERS';const headers = selectBrowserHeaders(await request.allHeaders());
      // Header reads can resolve out of order. Reserve the ID and pending slot together
      // after that await, so send order stays strict and concurrent readers share no ID.
      if (stopped || pending.size >= bounds.live + bounds.queued || serial >= bounds.requests) fail('INTERCEPT_004');
      job = { id: ++serial, name, route, state: 'waiting', resolve: null, reject: null,
        paused: false, responseSeen: false, abortSent: false };
      const reply = new Promise((resolve, reject) => { job.resolve = resolve; job.reject = reject; });
      pending.set(job.id, job); byRequest.set(request, job.id);
      boundary = 'INTERCEPT_SEND';await send({ version: 1, type: 'request', id: job.id, route: name, method: request.method(),
        headers, bodyBase64: body.toString('base64') });
      boundary = 'INTERCEPT_WAIT';const record = await reply;
      if (record.type === 'aborted') {
        try { await route.abort('aborted'); } catch { /* Chromium may have already cancelled it. */ }
      } else {
        const actual = responseRecord(record, name);
        boundary = 'INTERCEPT_FULFILL';await route.fulfill({ status: record.status, headers: actual.headers, body: actual.body });
      }
    } catch (error) {
      firstFailure ??= tagTransport(error, boundary);
      try { await route.abort('failed'); } catch { /* original failure retained */ }
      broken(firstFailure);
    } finally { if (job) pending.delete(job.id); }
  });
  return Object.freeze({
    failures,
    failureBranch() { return transportBranch(firstFailure); },
    invalidate() { broken(); },
    accept(record) {
      if (stopped) fail('INTERCEPT_005');
      if (record?.type === 'probe-paused') {
        envelope(record, 'probe-paused', ['id']); integer(record.id, bounds.requests);
        const job = pending.get(record.id);
        if (!job || job.name !== 'compile' || job.paused) fail('INTERCEPT_006');
        job.paused = true; return;
      }
      if (record?.type === 'aborted') envelope(record, 'aborted', ['id']);
      else if (record?.type !== 'response') fail('INTERCEPT_007');
      integer(record.id, bounds.requests);
      const job = pending.get(record.id);
      if ((!job || job.state === 'terminal') && record.type === 'response') {
        const terminal = completed.get(record.id);
        if (!terminal || !terminal.aborted || terminal.responseSeen) fail('INTERCEPT_008');
        responseRecord(record, terminal.name); terminal.responseSeen = true; return;
      }
      if (!job || !['waiting', 'aborting'].includes(job.state) ||
          record.type === 'aborted' && job.state !== 'aborting') fail('INTERCEPT_009');
      if (record.type === 'response') {
        if (job.responseSeen) fail('INTERCEPT_010');
        responseRecord(record, job.name); job.responseSeen = true;
        if (job.state === 'aborting') return;
      } else {
        aborts.push(Object.freeze({ id: job.id, route: job.name, paused: job.paused,
          event: 'requestfailed', acknowledged: true }));
      }
      completed.set(job.id, { name: job.name, aborted: record.type === 'aborted', responseSeen: job.responseSeen });
      job.state = 'terminal'; job.resolve(record);
    },
    probeState() {
      const job = [...pending.values()].find(value => value.name === 'compile' && value.paused);
      return job ? Object.freeze({ id: job.id, state: job.state }) : null;
    },
    abortProof() { return structuredClone(aborts); },
    async stop() {
      stopped = true; context.off('requestfailed', requestFailed);
      for (const job of pending.values()) job.reject(new Error('PLAYGROUND-UI-STOPPED'));
      await context.unroute('**/*');
    },
  });
}
