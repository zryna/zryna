// Explicit synthetic data at the original presentation boundary; never compiler evidence.
import { exact, fail } from './wire.mjs';

const payload = '<img src=x onerror="globalThis.__zrynaPresentationProbe=1"><script>globalThis.__zrynaPresentationProbe=1</script>';
const ids = ['diagnostic', 'unavailable', 'error', 'identity'];
const countKeys = ['script', 'img', 'iframe', 'svg', 'object', 'embed', 'eventAttributes'];

export function validatePresentationNegative(value) {
  exact(value, ['format', 'version', 'fixture', 'actualCompilerResult', 'renderer', 'rows', 'liveDocumentUnchanged']);
  if (value.format !== 'zryna.ui-inert-presentation-negative.v1' || value.version !== 1 ||
      value.fixture !== 'SYNTHETIC_PRESENTATION_NEGATIVE' || value.actualCompilerResult !== false ||
      value.renderer !== 'ORIGINAL_MOUNT_PLAYGROUND' || value.liveDocumentUnchanged !== true ||
      !Array.isArray(value.rows) || value.rows.length !== ids.length) fail('CORPUS_DOM');
  for (const [index, row] of value.rows.entries()) {
    exact(row, ['id', 'revision', 'source', 'sourceInvalid', 'status', 'identity', 'diagnostics',
      'before', 'after', 'detachedDocument', 'markerAbsent']);
    if (row.id !== ids[index] || row.revision !== 2 || row.source !== payload || row.sourceInvalid !== 'false' ||
        row.detachedDocument !== true || row.markerAbsent !== true || !Array.isArray(row.diagnostics)) fail('CORPUS_DOM');
    for (const name of ['before', 'after']) {
      exact(row[name], countKeys);
      if (countKeys.some(key => !Number.isSafeInteger(row[name][key]) || row[name][key] < 0 || row[name][key] > 256)) fail('CORPUS_DOM');
    }
    if (row.before.script !== 1 || countKeys.some(key => row.before[key] !== row.after[key]) ||
        countKeys.filter(key => key !== 'script').some(key => row.before[key] !== 0)) fail('CORPUS_DOM');
    const diagnostic = row.id === 'diagnostic';
    const expected = diagnostic ? [
      { tag: 'P', text: `error: PRESENTATION-FIXTURE — ${payload}` },
      { tag: 'BUTTON', text: `${payload}:1:1–1:1 (UTF-8 bytes 0–0)` },
      { tag: 'P', text: payload },
    ] : [];
    if (row.diagnostics.length !== expected.length) fail('CORPUS_DOM');
    for (const [i, child] of row.diagnostics.entries()) {
      exact(child, ['tag', 'text']);
      if (child.tag !== expected[i].tag || child.text !== expected[i].text) fail('CORPUS_DOM');
    }
    const status = diagnostic ? 'Compilation rejected. Review the compiler diagnostics.' :
      row.id === 'unavailable' ? `Compiler unavailable: PLAYGROUND-UNAVAILABLE — ${payload}` :
      row.id === 'error' ? `Compilation failed: ${payload}` : 'Compiled. Choose an export and run it.';
    const identity = row.id === 'identity' ? `Source revision 2; source SHA-256 ${payload}; component SHA-256 ${payload}` : '';
    if (row.status !== status || row.identity !== identity) fail('CORPUS_DOM');
  }
  return structuredClone(value);
}

