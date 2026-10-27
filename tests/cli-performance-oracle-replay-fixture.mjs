// Synthetic records around existing manifest examples/M2 artifact preimages; never execute.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { posix, win32 } from 'node:path';
import { gunzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { canonical, readSafe, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { metadata } from './cli-performance-windows-fixture.mjs';
export const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
const examples = JSON.parse(readFileSync(new URL('./cli-performance-oracle-manifest-examples.json', import.meta.url)));
const encoded = readSafe(fileURLToPath(new URL('.', import.meta.url)), 'cli-performance-oracle-m2-artifacts.json.gz.base64', 5781);
const container = Buffer.from(encoded.toString('ascii'), 'base64');
assert(Buffer.from(container.toString('base64') + '\n').equals(encoded), 'canonical bounded base64 test data');
assert.equal(container.length, 4334);
assert.equal(sha256(container), '46e485bbfcdead0fbab9e0e37862879eb7d473772882a27e9d4b93f925f4fe8a');
const expanded = gunzipSync(container, { maxOutputLength: 65536 });
assert.equal(expanded.length, 22923);
const payloads = JSON.parse(expanded).artifacts, goldens = new Map();
assert.deepEqual(Object.keys(payloads).sort(), ['javascript', 'native', 'webassembly']);
for (const pin of document.families[1].existingBuildGoldens) {
  const bytes = Buffer.from(payloads[pin.target], 'base64');
  assert.equal(bytes.toString('base64'), payloads[pin.target]);
  assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256);
  goldens.set(pin.target, bytes); // Checks use pre-existing methodology pins, never learned goldens.
}
export function oracleReplayFixture(windows = false) {
  const host = windows ? 'windows-x86_64' : 'linux-x86_64', p = windows ? win32 : posix;
  const root = windows ? 'C:\\source' : '/source', output = windows ? 'D:\\capture' : '/capture';
  const tool = name => ({ path: p.join(output, name), bytes: 128, sha256: '1'.repeat(64), mode: windows ? '666' : '755', prefixHex: '00'.repeat(128) });
  const tools = { cli: tool(`retained-release-cli${p.sep}${windows ? 'zryna.exe' : 'zryna'}`), node: tool(windows ? 'node.exe' : 'node'),
    ...(windows ? { helper: tool('helper.exe') } : {}) };
  const projected = Object.fromEntries(Object.entries(tools).map(([key, pin]) => [key, { path: pin.path, bytes: pin.bytes, sha256: pin.sha256 }]));
  const operator = { root, round: 'baseline-round-1', expectedHead: 'a'.repeat(40), expectedTree: 'b'.repeat(40) };
  const capture = { sourceCommit: operator.expectedHead, sourceTree: operator.expectedTree, round: operator.round, host: { platform: host },
    tools: projected, sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  const rows = matrixFor(document, host), files = new Map(), oracles = [];
  const put = (name, value) => files.set(name, Buffer.isBuffer(value) ? Buffer.from(value) : Buffer.from(canonical(value)));
  for (const [index, row] of rows.entries()) {
    const example = examples.examples.find(example => example.selector === row.id), m = JSON.parse(example.manifest);
    const artifact = row.id.startsWith('m2-modules/build/') ? goldens.get(row.target) : Buffer.from(`unexecuted artifact fixture ${row.id}\n`);
    const digest = sha256(artifact); Object.assign(m.artifacts[0], { bytes: artifact.length, sha256: digest });
    if (m.native) {
      m.native.ordinaryObjectSha256 = digest;
      if (m.native.execution) m.native.execution.ordinaryObjectSha256 = digest;
    }
    if (row.phase === 'build' && m.artifacts[0].metadata?.program_object_sha256) m.artifacts[0].metadata.program_object_sha256 = digest;
    const manifest = Buffer.from(`${JSON.stringify(m, null, 2)}\n`);
    const inventory = example.files.map(pin => {
      const bytes = pin.path === row.expectedResponse.manifest ? manifest : artifact;
      return { path: pin.path, bytes: bytes.length, sha256: sha256(bytes) };
    });
    const stdout = Buffer.from(JSON.stringify(row.expectedResponse) + '\n'), stderr = Buffer.alloc(0);
    for (const replay of [0, 1]) {
      const id = `${String(index).padStart(2, '0')}-${replay}`, prefix = `oracles/${id}`;
      const call = { file: tools.cli.path, args: row.argv.map(v => v === '$SOURCE_ROOT' ? root : v === '$NODE' ? tools.node.path : v), timeoutMs: 60000 };
      const outerCall = windows ? { file: tools.helper.path, args: ['--timeout-ms', '60000', '--output',
        p.join(output, 'oracles', `${id}.sample`), '--', call.file, ...call.args], timeoutMs: 71000 } : null;
      put(`${prefix}.process.json`, { call, outerCall, exitCode: 0, signal: null, error: null, elapsedMs: 1,
        stdoutBytes: stdout.length, stderrBytes: 0, stdoutTruncated: false, stderrTruncated: false,
        helperStdoutBytes: windows ? 0 : null, helperStderrBytes: windows ? 0 : null, readError: null });
      put(`${prefix}.stdout`, stdout); put(`${prefix}.stderr`, stderr);
      put(`${prefix}.after.json`, { status: 'passed', error: null }); put(`${prefix}.inventory.json`, inventory);
      for (const pin of inventory) put(`oracles/artifacts/${id}/${pin.path}`, pin.path === row.expectedResponse.manifest ? manifest : artifact);
      if (windows) {
        const h = metadata(stdout, stderr, call.args.length + 1); h.timeoutMs = 60000;
        const raw = Buffer.from(JSON.stringify(h) + '\n'); put(`${prefix}.metadata.raw`, raw); put(`${prefix}.sample/result.json`, raw);
        put(`${prefix}.sample/stdout.raw`, stdout); put(`${prefix}.sample/stderr.raw`, stderr);
        put(`${prefix}.helper.stdout`, Buffer.alloc(0)); put(`${prefix}.helper.stderr`, Buffer.alloc(0));
      }
    }
    oracles.push({ selector: row.id, manifest: row.expectedResponse.manifest, stdout: stdout.toString(), stderr: '', files: inventory });
  }
  put('oracles/catalogue.json', document);
  put('oracles/input.json', { host, tools, cells: rows.length, commands: rows.length * 2,
    executionGuardMs: 60000, combinedOutputBytes: 16777216, artifactTreeBytes: 67108864 });
  const architecture = { executable: tools.cli.path, args: ['architecture', 'check'], cwd: root, timeoutMs: 60000 };
  const architectureStdout = Buffer.from('synthetic architecture success\n');
  put('architecture.command.json', architecture);
  put('architecture.stdout.raw', architectureStdout); put('architecture.stderr.raw', Buffer.alloc(0));
  put('architecture.result.json', { command: architecture, exitCode: 0, signal: null, error: null, elapsedMs: 1,
    logs: { stdout: { bytes: architectureStdout.length, sha256: sha256(architectureStdout) }, stderr: { bytes: 0, sha256: sha256(Buffer.alloc(0)) } } });
  function seal() {
    const raw = Buffer.from(canonical(oracles)); capture.artifactOracleSha256 = sha256(raw);
    put('operator-input.json', operator); put('round/capture.json', capture);
    put('oracles/artifact-oracles.json', raw); put('round/artifact-oracles.json', raw);
    put('oracles/result.json', { status: 'unmeasured-correctness-and-paired-replay-passed', host, cells: rows.length, commands: rows.length * 2,
      tools, sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null,
      closureStatus: 'blocked', oracleFile: { path: p.join(output, 'oracles/artifact-oracles.json'), bytes: raw.length, sha256: sha256(raw) } });
  }
  seal();
  return { document, host, operator, capture, files, put, oracles, seal,
    read: (name, maximum) => { assert(files.has(name), name); const bytes = files.get(name); assert(bytes.length <= maximum); return bytes; },
    readOriginal: (name, maximum) => { const bytes = readFileSync(new URL(`../${name}`, import.meta.url)); assert(bytes.length <= maximum); return bytes; } };
}
