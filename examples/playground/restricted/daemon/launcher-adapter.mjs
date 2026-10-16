// Service reservation can precede host dispatch; never turn an undispatched request into an owner.
const fail = code => { throw new Error(`PLAYGROUND-${code}`); };
const tokenValid = token => typeof token === 'string' && /^[a-f0-9]{64}$/.test(token);

export function createLauncherHost(bridge) {
  let active = null;
  let closed = false;
  let poisoned = false;
  const safe = () => {
    if (closed || poisoned || bridge.recovery()) fail('HOST-TEARDOWN');
  };
  function dispatch(token, start) {
    safe();
    if (!tokenValid(token)) fail('JOB');
    if (active) fail('BUSY');
    const job = { token, started: false };
    active = job;
    const result = start();
    job.started = true;
    return result;
  }
  return Object.freeze({
    compile(bytes, signal, token) { return dispatch(token, () => bridge.compile(bytes, signal, token)); },
    evaluate(token, payload, signal) { return dispatch(token, () => bridge.evaluate(token, payload, signal)); },
    async finish(token, cancelled) {
      safe();
      if (!tokenValid(token) || typeof cancelled !== 'boolean') fail('JOB');
      if (!active) {
        // The service owns this token; no helper was dispatched and no uncertain owner exists.
        if (cancelled && bridge.idle()) return;
        fail('JOB');
      }
      if (active.token !== token) fail('JOB');
      if (!active.started) {
        if (!cancelled || !bridge.idle()) { poisoned = true; fail('HOST-TEARDOWN'); }
        active = null; return;
      }
      try { await bridge.finish(token, cancelled); }
      catch (error) { poisoned = true; throw error; }
      active = null;
    },
    recovery() { return bridge.recovery(); },
    async close() {
      if (closed) return;
      closed = true;
      try { await bridge.close(); }
      catch (error) { poisoned = true; throw error; }
      finally { if (!poisoned) active = null; }
    },
  });
}
