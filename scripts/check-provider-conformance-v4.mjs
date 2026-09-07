import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  lstatSync, readFileSync, readdirSync,
} from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { spawnSync } from 'node:child_process';
import { TextDecoder } from 'node:util';
import { fileURLToPath } from 'node:url';

import Ajv2020 from 'ajv/dist/2020.js';

const scriptPath = fileURLToPath(import.meta.url);
export const workspaceRoot = resolve(dirname(scriptPath), '..');
export const registryPath = resolve(workspaceRoot, 'tests/provider-conformance-v4.json');
const fixtureRoot = 'tests/provider-conformance-v4/fixtures';
const expectedRegistryDigest = 'ba2140e5ec068da57bae8c54ede454fda56125cf11b0fb985e6585268e2448a3';
const expectedCaseOrder = [
  'positive', 'malformed', 'unsupported', 'budget', 'recovery', 'ordering',
];
const requiredCapabilities = Object.freeze({
  module_resolution: false,
  semantic_diagnostics: false,
  control_flow_v1: true,
  data_ownership_syntax_v1: true,
});

export function digest(value) {
  return createHash('sha256').update(value).digest('hex');
}

function exactKeys(value, keys, label) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), `${label} must be an object`);
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), `${label} fields`);
}

function assertSafeFile(root, path) {
  let current = root;
  for (const segment of path.split('/')) {
    current = resolve(current, segment);
    assert(!lstatSync(current).isSymbolicLink(), `fixture link forbidden: ${path}`);
  }
  assert(lstatSync(current).isFile(), `fixture must be a file: ${path}`);
  return current;
}

function listFiles(root, directory) {
  const pending = [resolve(root, directory)];
  const files = [];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const path = resolve(current, entry.name);
      assert(!entry.isSymbolicLink(), `fixture link forbidden: ${path}`);
      if (entry.isDirectory()) pending.push(path);
      else {
        assert(entry.isFile(), `fixture must be a regular file: ${path}`);
        files.push(relative(root, path).split(sep).join('/'));
      }
    }
  }
  return files.sort();
}

function validateArtifact(artifact, ids, paths, root) {
  exactKeys(artifact, ['id', 'kind', 'path', 'sha256'], `artifact ${artifact?.id ?? '<unknown>'}`);
  assert.match(artifact.id, /^[a-z][a-z0-9-]*$/, 'portable artifact id');
  assert(['request', 'snapshot', 'source'].includes(artifact.kind), `artifact kind: ${artifact.id}`);
  assert.match(
    artifact.path,
    /^tests\/provider-conformance-v4\/fixtures\/[A-Za-z0-9.-]+\.(?:json|ndjson|zry)$/,
    `portable artifact path: ${artifact.id}`,
  );
  assert.match(artifact.sha256, /^[a-f0-9]{64}$/, `artifact digest: ${artifact.id}`);
  assert(!ids.has(artifact.id), `duplicate artifact id: ${artifact.id}`);
  ids.add(artifact.id);
  const portablePath = artifact.path.toLowerCase();
  assert(!paths.has(portablePath), `case-colliding artifact path: ${artifact.path}`);
  paths.add(portablePath);
  assert.equal(
    artifact.kind,
    artifact.path.endsWith('.zry') ? 'source' : artifact.path.endsWith('.ndjson') ? 'request' : 'snapshot',
    `artifact extension: ${artifact.id}`,
  );
  const file = assertSafeFile(root, artifact.path);
  const bytes = readFileSync(file);
  assert.equal(digest(bytes), artifact.sha256, `tampered artifact: ${artifact.id}`);
  if (artifact.kind === 'snapshot') {
    assert.equal(bytes.toString('utf8'), `${JSON.stringify(JSON.parse(bytes))}\n`, `canonical snapshot bytes: ${artifact.id}`);
  } else if (artifact.kind === 'request') {
    const text = bytes.toString('utf8');
    assert.equal(text, `${text.trimEnd()}\n`, `canonical request line: ${artifact.id}`);
    assert(!text.trimEnd().includes('\n'), `request artifact contains multiple lines: ${artifact.id}`);
  }
}

