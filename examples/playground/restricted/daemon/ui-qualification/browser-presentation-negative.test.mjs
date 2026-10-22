import test from 'node:test';
import assert from 'node:assert/strict';
import { validatePresentationNegative } from './browser-presentation-negative.mjs';

const text = '<img src=x onerror="globalThis.__zrynaPresentationProbe=1"><script>globalThis.__zrynaPresentationProbe=1</script>';
function fixture() {
  const counts = () => ({ script: 1, img: 0, iframe: 0, svg: 0, object: 0, embed: 0, eventAttributes: 0 });
  const rows = ['diagnostic', 'unavailable', 'error', 'identity'].map(id => ({
    id, revision: 2, source: text, sourceInvalid: 'false',
    status: id === 'diagnostic' ? 'Compilation rejected. Review the compiler diagnostics.' :
      id === 'unavailable' ? `Compiler unavailable: PLAYGROUND-UNAVAILABLE — ${text}` :
      id === 'error' ? `Compilation failed: ${text}` : 'Compiled. Choose an export and run it.',
    identity: id === 'identity' ? `Source revision 2; source SHA-256 ${text}; component SHA-256 ${text}` : '',
    diagnostics: id === 'diagnostic' ? [
      { tag: 'P', text: `error: PRESENTATION-FIXTURE — ${text}` },
      { tag: 'BUTTON', text: `${text}:1:1–1:1 (UTF-8 bytes 0–0)` },
      { tag: 'P', text },
    ] : [],
    before: counts(), after: counts(), detachedDocument: true, markerAbsent: true,
  }));
  return { format: 'zryna.ui-inert-presentation-negative.v1', version: 1,
    fixture: 'SYNTHETIC_PRESENTATION_NEGATIVE', actualCompilerResult: false,
    renderer: 'ORIGINAL_MOUNT_PLAYGROUND', rows, liveDocumentUnchanged: true };
}
const rejects = change => { const value = fixture();change(value);assert.throws(() => validatePresentationNegative(value)); };

test('closed four-branch projection remains explicitly synthetic and preserves script baseline', () => {
  const value = fixture(), copy = validatePresentationNegative(value);
  assert.deepEqual(copy, value);copy.rows[0].before.script = 9;assert.equal(value.rows[0].before.script, 1);
});
test('synthetic data cannot claim compiler provenance or another renderer', () => {
  rejects(v => { v.actualCompilerResult = true; });
  rejects(v => { v.fixture = 'COMPILER_REPORT'; });
  rejects(v => { v.renderer = 'REPLACEMENT_RENDERER'; });
});
test('missing, repeated or reordered rendering branches cannot pass', () => {
  rejects(v => { v.rows.pop(); });
  rejects(v => { v.rows[1] = structuredClone(v.rows[0]); });
  rejects(v => { v.rows.reverse(); });
});
test('parsed markup and event attributes fail even when a global handler never ran', () => {
  for (const key of ['script', 'img', 'iframe', 'svg', 'object', 'embed', 'eventAttributes']) {
    rejects(v => { v.rows[0].after[key]++; });
  }
  rejects(v => { v.rows[0].before.img = v.rows[0].after.img = 1; });
});
test('exact diagnostic message, guidance and source-span path text are required', () => {
  for (const index of [0, 1, 2]) rejects(v => { v.rows[0].diagnostics[index].text = 'safe-looking replacement'; });
  rejects(v => { v.rows[0].diagnostics[0].tag = 'IMG'; });
  rejects(v => { v.rows[0].diagnostics.pop(); });
});
test('availability, caught error and component identity text cannot be omitted or substituted', () => {
  for (const index of [1, 2, 3]) rejects(v => { v.rows[index].status = ''; });
  rejects(v => { v.rows[3].identity = 'already validated hash'; });
});
test('source, revision, detached-document and marker observations are mandatory', () => {
  rejects(v => { v.rows[0].source = ''; });
  rejects(v => { v.rows[0].revision = 1; });
  rejects(v => { v.rows[0].sourceInvalid = 'true'; });
  rejects(v => { v.rows[0].detachedDocument = false; });
  rejects(v => { v.rows[0].markerAbsent = false; });
  rejects(v => { v.liveDocumentUnchanged = false; });
});
test('closed projections reject extra fields and invalid counts', () => {
  rejects(v => { v.compiled = true; });
  rejects(v => { v.rows[0].arbitrary = 'untrusted'; });
  rejects(v => { v.rows[0].before.url = 'http://unexpected'; });
  rejects(v => { v.rows[0].after.script = Infinity; });
  rejects(v => { v.rows[0].before.script = v.rows[0].after.script = 0; });
  rejects(v => { v.rows[0].before.script = v.rows[0].after.script = 2; });
  rejects(v => { v.rows[0].before.script = v.rows[0].after.script = 257; });
});
