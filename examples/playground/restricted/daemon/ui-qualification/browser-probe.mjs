// First real-browser gate only. No corpus expansion until abort fidelity is actually proved.
import { setTimeout as pause } from 'node:timers/promises';
import { fail, start } from './wire.mjs';
import { interceptBrowser } from './browser-intercept.mjs';

async function until(predicate, milliseconds) {
  const deadline = performance.now() + milliseconds;
  do {
    const result = predicate();
    if (result) return result;
    await pause(5);
  } while (performance.now() < deadline);
  throw new Error('PLAYGROUND-UI-ABORT-UNPROVED');
}
function axSummary(nodes) {
  if (!Array.isArray(nodes) || nodes.length > 512) fail();
  return nodes.map(node => {
    const role = node.role?.value ?? ''; const name = node.name?.value ?? '';
    if (typeof role !== 'string' || role.length > 128 || typeof name !== 'string' || name.length > 4096) fail();
    return { role, name, ignored: node.ignored === true,
      properties: (node.properties ?? []).filter(value => ['focused', 'live', 'invalid', 'disabled'].includes(value.name))
        .map(value => ({ name: value.name, value: value.value?.value ?? null })) };
  });
}
export async function runInitialBrowserProbe({ chromium, configuration, send, register, signal }) {
  const selected = start(configuration);
  let browser; let context; let intercept; let primary; let result;
  const cleanup = [];
  let cancel;
  const cancelled = new Promise((_, reject) => {
    cancel = () => reject(new Error('PLAYGROUND-UI-STOPPED'));
    signal?.addEventListener('abort', cancel, { once: true });
    if (signal?.aborted) cancel();
  });
  cancelled.catch(() => {});
  try {
    if (signal?.aborted) throw new Error('PLAYGROUND-UI-STOPPED');
    browser = await chromium.launch({ executablePath: '/materials/browser/chrome-linux64/chrome',
      chromiumSandbox: true, headless: true, timeout: 5000,
      env: { PATH: '/materials/runtime/node/bin', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', TZ: 'UTC',
        HOME: '/tmp', TMPDIR: '/tmp', XDG_CONFIG_HOME: '/tmp/chrome-config', XDG_CACHE_HOME: '/tmp/chrome-cache' } });
    if (signal?.aborted) throw new Error('PLAYGROUND-UI-STOPPED');
    context = await browser.newContext({ serviceWorkers: 'block', acceptDownloads: false,
      viewport: { width: 1280, height: 960 }, reducedMotion: 'reduce' });
    intercept = await interceptBrowser(context, selected.origin, send);
    register(record => intercept.accept(record));
    const work = (async () => {
      const page = await context.newPage();
      page.setDefaultTimeout(5000);
      const cdp = await context.newCDPSession(page);
      await cdp.send('Accessibility.enable');
      const updates = [];
      cdp.on('Accessibility.nodesUpdated', event => {
        try { if (updates.length < 32) updates.push(axSummary(event.nodes)); }
        catch { intercept.invalidate(); }
      });
      const scripts = new Set();
      await cdp.send('Debugger.enable');
      cdp.on('Debugger.scriptParsed', event => {
        if (event.url.startsWith(selected.origin + '/examples/playground/restricted/') && scripts.size < 32) {
          scripts.add(event.url.slice(selected.origin.length));
        } else if (/^(?:blob:|data:|wasm:)/.test(event.url)) intercept.invalidate();
      });
      await page.goto(`${selected.origin}${selected.launchPath}#${selected.capability}`, { waitUntil: 'load', timeout: 10000 });
      const source = page.getByRole('textbox', { name: 'Source', exact: true });
      await source.waitFor();
      await page.waitForFunction(() => !document.getElementById('source').disabled, undefined, { timeout: 5000 });
      const initial = await page.evaluate(() => ({ pathname: location.pathname, hashRemoved: location.hash === '',
        title: document.title, status: document.getElementById('status').textContent,
        role: document.getElementById('status').getAttribute('role'),
        live: document.getElementById('status').getAttribute('aria-live') }));
      if (!initial.hashRemoved || initial.pathname !== selected.launchPath ||
          initial.title !== 'Zryna source playground' || initial.role !== 'status' || initial.live !== 'polite') fail();
      await source.focus();
      const before = await cdp.send('Accessibility.getFullAXTree');
      await source.fill('export function add(a: i32, b: i32): i32 { return a + b; }\n');
      await source.press('Control+Enter');
      const pending = await until(() => intercept.probeState(), 1000);
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      const abort = await until(() => intercept.abortProof().find(value => value.id === pending.id), 1000);
      if (!abort.paused || !abort.acknowledged || abort.event !== 'requestfailed') fail();
      await page.getByRole('status').filter({ hasText: 'Cancelled. The previous operation has finished cleanup.' }).waitFor();
      const after = await cdp.send('Accessibility.getFullAXTree');
      const state = await page.evaluate(() => ({ focusedId: document.activeElement?.id ?? '',
        status: document.getElementById('status').textContent,
        compileDisabled: document.getElementById('compile').disabled,
        runDisabled: document.getElementById('run').disabled,
        diagnosticChildren: document.getElementById('diagnostics').children.length }));
      if (state.compileDisabled || !state.runDisabled || state.diagnosticChildren !== 0) fail();
      result = { version: 1, type: 'result', runId: selected.runId,
        phase: 'initial-abort-static-ax', transport: 'operator-stdio-http-interception-v1',
        abort, initial, state, scripts: [...scripts].sort(),
        axBefore: axSummary(before.nodes), axAfter: axSummary(after.nodes), axUpdates: updates,
        corpus: 'unrun', audibleAssistiveTechnology: 'unrun', browserClosed: false };
      if (Buffer.byteLength(JSON.stringify(result)) > 1048576) fail();
    })();
    await Promise.race([work, intercept.failures, cancelled]);
  } catch (error) { primary = error; }
  finally {
    signal?.removeEventListener('abort', cancel);
    try { await intercept?.stop(); } catch (error) { cleanup.push(error); }
    try { await context?.close(); } catch (error) { cleanup.push(error); }
    try { await browser?.close(); } catch (error) { cleanup.push(error); }
  }
  if (cleanup.length) throw new AggregateError([...(primary ? [primary] : []), ...cleanup], 'PLAYGROUND-UI-CLEANUP');
  if (primary) throw primary;
  result.browserClosed = true;
  return result;
}