function validateSourceRequest(request, artifacts, referenced) {
  exactKeys(request, ['id', 'kind', 'sources'], 'analyze request');
  assert.equal(request.kind, 'analyze');
  assert(Number.isInteger(request.id) && request.id >= 0 && request.id <= 0xffff_ffff, 'analyze request id');
  assert(Array.isArray(request.sources) && request.sources.length > 0, 'analyze sources');
  const paths = new Set();
  for (const source of request.sources) {
    exactKeys(source, ['artifact', 'path'], 'analyze source');
    assert.match(source.path, /^[\x20-\x7e]+\.zry$/, 'portable source path');
    assert(!source.path.startsWith('/') && !source.path.includes('\\') && !source.path.includes('..'), 'relative source path');
    assert(!paths.has(source.path.toLowerCase()), `case-colliding source path: ${source.path}`);
    paths.add(source.path.toLowerCase());
    assert.equal(artifacts.get(source.artifact)?.kind, 'source', `source artifact: ${source.artifact}`);
    referenced.add(source.artifact);
  }
}

function validateStep(step, artifacts, referenced) {
  exactKeys(step, ['expected', 'request'], 'case step');
  exactKeys(step.request, step.request.kind === 'raw' ? ['artifact', 'kind'] :
    step.request.kind === 'handshake' ? ['id', 'kind'] : ['id', 'kind', 'sources'], 'step request');
  if (step.request.kind === 'raw') {
    assert.equal(artifacts.get(step.request.artifact)?.kind, 'request', `raw request: ${step.request.artifact}`);
    referenced.add(step.request.artifact);
  } else if (step.request.kind === 'handshake') {
    assert(Number.isInteger(step.request.id) && step.request.id >= 0 && step.request.id <= 0xffff_ffff, 'handshake request id');
  } else {
    validateSourceRequest(step.request, artifacts, referenced);
  }
  const expectedKeys = step.expected.kind === 'snapshot' ? ['artifact', 'id', 'kind'] :
    step.expected.kind === 'diagnostic' ? ['code', 'id', 'kind'] : ['id', 'kind'];
  exactKeys(step.expected, expectedKeys, 'step expectation');
  assert(['diagnostic', 'handshake', 'snapshot'].includes(step.expected.kind), 'expectation kind');
  assert(
    step.expected.id === null ||
      (Number.isInteger(step.expected.id) && step.expected.id >= 0 && step.expected.id <= 0xffff_ffff),
    'expectation id',
  );
  if (step.request.kind === 'raw') assert.equal(step.expected.id, null, 'raw request diagnostic id');
  else assert.equal(step.expected.id, step.request.id, 'correlated response id');
  if (step.expected.kind === 'snapshot') {
    assert.equal(artifacts.get(step.expected.artifact)?.kind, 'snapshot', `snapshot artifact: ${step.expected.artifact}`);
    referenced.add(step.expected.artifact);
  } else if (step.expected.kind === 'diagnostic') {
    assert.match(step.expected.code, /^ZRYNA-F[0-9]{4}$/, 'stable provider diagnostic');
  }
}

export function validateProviderConformanceRegistry(
  bytes,
  root = workspaceRoot,
  expectedDigest = expectedRegistryDigest,
) {
  assert(Buffer.byteLength(bytes) <= 64 * 1024, 'bounded provider-conformance registry');
  assert.equal(digest(bytes), expectedDigest, 'provider-conformance registry differs from its frozen oracle');
  const registry = JSON.parse(bytes);
  exactKeys(registry, [
    'artifacts', 'bootstrapAuthority', 'cases', 'profile', 'protocolVersion',
    'requiredCapabilities', 'schemaVersion',
  ], 'provider-conformance registry');
  assert.equal(registry.schemaVersion, 1);
  assert.equal(registry.protocolVersion, 4);
  assert.equal(registry.profile, 'zryna-provider-conformance-v4');
  assert.deepEqual(registry.requiredCapabilities, requiredCapabilities);
  exactKeys(registry.bootstrapAuthority, ['arguments', 'executable', 'provider', 'providerVersion'], 'bootstrap authority');
  assert.deepEqual(registry.bootstrapAuthority, {
    provider: 'typescript-6', providerVersion: '6.0.3', executable: 'node',
    arguments: ['adapters/typescript-6/src/worker-v4.mjs'],
  });
  assert(Array.isArray(registry.artifacts) && registry.artifacts.length > 0, 'artifact inventory');
  assert.deepEqual(
    registry.artifacts.map((artifact) => artifact.id),
    registry.artifacts.map((artifact) => artifact.id).toSorted(),
    'artifact inventory must remain ordered',
  );
  const ids = new Set();
  const paths = new Set();
  for (const artifact of registry.artifacts) validateArtifact(artifact, ids, paths, root);
  assert.deepEqual(
    listFiles(root, fixtureRoot),
    registry.artifacts.map((artifact) => artifact.path).toSorted(),
    'unregistered or missing provider-conformance fixture',
  );
  assert(Array.isArray(registry.cases), 'case inventory');
  assert.deepEqual(registry.cases.map((entry) => entry.id), expectedCaseOrder, 'case order');
  const artifacts = new Map(registry.artifacts.map((artifact) => [artifact.id, artifact]));
  const referenced = new Set();
  for (const entry of registry.cases) {
    exactKeys(entry, ['category', 'id', 'steps'], `case ${entry?.id ?? '<unknown>'}`);
    assert.equal(entry.category, entry.id, `case category: ${entry.id}`);
    assert(Array.isArray(entry.steps) && entry.steps.length > 0, `case steps: ${entry.id}`);
    assert.equal(entry.steps[0]?.request?.kind, 'handshake', `case handshake: ${entry.id}`);
    for (const step of entry.steps) validateStep(step, artifacts, referenced);
  }
  assert.deepEqual([...referenced].toSorted(), [...ids].toSorted(), 'every fixture must be referenced');
  return registry;
}