export async function runPresentationNegative(page) {
  const observed = await page.evaluate(async () => {
    // This exact original module has already mounted the live, authenticated playground.
    const mainUrl = new URL('/examples/playground/restricted/main.mjs', location.origin).href;
    if (![...document.scripts].some(script => script.type === 'module' && script.src === mainUrl)) throw new Error('PRESENTATION-MODULE');
    const { mountPlayground } = await import(mainUrl);
    const text = '<img src=x onerror="globalThis.__zrynaPresentationProbe=1"><script>globalThis.__zrynaPresentationProbe=1</script>';
    const markerAbsent = () => !Object.hasOwn(globalThis, '__zrynaPresentationProbe');
    const counts = doc => {
      const nodes = [...doc.querySelectorAll('*')];let attributes = 0, eventAttributes = 0;
      if (nodes.length > 1024) throw new Error('PRESENTATION-NODES');
      for (const node of nodes) for (const attribute of node.attributes) {
        if (++attributes > 8192) throw new Error('PRESENTATION-ATTRIBUTES');
        if (/^on/i.test(attribute.name)) eventAttributes++;
      }
      return Object.fromEntries([
        ...['script', 'img', 'iframe', 'svg', 'object', 'embed'].map(tag => [tag, doc.querySelectorAll(tag).length]),
        ['eventAttributes', eventAttributes],
      ]);
    };
    // Complete live markup plus mutable form/focus/selection state remain RAM-only.
    const live = () => {
      const html = document.documentElement.outerHTML;
      const controls = [...document.querySelectorAll('input,textarea,button,select,option')];
      if (html.length > 131072 || controls.length > 256) throw new Error('PRESENTATION-LIVE-BOUND');
      const values = JSON.stringify(controls.map(node => ({ value: node.value, disabled: node.disabled,
        checked: node.checked, selected: node.selected, selectedIndex: node.selectedIndex,
        start: node.selectionStart, end: node.selectionEnd, direction: node.selectionDirection })));
      if (values.length > 65536) throw new Error('PRESENTATION-VALUES');
      const selection = document.getSelection(), positions = [];
      if (selection) {
        if (selection.rangeCount > 16) throw new Error('PRESENTATION-SELECTION');
        positions.push(selection.anchorNode, selection.anchorOffset, selection.focusNode, selection.focusOffset);
        for (let i = 0; i < selection.rangeCount; i++) {
          const range = selection.getRangeAt(i);
          positions.push(range.startContainer, range.startOffset, range.endContainer, range.endOffset);
        }
      }
      return { html, values, active: document.activeElement, positions };
    };
    const sameLive = (before, after) => before.html === after.html && before.values === after.values &&
      before.active === after.active && before.positions.length === after.positions.length &&
      before.positions.every((value, index) => value === after.positions[index]);
    if (!markerAbsent()) throw new Error('PRESENTATION-MARKER');
    const original = live(), rows = [];
    for (const id of ['diagnostic', 'unavailable', 'error', 'identity']) {
      // A real Document without a browsing context. Never replace the live controller or page.
      const fixture = document.cloneNode(true);
      if (fixture.defaultView !== null) throw new Error('PRESENTATION-DOCUMENT');
      const before = counts(fixture);
      const controller = {
        revision: 0, busy: false, cancel() {}, edit() { this.revision++; },
        async compile() {
          if (id === 'error') throw new Error(text);
          const diagnostics = id === 'diagnostic' ? [{ severity: 'error', code: 'PRESENTATION-FIXTURE',
            message: text, guidance: text, path: text, byte_start: 0, byte_end: 0,
            line_start: 1, line_end: 1, column_start: 1, column_end: 1 }] : [];
          return { revision: this.revision, status: id === 'diagnostic' ? 'rejected' :
            id === 'unavailable' ? 'unavailable' : 'compiled', report: { diagnostics },
            failure: { code: 'PLAYGROUND-UNAVAILABLE', message: text }, sourceSha256: text,
            identity: { componentSha256: text }, exports: [{ logical: 'presentation_fixture', arity: 0 }] };
        },
      };
      const view = mountPlayground(fixture, controller), source = fixture.getElementById('source');
      source.value = text;source.dispatchEvent(new Event('input', { bubbles: true }));
      await view.compile();
      rows.push({ id, revision: controller.revision, source: source.value,
        sourceInvalid: source.getAttribute('aria-invalid'), status: fixture.getElementById('status').textContent,
        identity: fixture.getElementById('identity').textContent,
        diagnostics: [...fixture.getElementById('diagnostics').children].flatMap(item =>
          [...item.children].map(child => ({ tag: child.tagName, text: child.textContent }))),
        before, after: counts(fixture), detachedDocument: fixture.defaultView === null, markerAbsent: markerAbsent() });
    }
    return { format: 'zryna.ui-inert-presentation-negative.v1', version: 1,
      fixture: 'SYNTHETIC_PRESENTATION_NEGATIVE', actualCompilerResult: false,
      renderer: 'ORIGINAL_MOUNT_PLAYGROUND', rows, liveDocumentUnchanged: sameLive(original, live()) && markerAbsent() };
  });
  return validatePresentationNegative(observed);
}
