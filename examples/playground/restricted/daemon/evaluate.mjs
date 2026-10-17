// Fixed evaluation executable inside the independently authenticated, isolated image.
import { admitEvaluation, evaluationInputLimit, readImageFile } from './evaluation-input.mjs';
import { evaluatePage } from './evaluation-page.mjs';
import { evaluationServer } from './evaluation-server.mjs';
import { validateObservation } from '../evaluation.mjs';
import { fail } from '../limits.mjs';

export async function evaluateInBrowser({ chromium, inputBytes, identityBytes, templateBytes }) {
  const payload = await admitEvaluation(inputBytes, identityBytes, templateBytes);
  const server = await evaluationServer(payload, templateBytes);
  let browser;
  let observation;
  let primary;
  const cleanup = [];
  try {
    browser = await chromium.launch({ executablePath: '/materials/browser/chrome-linux64/chrome',
      headless: true, chromiumSandbox: true, timeout: 5000,
      env: { PATH: '/materials/runtime/node/bin', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8',
        HOME: '/tmp', TMPDIR: '/tmp', XDG_CONFIG_HOME: '/tmp/chrome-config',
        XDG_CACHE_HOME: '/tmp/chrome-cache', TZ: 'UTC', SOURCE_DATE_EPOCH: '0' } });
    // Chromium's own default sandbox remains mandatory. Never permit a sandbox-disabling switch.
    const context = await browser.newContext({ serviceWorkers: 'block', javaScriptEnabled: true });
    const page = await context.newPage();
    await page.goto(server.origin, { waitUntil: 'load', timeout: 5000 });
    observation = await page.evaluate(evaluatePage);
    validateObservation(observation, payload.input.selection);
  } catch (error) { primary = error; }
  finally {
    try { await browser?.close(); } catch (error) { cleanup.push(error); }
    try { await server.close(); } catch (error) { cleanup.push(error); }
  }
  if (cleanup.length) fail('EVALUATION-CLEANUP');
  if (primary) throw primary;
  return observation;
}

export async function runEvaluationExecutable() {
  const { chromium } = await import('/materials/playwright/index.mjs');
  const inputBytes = await readImageFile('/input/request', evaluationInputLimit);
  const identityBytes = await readImageFile('/resources/evaluation-identity.json', 4096);
  const templateBytes = await readImageFile('/resources/loader.js', 65536);
  const observation = await evaluateInBrowser({ chromium, inputBytes, identityBytes, templateBytes });
  process.stdout.write(`${JSON.stringify(observation)}\n`);
}