export function loadAndValidateProviderConformance() {
  return validateProviderConformanceRegistry(readFileSync(registryPath));
}

function artifactBytes(registry, artifactId, root) {
  const artifact = registry.artifacts.find((entry) => entry.id === artifactId);
  assert(artifact, `unknown artifact: ${artifactId}`);
  return readFileSync(resolve(root, artifact.path));
}

function requestLine(request, registry, root) {
  if (request.kind === 'raw') return artifactBytes(registry, request.artifact, root).toString('utf8').trimEnd();
  if (request.kind === 'handshake') return JSON.stringify({ id: request.id, method: 'handshake' });
  const files = request.sources.map((source) => ({
    path: source.path,
    text: artifactBytes(registry, source.artifact, root).toString('utf8'),
  }));
  return JSON.stringify({
    id: request.id, method: 'analyze', params: { schema_version: 4, files },
  });
}

function sourceMap(request, registry, root) {
  if (request.kind !== 'analyze') return [];
  return request.sources.map((source) => ({
    path: source.path,
    bytes: artifactBytes(registry, source.artifact, root),
  })).toSorted((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
}

function visitSpans(value, sources) {
  if (value === null || typeof value !== 'object') return;
  if (!Array.isArray(value) && ['file', 'start', 'end'].every((key) => Object.hasOwn(value, key))) {
    exactKeys(value, ['end', 'file', 'start'], 'source span');
    assert(Number.isInteger(value.file) && value.file >= 0 && value.file < sources.length, 'source span file');
    const source = sources[value.file].bytes;
    assert(Number.isInteger(value.start) && Number.isInteger(value.end), 'source span offsets');
    assert(value.start >= 0 && value.start <= value.end && value.end <= source.length, 'source span bounds');
    new TextDecoder('utf-8', { fatal: true }).decode(source.subarray(value.start, value.end));
  }
  if (!Array.isArray(value) && typeof value.text === 'string' && value.span) {
    const span = value.span;
    const observed = sources[span.file].bytes.subarray(span.start, span.end).toString('utf8');
    assert.equal(observed, value.text, `source text binding: ${value.text}`);
  }
  if (!Array.isArray(value) && typeof value.text === 'string' && value.value_span) {
    const span = value.value_span;
    const observed = sources[span.file].bytes.subarray(span.start, span.end).toString('utf8');
    assert.equal(observed, value.text, `source value binding: ${value.text}`);
  }
  if (!Array.isArray(value) && value.span && value.kind &&
      ['i32-literal', 'string-literal'].includes(value.kind.kind)) {
    const span = value.span;
    const observed = sources[span.file].bytes.subarray(span.start, span.end).toString('utf8');
    assert.equal(observed, value.kind.spelling, `source spelling binding: ${value.kind.kind}`);
  }
  for (const child of Array.isArray(value) ? value : Object.values(value)) visitSpans(child, sources);
}

export function verifySourceBinding(snapshot, sources) {
  assert.deepEqual(
    snapshot.files.map((file) => ({ id: file.id, path: file.path })),
    sources.map((source, id) => ({ id, path: source.path })),
    'snapshot file identities must bind the canonical source order',
  );
  visitSpans(snapshot, sources);
}

function verifyHandshake(response, expected, registry, bootstrap) {
  exactKeys(response, ['id', 'result'], 'handshake response');
  assert.equal(response.id, expected.id);
  exactKeys(response.result, ['capabilities', 'protocol_version', 'provider', 'provider_version'], 'handshake result');
  assert.equal(response.result.protocol_version, 4);
  assert.deepEqual(response.result.capabilities, requiredCapabilities);
  assert.match(response.result.provider, /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/, 'provider identity');
  assert(typeof response.result.provider_version === 'string' && response.result.provider_version.length > 0 &&
    Buffer.byteLength(response.result.provider_version) <= 128, 'provider version');
  if (bootstrap) {
    assert.equal(response.result.provider, registry.bootstrapAuthority.provider);
    assert.equal(response.result.provider_version, registry.bootstrapAuthority.providerVersion);
  }
}

function verifyDiagnostic(response, expected) {
  exactKeys(response, ['error', 'id'], 'diagnostic response');
  assert.equal(response.id, expected.id);
  exactKeys(response.error, ['code', 'message'], 'diagnostic');
  assert.equal(response.error.code, expected.code);
  assert(typeof response.error.message === 'string' && response.error.message.length > 0, 'diagnostic message');
  assert(Buffer.byteLength(response.error.message) <= 4096, 'diagnostic message bound');
}

function verifyResponse(response, step, registry, root, validateSnapshot, bootstrap) {
  const expected = step.expected;
  if (expected.kind === 'handshake') return verifyHandshake(response, expected, registry, bootstrap);
  if (expected.kind === 'diagnostic') return verifyDiagnostic(response, expected);
  const snapshot = JSON.parse(artifactBytes(registry, expected.artifact, root));
  assert.equal(validateSnapshot(snapshot), true, JSON.stringify(validateSnapshot.errors));
  assert.deepEqual(response, { id: expected.id, result: snapshot }, `snapshot mismatch: ${expected.artifact}`);
  assert.equal(validateSnapshot(response.result), true, JSON.stringify(validateSnapshot.errors));
  verifySourceBinding(response.result, sourceMap(step.request, registry, root));
}

function executeCase(entry, registry, options) {
  const input = `${entry.steps.map((step) => requestLine(step.request, registry, options.root)).join('\n')}\n`;
  const result = options.spawn(options.command, options.args, {
    cwd: options.root,
    env: process.env,
    shell: false,
    windowsHide: true,
    input,
    encoding: 'utf8',
    timeout: 30_000,
    maxBuffer: 16 * 1024 * 1024,
  });
  assert.equal(result.error, undefined, `${entry.id} provider spawn`);
  assert.equal(result.signal, null, `${entry.id} provider signal`);
  assert.equal(result.status, 0, `${entry.id} provider status: ${result.stderr ?? ''}`);
  assert.equal(result.stderr, '', `${entry.id} provider stderr`);
  assert(result.stdout.endsWith('\n'), `${entry.id} response must end with newline`);
  const lines = result.stdout.trimEnd().split('\n');
  assert.equal(lines.length, entry.steps.length, `${entry.id} response count`);
  const responses = lines.map((line) => JSON.parse(line));
  for (const [index, response] of responses.entries()) {
    verifyResponse(response, entry.steps[index], registry, options.root, options.validateSnapshot, options.bootstrap);
  }
  return result.stdout;
}

export function assertDeterministic(first, second, label = 'provider session') {
  assert.equal(second, first, `${label} is nondeterministic`);
}

export function runProviderConformance({
  command,
  args,
  root = workspaceRoot,
  spawn = spawnSync,
  bootstrap,
} = {}) {
  const registry = validateProviderConformanceRegistry(readFileSync(resolve(root, 'tests/provider-conformance-v4.json')), root);
  const usingBootstrap = command === undefined;
  const selectedCommand = command ?? process.execPath;
  const selectedArgs = args ?? registry.bootstrapAuthority.arguments;
  const schema = JSON.parse(readFileSync(resolve(root, 'schemas/zryna-syntax-v4.schema.json')));
  const validateSnapshot = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const options = {
    root, spawn, command: selectedCommand, args: selectedArgs,
    bootstrap: bootstrap ?? usingBootstrap, validateSnapshot,
  };
  for (const entry of registry.cases) {
    const first = executeCase(entry, registry, options);
    const second = executeCase(entry, registry, options);
    assertDeterministic(first, second, entry.id);
  }
  return registry.cases.length;
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  try {
    const separator = process.argv.indexOf('--');
    assert(separator !== -1 || process.argv.length === 2, 'unknown arguments; provider command must follow --');
    const override = separator === -1 ? [] : process.argv.slice(separator + 1);
    assert(separator === -1 || override.length > 0, 'provider command must follow --');
    const count = override.length === 0 ? runProviderConformance() : runProviderConformance({
      command: override[0], args: override.slice(1), bootstrap: false,
    });
    console.log(`Provider conformance v4: ${count} canonical cases passed twice.`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
