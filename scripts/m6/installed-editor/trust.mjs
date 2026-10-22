import assert from 'node:assert/strict';
import { lstat, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';
import { remaining } from './process.mjs';

async function markedJson(path) {
  try {
      const marker = await lstat(path + '.ready');
      assert(marker.isFile() && !marker.isSymbolicLink() && marker.size === 0);
      const stat = await lstat(path);
      assert(stat.isFile() && !stat.isSymbolicLink() && stat.size <= 1048576);
      return { value: JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(await readFile(path))) };
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  return null;
}

export async function waitJson(path, deadline, failed = () => null, failurePath = null) {
  for (;;) {
    remaining(deadline);
    if (failurePath) {
      const failure = await markedJson(failurePath);
      if (failure) {
        assert.equal(failure.value.failed, true, 'Marked host failure must identify a failure.');
        throw new Error(`Installed editor host failed: ${JSON.stringify(failure.value)}`);
      }
    }
    const result = await markedJson(path);
    if (result) return result.value;
    const error = failed();
    if (error) throw error;
    await delay(Math.min(50, remaining(deadline)));
  }
}

export function debuggerEndpoint(text) {
  const [port, path, ...extra] = text.trim().split(/\r?\n/);
  assert(extra.length === 0 && /^[1-9][0-9]{0,4}$/.test(port));
  assert(Number(port) <= 65535 && /^\/devtools\/browser\/[A-Za-z0-9-]+$/.test(path));
  return `ws://127.0.0.1:${port}${path}`;
}

// The extension host can publish its untrusted observation before the workbench
// has exposed a dialog target. Re-enumerate targets under the original deadline.
export async function waitTrustDialog(pages, workspaceName, deadline, failed, observe = () => {}) {
  let last = [];
  try {
    for (;;) {
      remaining(deadline);
      const error = await failed();
      if (error) throw error;
      const current = pages();
      assert(current.length <= 16, 'Fresh editor page bound exceeded.');
      const selected = [];
      last = [];
      for (const page of current) {
        const dialog = page.getByRole('dialog').filter({ hasText: workspaceName });
        const matches = await dialog.count();
        remaining(deadline);
        const row = { url: page.url().slice(0, 2048), matchingDialogs: matches };
        if (matches === 0) {
          row.dom = await page.evaluate(() => ({ title: document.title.slice(0, 256),
            bodyText: (document.body?.innerText ?? '').slice(0, 8192),
            dialogs: [...document.querySelectorAll('[role="dialog"],[role="alertdialog"]')]
              .slice(0, 4).map(node => ({ role: node.getAttribute('role'),
                label: (node.getAttribute('aria-label') ?? '').slice(0, 256),
                text: (node.textContent ?? '').slice(0, 4096) })),
          }));
          remaining(deadline);
        }
        last.push(row);
        assert(matches <= 1, 'Ambiguous fresh workspace trust dialog.');
        if (matches === 1) selected.push({ page, dialog });
      }
      observe(last);
      assert(selected.length <= 1, 'Ambiguous fresh workspace trust page.');
      if (selected.length === 1) return selected[0];
      await delay(Math.min(50, remaining(deadline)));
    }
  } catch (error) {
    error.trustDialogObservation = { workspaceName, pages: last };
    throw error;
  }
}

export async function trustFreshFolder({ userData, workspaceName, initialPath, failurePath,
  deadline, failed, output }) {
  const initial = await waitJson(initialPath, deadline, failed, failurePath);
  assert.equal(initial.initialTrusted, false, 'Fresh workspace must first be observed untrusted.');
  let endpoint;
  for (;;) {
    remaining(deadline);
    if (failed()) throw failed();
    try { endpoint = debuggerEndpoint(await readFile(join(userData, 'DevToolsActivePort'), 'utf8')); break; }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    await delay(Math.min(50, remaining(deadline)));
  }
  const browser = await chromium.connectOverCDP(endpoint, { timeout: remaining(deadline), noDefaults: true });
  let observation = [];
  try {
    const { dialog } = await waitTrustDialog(
      () => browser.contexts().flatMap(context => context.pages()), workspaceName, deadline,
      async () => {
        if (failurePath) {
          const failure = await markedJson(failurePath);
          if (failure) return new Error(`Installed editor host failed: ${JSON.stringify(failure.value)}`);
        }
        return failed();
      }, pages => { observation = pages; });
    const parents = dialog.getByRole('checkbox');
    for (const checkbox of await parents.all()) {
      assert.equal(await checkbox.isChecked(), false, 'Parent-folder trust must stay unselected.');
    }
    const button = dialog.getByRole('button', { name: 'Yes, I trust the authors', exact: true });
    assert.equal(await button.count(), 1);
    await button.click({ timeout: remaining(deadline) });
    return { initialTrusted: false, action: 'Exact fresh-folder normal trust dialog accepted',
      parentTrustSelected: false, workspaceName, interaction: 'Actual pinned Playwright/CDP workbench UI' };
  } catch (error) {
    if (output) await writeFile(join(output, 'trust-dialog-failure.json'),
      `${JSON.stringify({ workspaceName, pages: error.trustDialogObservation?.pages ?? observation, error: String(error.stack ?? error) }, null, 2)}\n`,
      { flag: 'wx' });
    throw error;
  } finally {
    // Disconnect the trusted harness connection; workbench shutdown belongs to the runner/owner.
    await browser.close();
  }
}
