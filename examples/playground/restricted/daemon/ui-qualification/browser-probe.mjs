// The corpus starts only after this run independently proves the genuine initial abort.
import { setTimeout as pause } from 'node:timers/promises';
import { start, transportBranch, emptyBrowserProcess, classifyBrowserError } from './wire.mjs';
import { observeBrowserProcess } from './browser-diagnostics.mjs';
export { observeBrowserProcess } from './browser-diagnostics.mjs';
import { interceptBrowser } from './browser-intercept.mjs';
import { runBrowserCorpus } from './browser-corpus.mjs';

const causes = new WeakMap();
const noCleanup = () => ({ intercept: false, context: false, browser: false });
const noLifecycle = () => ({ browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false });
function rejected(cause) { const error = new Error('PLAYGROUND-UI-PROBE-CHECK'); causes.set(error,
  { cause, cleanupFailed: false, cleanup: noCleanup(), lifecycle: noLifecycle(), process: emptyBrowserProcess(), branch: 'UNSPECIFIED' }); return error; }
export function browserFailure(error) {
  const retained = causes.get(error);
  if (retained) return Object.freeze(structuredClone(retained));
  return Object.freeze({ cause: classifyBrowserError(error), cleanupFailed: false, cleanup: noCleanup(), lifecycle: noLifecycle(), process: emptyBrowserProcess(), branch: transportBranch(error) });
}

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
  if (!Array.isArray(nodes) || nodes.length > 512) throw rejected('AX_NODES');
  return nodes.map(node => {
    const role = node.role?.value ?? ''; const name = node.name?.value ?? '';
    if (typeof role !== 'string' || role.length > 128 || typeof name !== 'string' || name.length > 4096) throw rejected('AX_NODE_TEXT');
    return { role, name, ignored: node.ignored === true,
      properties: (node.properties ?? []).filter(value => ['focused', 'live', 'invalid', 'disabled'].includes(value.name))
        .map(value => ({ name: value.name, value: value.value?.value ?? null })) };
  });
}
export async function runInitialBrowserProbe({ chromium, configuration, send, register, signal, checkpoint = () => {}, onFailure = () => {}, onCleanup = () => {} }) {
  const selected = start(configuration);
  let browser; let context; let intercept; let primary; let result;
  let firstBranch = 'UNSPECIFIED';let processObservation = emptyBrowserProcess();let stopProcess;
  const cleanup = [];
  const cleanupBits = noCleanup();const lifecycle = noLifecycle();let observing = true;
  const observed = name => () => { if (observing) lifecycle[name] = true; };
  const mark = value => { if (!primary) checkpoint(value); };
  let cancel;
  const cancelled = new Promise((_, reject) => {
    cancel = () => reject(new Error('PLAYGROUND-UI-STOPPED'));
    signal?.addEventListener('abort', cancel, { once: true });
    if (signal?.aborted) cancel();
  });
  cancelled.catch(() => {});
  try {
    if (signal?.aborted) throw new Error('PLAYGROUND-UI-STOPPED');
    mark('BROWSER_LAUNCH');
    browser = await chromium.launch({ executablePath: '/materials/browser/chrome-linux64/chrome',
      chromiumSandbox: true, headless: true, timeout: 5000,
      env: { PATH: '/materials/runtime/node/bin', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', TZ: 'UTC',
        HOME: '/tmp', TMPDIR: '/tmp', XDG_CONFIG_HOME: '/tmp/chrome-config', XDG_CACHE_HOME: '/tmp/chrome-cache' } });
    stopProcess = observeBrowserProcess(browser);
    browser.on('disconnected', observed('browserDisconnected'));
    if (signal?.aborted) throw new Error('PLAYGROUND-UI-STOPPED');
    mark('CONTEXT');
    context = await browser.newContext({ serviceWorkers: 'block', acceptDownloads: false,
      viewport: { width: 1280, height: 960 }, reducedMotion: 'reduce' });
    context.on('close', observed('contextClosed'));
    mark('INTERCEPT');intercept = await interceptBrowser(context, selected.origin, send);
    register(record => intercept.accept(record));
    const work = (async () => {
      mark('PAGE');const page = await context.newPage();
      page.on('close', observed('pageClosed'));page.on('crash', observed('pageCrashed'));
      page.setDefaultTimeout(5000);
      mark('NAVIGATE');await page.goto(`${selected.origin}${selected.launchPath}#${selected.capability}`, { waitUntil: 'load', timeout: 10000 });
      mark('SOURCE_READY');
      const source = page.getByRole('textbox', { name: 'Source', exact: true });
      await source.waitFor();
      await page.waitForFunction(() => !document.getElementById('source').disabled, undefined, { timeout: 5000 });
      mark('CDP_ATTACH');const cdp = await context.newCDPSession(page);
      const updates = [];
      cdp.on('Accessibility.nodesUpdated', event => {
        try { if (updates.length < 32) updates.push(axSummary(event.nodes)); }
        catch (error) { primary ??= error;intercept.invalidate(); }
      });
      mark('AX_ENABLE');await cdp.send('Accessibility.enable');
      const scripts = new Set();
      cdp.on('Debugger.scriptParsed', event => {
        if (event.url.startsWith(selected.origin + '/examples/playground/restricted/') && scripts.size < 32) {
          scripts.add(event.url.slice(selected.origin.length));
        } else if (/^(?:blob:|data:|wasm:)/.test(event.url)) {
          primary ??= rejected('SCRIPT_SOURCE');intercept.invalidate();
        }
      });
      mark('DEBUGGER_ENABLE');await cdp.send('Debugger.enable');
      mark('INITIAL_STATE');const initial = await page.evaluate(() => ({ pathname: location.pathname, hashRemoved: location.hash === '',
        title: document.title, status: document.getElementById('status').textContent,
        role: document.getElementById('status').getAttribute('role'),
        live: document.getElementById('status').getAttribute('aria-live') }));
      if (!initial.hashRemoved) throw rejected('INITIAL_HASH');
      if (initial.pathname !== selected.launchPath) throw rejected('INITIAL_PATH');
      if (initial.title !== 'Zryna source playground') throw rejected('INITIAL_TITLE');
      if (initial.role !== 'status') throw rejected('INITIAL_ROLE');
      if (initial.live !== 'polite') throw rejected('INITIAL_LIVE');
      mark('AX_BEFORE');
      await source.focus();
      const before = await cdp.send('Accessibility.getFullAXTree');
      mark('SOURCE_EDIT');await source.fill('export function add(a: i32, b: i32): i32 { return a + b; }\n');
      await source.press('Control+Enter');
      mark('COMPILE_PAUSED');const pending = await until(() => intercept.probeState(), 1000);
      mark('CANCEL_CLICK');await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      mark('ABORT_PROOF');const abort = await until(() => intercept.abortProof().find(value => value.id === pending.id), 1000);
      if (!abort.paused || !abort.acknowledged || abort.event !== 'requestfailed') throw rejected('ABORT_EVENT');
      mark('CANCELLED_STATE');
      await page.getByRole('status').filter({ hasText: 'Cancelled. The previous operation has finished cleanup.' }).waitFor();
      mark('AX_AFTER');const after = await cdp.send('Accessibility.getFullAXTree');
      mark('FINAL_STATE');const state = await page.evaluate(() => ({ focusedId: document.activeElement?.id ?? '',
        status: document.getElementById('status').textContent,
        compileDisabled: document.getElementById('compile').disabled,
        runDisabled: document.getElementById('run').disabled,
        diagnosticChildren: document.getElementById('diagnostics').children.length }));
      if (state.compileDisabled || !state.runDisabled || state.diagnosticChildren !== 0) throw rejected('FINAL_CONTROLS');
      await intercept.beginCorpus();
      const corpus = await runBrowserCorpus({ page, cdp, source, checkpoint: mark, summarizeAX: axSummary });
      mark('RESULT');
      result = { version: 1, type: 'result', runId: selected.runId,
        phase: 'initial-abort-corpus-ax', transport: 'operator-stdio-browser-loopback-http-v1',
        abort, initial, state, scripts: [...scripts].sort(),
        axBefore: axSummary(before.nodes), axAfter: axSummary(after.nodes), axUpdates: updates,
        corpus, audibleAssistiveTechnology: 'unrun', browserClosed: false };
      if (Buffer.byteLength(JSON.stringify(result)) > 1048576) throw rejected('RESULT_SIZE');
    })();
    await Promise.race([work, intercept.failures, cancelled]);
  } catch (error) {
    primary ??= error;firstBranch = intercept?.failureBranch() ?? 'UNSPECIFIED';
    if (firstBranch === 'UNSPECIFIED') firstBranch = transportBranch(primary);
    try { onFailure(primary, firstBranch); } catch { /* Observation cannot interrupt owned cleanup. */ }
  }
  finally {
    observing = false;processObservation = stopProcess?.() ?? processObservation;
    signal?.removeEventListener('abort', cancel);
    try { await intercept?.stop(); } catch (error) { cleanupBits.intercept = true;cleanup.push(error); }
    try { await context?.close(); } catch (error) { cleanupBits.context = true;cleanup.push(error); }
    try { await browser?.close(); } catch (error) { cleanupBits.browser = true;cleanup.push(error); }
  }
  try { onCleanup(Object.freeze(structuredClone({ cleanupFailed: cleanup.length > 0,
    cleanup: cleanupBits, lifecycle, process: processObservation }))); }
  catch { /* Observation cannot replace the primary failure or cleanup outcome. */ }
  if (cleanup.length) {
    const error = new AggregateError([...(primary ? [primary] : []), ...cleanup], 'PLAYGROUND-UI-CLEANUP');
    causes.set(error, { ...browserFailure(primary), branch: firstBranch, cleanupFailed: true, cleanup: cleanupBits, lifecycle, process: processObservation });throw error;
  }
  if (primary) { causes.set(primary, { ...browserFailure(primary), branch: firstBranch, lifecycle, process: processObservation });throw primary; }
  result.browserClosed = true;
  return result;
}
