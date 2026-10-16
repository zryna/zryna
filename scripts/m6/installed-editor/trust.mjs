import assert from 'node:assert/strict';
import { lstat, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';
import { remaining } from './process.mjs';

export async function waitJson(path, deadline, failed = () => null) {
  for (;;) {
    remaining(deadline);
    try {
      const marker = await lstat(path + '.ready');
      assert(marker.isFile() && !marker.isSymbolicLink() && marker.size === 0);
      const stat = await lstat(path);
      assert(stat.isFile() && !stat.isSymbolicLink() && stat.size <= 1048576);
      return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(await readFile(path)));
    }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (failed()) throw failed();
    await delay(Math.min(50, remaining(deadline)));
  }
}

export function debuggerEndpoint(text) {
  const [port, path, ...extra] = text.trim().split(/\r?\n/);
  assert(extra.length === 0 && /^[1-9][0-9]{0,4}$/.test(port));
  assert(Number(port) <= 65535 && /^\/devtools\/browser\/[A-Za-z0-9-]+$/.test(path));
  return `ws://127.0.0.1:${port}${path}`;
}

export async function trustFreshFolder({ userData, workspaceName, initialPath, deadline, failed }) {
  const initial = await waitJson(initialPath, deadline, failed);
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
  try {
    const pages = browser.contexts().flatMap(context => context.pages());
    const selected = [];
    for (const page of pages) {
      const dialog = page.getByRole('dialog').filter({ hasText: workspaceName });
      if (await dialog.count() === 1) selected.push({ page, dialog });
    }
    assert.equal(selected.length, 1, 'Exact fresh workspace trust dialog required.');
    const { dialog } = selected[0];
    const parents = dialog.getByRole('checkbox');
    for (const checkbox of await parents.all()) {
      assert.equal(await checkbox.isChecked(), false, 'Parent-folder trust must stay unselected.');
    }
    const button = dialog.getByRole('button', { name: 'Yes, I trust the authors', exact: true });
    assert.equal(await button.count(), 1);
    await button.click({ timeout: remaining(deadline) });
    return { initialTrusted: false, action: 'Exact fresh-folder normal trust dialog accepted',
      parentTrustSelected: false, workspaceName, interaction: 'Actual pinned Playwright/CDP workbench UI' };
  } finally {
    // Disconnect the trusted harness connection; workbench shutdown belongs to the runner/owner.
    await browser.close();
  }
}
