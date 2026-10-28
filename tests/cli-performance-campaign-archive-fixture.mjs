// Actual tiny private Git data plus synthetic retained records; no workload, compiler or network.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, posix, win32 } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical, readSafe, sha256 } from '../scripts/stability-gates/input.mjs';
import { captureSourceCensus } from '../scripts/stability-gates/source-census-v2.mjs';
import { NUMERIC_WORKFLOWS } from '../scripts/stability-gates/cli-performance-campaign-plan-v1.mjs';
import { linkNumericCampaignPhase } from '../scripts/stability-gates/cli-performance-campaign-link-v1.mjs';
import { bindNumericCampaignSource } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { campaignFixture, PREFIX } from './cli-performance-campaign-fixture.mjs';
import { releaseImageFixture } from './release-image-replay-fixture.mjs';
import { oracleReplayFixture } from './cli-performance-oracle-replay-fixture.mjs';
import { linuxReplayFixture } from './cli-performance-linux-replay-fixture.mjs';
import { windowsReplayFixture } from './cli-performance-windows-replay-fixture.mjs';
import { exactWindows } from './cli-performance-linked-metrics-fixture.mjs';
import { archiveFixture } from './cli-performance-archive-fixture.mjs';
const CAPTURE = 'm418-hosted-observations/capture/';
const projected = pin => ({ path: pin.path, bytes: pin.bytes, sha256: pin.sha256 });
function remap(value, changes) {
  if (typeof value === 'string') {
    for (const [before, after] of changes) {
      if (value === before || value.startsWith(before + '/') || value.startsWith(before + '\\')) return after + value.slice(before.length);
    }
    return value;
  }
  if (Array.isArray(value)) return value.map(child => remap(child, changes));
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, remap(child, changes)]));
  return value;
}
function records(files, changes) {
  return new Map([...files].map(([name, bytes]) => {
    if (!name.endsWith('.json') || name.startsWith('oracles/artifacts/')) return [name, Buffer.from(bytes)];
    const old = JSON.parse(bytes), value = remap(old, changes);
    return [name, JSON.stringify(old) === JSON.stringify(value) ? Buffer.from(bytes) : Buffer.from(canonical(value))];
  }));
}
function sourceTree(t, origin, document) {
  const base = mkdtempSync(join(tmpdir(), 'numeric-composite-')), root = join(base, 'source');
  t.after(() => rmSync(base, { recursive: true, force: true })); mkdirSync(root);
  const git = args => execFileSync('git', ['-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgSign=false', ...args],
    { cwd: root, encoding: 'utf8', timeout: 30000, maxBuffer: 16 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
  git(['-c', 'init.templateDir=', 'init', '-q']); git(['config', 'core.autocrlf', 'false']);
  const put = (path, bytes) => { mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), bytes); };
  for (const path of NUMERIC_WORKFLOWS) put(path, Buffer.from(origin.responses.get(`${PREFIX}/contents/${path}?ref=${origin.plan.source.commit}`).content, 'base64'));
  const pins = new Map(document.materials.map(pin => [pin.path, pin]));
  for (const family of document.families) for (const pin of [...family.sources, family.oracle.material]) pins.set(pin.path, pin);
  const repository = fileURLToPath(new URL('../', import.meta.url));
  for (const pin of pins.values()) {
    const bytes = readSafe(repository, pin.path, pin.bytes);
    assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256); put(pin.path, bytes);
  }
  const affinity = 'scripts/stability-gates/windows-parent-affinity-v1/';
  for (const path of ['.github/workflows/windows-job-metrics.yml', ...['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'].map(name => affinity + name)]) {
    put(path, readSafe(repository, path));
  }
  git(['add', '--force', '--', '.']);
  git(['-c', 'user.name=Fixture', '-c', 'user.email=fixture@invalid', 'commit', '-qm', 'Synthetic archive source fixture']);
  const source = { commit: git(['rev-parse', 'HEAD']).trim(), tree: git(['rev-parse', 'HEAD^{tree}']).trim(), ref: origin.plan.source.ref };
  const captures = {}, files = new Map();
  for (const [key, path] of [['before', 'release-build/source-before'], ['after', 'release-build/source-after'], ['afterRound', 'source-after-round']]) {
    const directory = join(base, key);
    captures[key] = captureSourceCensus({ root, directory, executable: 'git', env: process.env });
    for (const name of readdirSync(directory)) files.set(`${path}/${name}`, readFileSync(join(directory, name)));
  }
  return { root, source, captures, files };
}
export async function campaignArchiveFixture(t, { windows = false, phase = 'baselines', hostEvidence = false } = {}) {
  assert(['baselines', 'heldout'].includes(phase));
  const release = releaseImageFixture(windows), oracle = oracleReplayFixture(windows);
  assert(typeof hostEvidence === 'boolean' && (!hostEvidence || windows));
  if (hostEvidence) {
    const sdk = { INCLUDE: 'C:\\sdk\\include', LIB: 'C:\\sdk\\lib', LIBPATH: 'C:\\sdk\\lib' };
    Object.assign(release.operator.environment, sdk); Object.assign(release.build.environment, sdk);
    release.operator.calibrationDirectory = 'C:\\evidence\\windows-job-metrics-calibration';
  }
  const raw = windows ? await windowsReplayFixture() : linuxReplayFixture();
  if (windows) exactWindows(raw);
  const origin = campaignFixture(phase), tree = sourceTree(t, origin, raw.document);
  const role = phase === 'heldout' ? 'held-out-candidate' : 'baseline-round-1', host = release.host;
  const p = windows ? win32 : posix, output = p.dirname(p.dirname(release.retained.copy.path));
  const helper = { path: p.join(output, windows ? 'helper.exe' : 'helper'), bytes: 128, sha256: '1'.repeat(64),
    mode: windows ? '666' : '755', prefixHex: '00'.repeat(128) };
  const changes = [[raw.capture.tools.cli.path, release.retained.copy.path], [raw.capture.tools.node.path, release.operator.tools.node],
    [raw.operator.root, release.operator.root], [oracle.capture.tools.cli.path, release.retained.copy.path],
    [oracle.capture.tools.node.path, release.operator.tools.node], [p.dirname(p.dirname(oracle.capture.tools.cli.path)), output]];
  if (windows) changes.push([raw.capture.tools.helper.path, helper.path],
    [p.join(p.dirname(raw.capture.operatorAdmission.path), 'round'), p.join(output, 'round')],
    [raw.capture.operatorAdmission.path, p.join(output, 'windows-operator-binding.json')], [oracle.capture.tools.helper.path, helper.path]);
  const replacements = changes.filter(([a, z]) => a !== z).sort((a, z) => z[0].length - a[0].length);
  const files = new Map([...records(oracle.files, replacements), ...records(raw.files, replacements), ...tree.files]);
  const put = (name, value) => files.set(name, Buffer.isBuffer(value) ? Buffer.from(value) : Buffer.from(canonical(value)));
  const capture = Object.assign(release.capture, remap(raw.capture, replacements));
  Object.assign(capture, { sourceCommit: tree.source.commit, sourceTree: tree.source.tree, round: role,
    tools: { ...capture.tools, cli: projected(release.retained.copy), node: projected(release.build.tools.after.node),
      ...(windows ? { helper: projected(helper) } : {}) }, environment: remap(release.capture.environment, replacements) });
  // Restore the fixed collector environment from the release fixture, not the minimal raw fixture.
  const keys = windows ? ['PATH', 'SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'CARGO_HOME', 'RUSTUP_HOME', 'LIB', 'INCLUDE']
    : ['PATH', 'HOME', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME'];
  capture.environment = { ...Object.fromEntries(keys.map(key => [key, release.operator.environment[key]])),
    RUSTUP_TOOLCHAIN: '1.97.1', RUSTC: release.operator.tools.rustc };
  Object.assign(release.expectedSource, { commit: tree.source.commit, tree: tree.source.tree });
  Object.assign(release.operator, { expectedHead: tree.source.commit, expectedTree: tree.source.tree, round: role });
  release.operator.environment.GITHUB_RUN_ID = String(origin.plan.runId); release.build.environment.GITHUB_RUN_ID = String(origin.plan.runId);
  release.build.source = { before: tree.captures.before, after: tree.captures.after };
  const fullTools = { cli: release.retained.copy, node: release.build.tools.after.node, ...(windows ? { helper } : {}) };
  const input = remap(JSON.parse(oracle.files.get('oracles/input.json')), replacements); input.tools = fullTools; put('oracles/input.json', input);
  const result = remap(JSON.parse(oracle.files.get('oracles/result.json')), replacements); result.tools = fullTools; put('oracles/result.json', result);
  const oracleBytes = Buffer.from(canonical(oracle.oracles)); capture.artifactOracleSha256 = sha256(oracleBytes);
  put('round/artifact-oracles.json', oracleBytes); put('oracles/artifact-oracles.json', oracleBytes);
  const session = remap(raw.session, replacements); session.round = role;
  if (windows) {
    const admission = remap(JSON.parse(raw.files.get('round/operator-admission.json')), replacements);
    Object.assign(admission, { sourceCommit: tree.source.commit, sourceTree: tree.source.tree, tools: capture.tools,
      artifactOracleSha256: capture.artifactOracleSha256 });
    const bytes = Buffer.from(canonical(admission)); put('round/operator-admission.json', bytes);
    capture.operatorAdmission = { path: p.join(output, 'windows-operator-binding.json'), bytes: bytes.length, sha256: sha256(bytes) };
    session.operatorAdmission = structuredClone(capture.operatorAdmission);
  }
  for (const [index, cell] of session.cells.entries()) for (const attempt of cell.attempts) {
    const stdout = Buffer.from(oracle.oracles[index].stdout); put(`round/${attempt.id}.stdout`, stdout);
    if (!windows) {
      const result = JSON.parse(files.get(`round/${attempt.id}.json`)); result.stdoutBytes = stdout.length;
      put(`round/${attempt.id}.json`, result);
    } else {
      put(`round/${attempt.id}.sample/stdout.raw`, stdout);
      const h = JSON.parse(files.get(`round/${attempt.id}.metadata.raw`)); h.stdoutBytes = stdout.length;
      const bytes = Buffer.from(JSON.stringify(h) + '\n'); put(`round/${attempt.id}.metadata.raw`, bytes); put(`round/${attempt.id}.sample/result.json`, bytes);
      const read = JSON.parse(files.get(`round/${attempt.id}.sample-read.json`)); read.stdoutBytes = stdout.length; read.metadataBytes = bytes.length;
      put(`round/${attempt.id}.sample-read.json`, read);
    }
  }
  release.sync();
  for (const [name, bytes] of release.files) files.set(name, Buffer.from(bytes));
  // The original oracle replay requires canonical equality of these shared documents.
  put('operator-input.json', release.operator); put('round/capture.json', capture); put('round/catalogue.json', raw.document);
  session.captureSha256 = sha256(canonical(capture)); put('round/session.json', session);
  put('source-after-round.json', tree.captures.afterRound);
  const allFiles = new Map([...files].map(([path, bytes]) => [CAPTURE + path, bytes]));
  const fixture = { root: tree.root, source: tree.source, files: allFiles, release, capture, session, host, role };
  fixture.reseal = async () => {
    const bytes = archiveFixture([...allFiles]), f = campaignFixture(phase, tree.source);
    const slot = f.jobs.find(job => job.name === `${host} / ${role} / collect`); assert(slot);
    const name = `cli-performance-numeric-${host}-${role}-${f.plan.runId}-1`;
    const artifact = f.artifacts.find(value => value.name === name); assert(artifact);
    artifact.size_in_bytes = bytes.length; artifact.digest = `sha256:${sha256(bytes)}`;
    fixture.selection = await linkNumericCampaignPhase(...f.arguments());
    fixture.sourceKey = bindNumericCampaignSource(fixture.selection, tree.root);
    fixture.artifactId = artifact.id; fixture.bytes = bytes; fixture.platform = f;
    return fixture;
  };
  return fixture.reseal();
}
