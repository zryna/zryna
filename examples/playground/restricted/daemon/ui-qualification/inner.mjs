// Fixed private test entrypoint. Image/material auditing and ownership are external prerequisites.
import { createFrameDecoder, createFrameWriter } from './framing.mjs';
import { bounds, fail, hash, start, innerPhases, innerFailure } from './wire.mjs';
import { runInitialBrowserProbe, browserFailure } from './browser-probe.mjs';
import { readImagePlan } from './plan.mjs';

export async function runUiDriver(input, output, loadChromium, planSha256) {
  if (!hash(planSha256)) fail();
  const writer = createFrameWriter(output);
  const cancellation = new AbortController();
  let started = false; let accept = null; let terminal = false; let rejectStream;
  let configuration = null; let phase = 'LOAD_CHROMIUM';
  let failureObservation = null;
  const checkpoint = value => { if (!innerPhases.includes(value)) fail();phase = value; };
  const streamFailure = new Promise((_, reject) => { rejectStream = reject; });
  streamFailure.catch(() => {});
  let resolveStart;
  const initial = new Promise(resolve => { resolveStart = resolve; });
  const decoder = createFrameDecoder(record => {
    if (terminal) fail();
    if (!started) { started = true; resolveStart(start(record)); }
    else if (!accept) fail();
    else accept(record);
  });
  const data = chunk => { try { decoder.push(chunk); } catch (error) { rejectStream(error); } };
  const ended = () => {
    try { decoder.end(); } catch (error) { rejectStream(error); return; }
    if (!terminal) rejectStream(new Error('PLAYGROUND-UI-TRANSPORT'));
  };
  const failed = () => rejectStream(new Error('PLAYGROUND-UI-TRANSPORT'));
  input.on('data', data); input.on('end', ended); input.on('error', failed); output.on('error', failed);
  const timer = setTimeout(() => rejectStream(new Error('PLAYGROUND-UI-DEADLINE')), bounds.lifetimeMs);
  try {
    const candidate = await Promise.race([initial, streamFailure]);
    if (candidate.planSha256 !== planSha256) fail();
    configuration = candidate;
    const chromium = await loadChromium();
    const run = runInitialBrowserProbe({ chromium, configuration,
      send: record => writer.write(record), register: handler => { accept = handler; }, signal: cancellation.signal, checkpoint });
    let result;
    try { result = await Promise.race([run, streamFailure]); }
    catch (error) {
      cancellation.abort();
      try { await run; } catch (stopped) {
        const cleanup = browserFailure(stopped);
        failureObservation = { ...browserFailure(error), cleanupFailed: cleanup.cleanupFailed, cleanup: cleanup.cleanup };
      }
      throw error;
    }
    terminal = true; await writer.write(result); await writer.stop();
  } catch (error) {
    if (configuration && !terminal) {
      terminal = true;
      const record = innerFailure({ version: 1, type: 'failure', runId: configuration.runId,
        phase, ...(failureObservation ?? browserFailure(error)) }, configuration.runId);
      await writer.write(record);await writer.stop();
    }
    throw error;
  } finally {
    clearTimeout(timer); input.off('data', data); input.off('end', ended); input.off('error', failed);
    input.pause();
    output.off('error', failed);
  }
}

if (import.meta.url === 'file:///app/ui-driver.mjs') {
  Promise.resolve().then(() => runUiDriver(process.stdin, process.stdout, async () => {
    const { chromium } = await import('/materials/playwright/index.mjs');
    return chromium;
  }, readImagePlan())).catch(() => { process.stderr.write('PLAYGROUND-UI-FAILED\n'); process.exitCode = 1; });
}
