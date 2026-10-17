// Actual controls only. Report authentication and scalar oracles are checked independently by the host.
import { runPresentationNegative } from './browser-presentation-negative.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';

const check = condition => { if (!condition) throw new Error('PLAYGROUND-UI-CORPUS-CHECK'); };
async function idle(page, status, timeout) {
  await page.waitForFunction(expected => document.getElementById('status').textContent === expected &&
    !document.getElementById('compile').disabled && document.getElementById('cancel').disabled,
  status, { timeout });
}
async function compilationView(page) {
  return page.evaluate(() => ({ source: document.getElementById('source').value,
    status: document.getElementById('status').textContent, identity: document.getElementById('identity').textContent,
    runDisabled: document.getElementById('run').disabled,
    controls: { compileDisabled: document.getElementById('compile').disabled, cancelDisabled: document.getElementById('cancel').disabled, selectedExport: document.getElementById('export').value },
    exports: [...document.getElementById('export').options].map(option => ({ value: option.value, text: option.textContent })),
    diagnostics: [...document.getElementById('diagnostics').children].map(item =>
      [...item.children].map(child => ({ tag: child.tagName, text: child.textContent }))),
    unexpectedDiagnosticElements: document.getElementById('diagnostics').querySelectorAll('script,img,iframe,svg,a,style').length }));
}
async function selectedSpan(page, index) {
  const button = page.locator('#diagnostics > li').nth(index).getByRole('button');
  await button.focus();await page.keyboard.press('Enter');
  return page.evaluate(() => ({ focusedId: document.activeElement?.id ?? '',
    start: document.getElementById('source').selectionStart, end: document.getElementById('source').selectionEnd }));
}
export async function runBrowserCorpus({ page, cdp, source, checkpoint, summarizeAX, completion }) {
  const rows = [];let scalarIndex = 0;let invalidArgument = null, keyboard = null, revisionInvalidation = null;
  let sourceBudget = null, unicodeNavigation = null, argumentAX = null, invalidAX = null, diagnosticAX = null, inertSource = null;
  for (const [index, row] of qualificationCases.entries()) {
    checkpoint('CORPUS_EDIT');
    if (['unicode-navigation', 'inert-source-text'].includes(row.id)) await source.fill(row.source);
    else await page.getByRole('combobox', { name: 'Examples', exact: true }).selectOption(row.id);
    check(await source.inputValue() === row.source);
    const scriptsBefore = row.id === 'inert-source-text' ? await page.locator('script').count() : null;
    const edited = await page.evaluate(() => ({ runDisabled: document.getElementById('run').disabled,
      exports: document.getElementById('export').options.length, identity: document.getElementById('identity').textContent,
      observation: document.getElementById('observation').textContent }));
    check(edited.runDisabled && edited.exports === 0 && edited.identity === '' && edited.observation === 'No observation.');
    if (index === 1) revisionInvalidation = edited;
    checkpoint('CORPUS_COMPILE');
    let compileFence;
    if (index === 0) {
      await source.focus();await page.keyboard.press('Tab');
      keyboard = await page.evaluate(() => ({ tabFromSource: document.activeElement?.id ?? '' }));
      check(keyboard.tabFromSource === 'compile');compileFence = completion.beginPublication('COMPILE', index);await page.keyboard.press('Enter');
    } else { compileFence = completion.beginPublication('COMPILE', index);
      if (index === 1) await source.press('Control+Enter');
      else await page.getByRole('button', { name: 'Compile source', exact: true }).click(); }
    await idle(page, row.cases ? 'Compiled. Choose an export and run it.' :
      'Compilation rejected. Review the compiler diagnostics.', 15000);
    const view = await compilationView(page);check(view.unexpectedDiagnosticElements === 0);completion.publish(compileFence);
    const observations = [];
    for (const [caseIndex, expected] of scalarCases(row).entries()) {
      checkpoint('CORPUS_EVALUATE');
      await page.getByRole('combobox', { name: 'Export', exact: true }).selectOption(expected.logical);
      const fields = page.locator('#arguments input');check(await fields.count() === expected.args.length);
      for (const [argument, value] of expected.args.entries()) await fields.nth(argument).fill(String(value));
      let evaluationFence;
      if (index === 0 && caseIndex === 0) {
        argumentAX = summarizeAX((await cdp.send('Accessibility.getFullAXTree')).nodes);
        await fields.nth(0).fill('-0');await page.getByRole('button', { name: 'Run export', exact: true }).click();
        invalidArgument = await page.evaluate(() => ({ focusedId: document.activeElement?.id ?? '',
          invalid: document.getElementById('argument-0').getAttribute('aria-invalid'),
          error: document.getElementById('argument-error-0').textContent,
          observation: document.getElementById('observation').textContent }));
        check(invalidArgument.focusedId === 'argument-0' && invalidArgument.invalid === 'true');
        invalidAX = summarizeAX((await cdp.send('Accessibility.getFullAXTree')).nodes);
        await fields.nth(0).fill(String(expected.args[0]));
        await page.getByRole('button', { name: 'Run export', exact: true }).focus();evaluationFence = completion.beginPublication('EVALUATE', index, scalarIndex);await page.keyboard.press('Space');
      } else { evaluationFence = completion.beginPublication('EVALUATE', index, scalarIndex);await page.getByRole('button', { name: 'Run export', exact: true }).click(); }
      await idle(page, 'Evaluation completed.', 10000);
      const actual = await page.evaluate(() => ({ text: document.getElementById('observation').textContent,
        identity: document.getElementById('identity').textContent,
        selection: { source: document.getElementById('source').value, logical: document.getElementById('export').value,
          args: [...document.querySelectorAll('#arguments input')].map(input => input.value), status: document.getElementById('status').textContent,
          compileDisabled: document.getElementById('compile').disabled, cancelDisabled: document.getElementById('cancel').disabled, runDisabled: document.getElementById('run').disabled } }));
      check(actual.text === `${expected.logical}(${expected.args.join(', ')}) = ${expected.value} (signed i32)`);
      completion.publish(evaluationFence);scalarIndex++;
      observations.push({ logical: expected.logical, args: [...expected.args], value: expected.value, ...actual });
    }
    if (row.id === 'unicode-navigation') {
      checkpoint('UNICODE_NAVIGATION');
      const location = view.diagnostics.findIndex(children => children.some(child => child.tag === 'BUTTON'));
      check(location !== -1);const selected = await selectedSpan(page, location);
      unicodeNavigation = { diagnosticIndex: location, ...selected };
      check(selected.focusedId === 'source');
      diagnosticAX = summarizeAX((await cdp.send('Accessibility.getFullAXTree')).nodes);
    }
    if (row.id === 'inert-source-text') {
      inertSource = await page.evaluate(() => ({ sourceExact: document.getElementById('source').value ===
        '<img src=x onerror="globalThis.__zrynaMarkupRan=1">\n', imgElements: document.querySelectorAll('img').length,
        scriptElementsAfter: document.scripts.length, markerAbsent: !Object.hasOwn(globalThis, '__zrynaMarkupRan') }));
      inertSource.scriptElementsBefore = scriptsBefore;
      check(inertSource.sourceExact && inertSource.imgElements === 0 && inertSource.markerAbsent &&
        inertSource.scriptElementsAfter === scriptsBefore && scriptsBefore === 1);
    }
    rows.push({ id: row.id, ...view, observations });
  }
  checkpoint('SOURCE_BUDGET');
  await source.fill('é'.repeat(2049));
  sourceBudget = await page.evaluate(() => ({ bytes: document.getElementById('byte-count').textContent,
    invalid: document.getElementById('source').getAttribute('aria-invalid'),
    error: document.getElementById('source-error').textContent,
    compileDisabled: document.getElementById('compile').disabled, runDisabled: document.getElementById('run').disabled }));
  check(sourceBudget.invalid === 'true' && sourceBudget.compileDisabled && sourceBudget.runDisabled);
  await source.fill(qualificationCases[0].source);
  const recovered = await page.evaluate(() => ({ invalid: document.getElementById('source').getAttribute('aria-invalid'),
    error: document.getElementById('source-error').textContent, compileDisabled: document.getElementById('compile').disabled,
    runDisabled: document.getElementById('run').disabled,
    controls: { compileDisabled: document.getElementById('compile').disabled, cancelDisabled: document.getElementById('cancel').disabled, selectedExport: document.getElementById('export').value }, identity: document.getElementById('identity').textContent }));
  check(recovered.invalid === 'false' && recovered.error === '' && !recovered.compileDisabled && recovered.runDisabled && recovered.identity === '');
  checkpoint('PRESENTATION_NEGATIVE');const arbitraryInertMarkup = await runPresentationNegative(page);
  return { format: 'zryna.ui-corpus-observation.v1', version: 1, rows, keyboard, invalidArgument,
    revisionInvalidation, sourceBudget, recovered, unicodeNavigation, argumentAX, invalidAX, diagnosticAX, inertSource,
    sessionIsolation: 'unrun', arbitraryInertMarkup, audibleAssistiveTechnology: 'unrun' };
}
