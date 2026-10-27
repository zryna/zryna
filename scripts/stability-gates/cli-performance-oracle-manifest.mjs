// Fixed-fixture manifest projection from public source contracts; never a new codegen golden.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { TextDecoder } from 'node:util';
import { canonical, exact, MAX_DOCUMENT, portable, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
const hash = value => assert(typeof value === 'string' && /^[a-f0-9]{64}$/u.test(value), 'manifest digest');
function decode(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= MAX_DOCUMENT, 'bounded manifest');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes), value = JSON.parse(text);
  canonical(value);
  // Existing serializers emit canonical primitive spellings and ASCII portable paths; whitespace may vary.
  const compact = text.replace(/"(?:\\.|[^"\\])*"|\s+/gu, token => token.startsWith('"') ? token : '');
  assert.equal(compact, JSON.stringify(value), 'duplicate keys or noncanonical manifest token spellings');
  return value;
}
function originalGraph(family, readOriginal) {
  const original = new Map(family.sources.map(pin => {
    const bytes = readOriginal(pin.path, pin.bytes);
    assert(Buffer.isBuffer(bytes) && bytes.length === pin.bytes); assert.equal(sha256(bytes), pin.sha256);
    return [pin.path, { pin, text: bytes.toString('utf8') }];
  }));
  const modules = family.moduleGraph.map(node => ({ node, ...original.get(node.original) }))
    .sort((a, b) => a.node.destination < b.node.destination ? -1 : 1);
  const sources = modules.map(({ node, pin }, id) => ({ id, path: node.destination, sha256: pin.sha256 }));
  const edges = modules.flatMap(({ node, text }) => {
    const imports = [...text.matchAll(/^import\s+\{([^}]+)\}\s+from\s+"([^"\n]+)";$/gmu)];
    assert.deepEqual(imports.map(m => m[2]), node.imports.map(e => e.specifier), 'original named imports');
    return imports.flatMap((m, index) => m[1].split(',').map(part => {
      const binding = /^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?\s*$/u.exec(part);
      assert(binding, 'fixed original import binding');
      return { importer: node.destination, target: node.imports[index].resolvesTo, specifier: m[2],
        imported: binding[1], local: binding[2] ?? binding[1] };
    }));
  }).sort((a, b) => {
    for (const key of ['importer', 'specifier', 'imported', 'local']) if (a[key] !== b[key]) return a[key] < b[key] ? -1 : 1;
    return 0;
  });
  return { sources, edges, modules: sources.map(source => ({ ...source,
    bytes: original.get(modules[source.id].node.original).pin.bytes })) };
}
function graphDigest(domain, entry, sources, edges) {
  const chunks = [Buffer.from(domain)];
  const u32 = value => { const bytes = Buffer.alloc(4); bytes.writeUInt32LE(value); chunks.push(bytes); };
  const text = value => { const bytes = Buffer.from(value); u32(bytes.length); chunks.push(bytes); };
  u32(1); text(entry); u32(sources.length);
  for (const source of sources) { text(source.path); chunks.push(Buffer.from(source.sha256, 'hex')); }
  u32(edges.length);
  for (const edge of edges) for (const key of ['importer', 'specifier', 'imported', 'local']) text(edge[key]);
  return sha256(Buffer.concat(chunks));
}
function ownershipMetadata(artifact, row) {
  const metadata = artifact.metadata;
  if (row.target === 'javascript') assert.deepEqual(metadata, { profile: 'ecmascript-module', scalar_abi: 'v1' });
  else if (row.target === 'webassembly') assert.deepEqual(metadata,
    { profile: 'core-webassembly', memory: 'linear32-v1', imports: false, scalar_abi: 'v1' });
  else {
    exact(metadata, ['profile', 'triple', 'format', 'scalar_abi', 'program_object_sha256', 'runtime_object_sha256', 'harness_sha256'], 'native metadata');
    assert.equal(metadata.profile, 'linux-x86-64-elf'); assert.equal(metadata.triple, 'x86_64-unknown-linux-gnu');
    assert.equal(metadata.scalar_abi, 'v1'); hash(metadata.program_object_sha256);
    assert.equal(metadata.format, row.phase === 'build' ? 'elf-relocatable' : 'elf-executable');
    if (row.phase === 'build') {
      assert.equal(metadata.program_object_sha256, artifact.sha256);
      assert.equal(metadata.runtime_object_sha256, null); assert.equal(metadata.harness_sha256, null);
    } else { hash(metadata.runtime_object_sha256); hash(metadata.harness_sha256); }
  }
}
export function validateRetainedManifest({ document, row, host, manifestBytes, files, readOriginal }) {
  assert.deepEqual(row, matrixFor(document, host).find(candidate => candidate.id === row.id), 'frozen manifest selector');
  const family = document.families.find(f => row.id.startsWith(`${f.id}/`)), m = decode(manifestBytes);
  const generic = family.id === 'm7-mixed', ownership = family.id === 'm3-ownership';
  const common = ['command', 'artifacts', 'invocation'];
  exact(m, generic ? [...common, 'format', 'profile', 'syntaxProtocol', 'artifactStem', 'source', 'exports', 'native'] :
    [...common, 'version', 'profile', 'entrypoint', 'stem', 'targets', 'results', 'diagnostics',
      ...(family.id === 'm1-wrap' ? ['source_sha256'] : ['graph_sha256', 'sources', 'edges']),
      ...(ownership ? ['protocol_version', 'layouts', 'runtime_abi'] : [])], 'fixed manifest');
  assert.equal(m.command, row.phase); const stem = row.argv[row.argv.indexOf('--name') + 1];
  assert.equal(generic ? m.artifactStem : m.stem, stem);
  const graph = originalGraph(family, readOriginal);
  if (generic) {
    assert.equal(m.format, 'zryna.generic-bundle.v1'); assert.equal(m.profile, 'bounded-generics-v1'); assert.equal(m.syntaxProtocol, 5);
    exact(m.source, ['entrypoint', 'entryModule', 'modules', 'edges', 'graphSha256', 'executionSha256',
      'typeUniverseSha256', 'linear32Sha256', 'linuxX8664Sha256', 'runtimeAbiSha256'], 'generic source');
    assert.equal(m.source.entrypoint, family.entry); assert.equal(m.source.entryModule, graph.sources.find(s => s.path === family.entry).id);
    assert.deepEqual(m.source.modules, graph.modules); assert.deepEqual(m.source.edges, graph.edges);
    assert.equal(m.source.graphSha256, graphDigest('ZRYNA-BOUNDED-GENERICS-GRAPH-V1\0', family.entry, graph.sources, graph.edges));
    for (const key of ['executionSha256', 'typeUniverseSha256', 'linear32Sha256', 'linuxX8664Sha256', 'runtimeAbiSha256']) hash(m.source[key]);
    assert(Array.isArray(m.exports) && m.exports.length > 0 && m.exports.length <= 64);
    for (const e of m.exports) {
      exact(e, ['name', 'parameters', 'result'], 'generic export');
      assert(typeof e.name === 'string' && /^[A-Za-z_][A-Za-z0-9_]*$/u.test(e.name));
      assert(Array.isArray(e.parameters) && e.parameters.length <= 64 && e.parameters.every(t => ['bool', 'i32'].includes(t)));
      assert(['bool', 'i32'].includes(e.result));
    }
    assert.deepEqual(m.exports.map(e => e.name), [...new Set(m.exports.map(e => e.name))].sort());
    assert.deepEqual(m.exports.find(e => e.name === family.export),
      { name: family.export, parameters: family.arguments.map(a => a.type), result: family.expectedValue.type });
  } else {
    const profiles = { 'm1-wrap': [1, 'zryna-m1-cli-v1'], 'm2-modules': [2, 'zryna-control-flow-v1'], 'm3-ownership': [3, 'zryna-data-ownership-v1'] };
    assert.equal(m.version, profiles[family.id][0]); assert.equal(m.profile, profiles[family.id][1]);
    assert.equal(m.entrypoint, family.entry); assert.deepEqual(m.targets, [row.target]); assert.deepEqual(m.diagnostics, []);
    if (family.id === 'm1-wrap') assert.equal(m.source_sha256, graph.sources[0].sha256);
    else {
      assert.deepEqual(m.sources, graph.sources); assert.deepEqual(m.edges, graph.edges);
      assert.equal(m.graph_sha256, graphDigest(ownership ? 'ZRYNA-M3-GRAPH\0' : 'ZRYNA-M2-GRAPH\0', family.entry, graph.sources, graph.edges));
    }
    if (ownership) {
      assert.equal(m.protocol_version, 4);
      exact(m.layouts, ['type_universe_sha256', 'linear32_sha256', 'linux_x86_64_sha256'], 'ownership layouts');
      for (const value of Object.values(m.layouts)) hash(value);
      exact(m.runtime_abi, ['identifier', 'version', 'sha256'], 'ownership runtime ABI');
      assert.equal(m.runtime_abi.identifier, 'zryna-ownership-runtime-v1'); assert.equal(m.runtime_abi.version, 1); hash(m.runtime_abi.sha256);
    }
  }
  assert(Array.isArray(files) && files.length > 0 && files.length <= 64);
  let total = 0;
  for (const pin of files) {
    exact(pin, ['path', 'bytes', 'sha256'], 'retained file'); portable(pin.path); hash(pin.sha256);
    assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0); total += pin.bytes;
  }
  assert(total <= 64 * 1024 * 1024); const paths = files.map(pin => pin.path);
  assert.deepEqual(paths, [...new Set(paths)].sort());
  const manifest = files.find(pin => pin.path === row.expectedResponse.manifest); assert(manifest, 'retained manifest file');
  assert.equal(manifest.bytes, manifestBytes.length); assert.equal(manifest.sha256, sha256(manifestBytes));
  assert(Array.isArray(m.artifacts) && m.artifacts.length === 1); const artifact = m.artifacts[0];
  exact(artifact, ['target', 'kind', 'bytes', 'sha256', ...(ownership ? ['filename', 'metadata'] : ['path'])], 'selected artifact');
  assert.equal(artifact.target, row.target);
  const executable = row.target === 'native' && row.phase === 'run' && !generic;
  const extension = row.target === 'javascript' ? 'mjs' : row.target === 'webassembly' ? 'wasm' : executable ? 'elf' : 'o';
  const kind = row.target === 'javascript' ? 'ecmascript-module' : row.target === 'webassembly' ? 'core-webassembly-module' :
    executable ? 'linux-x86-64-invocation-executable' : 'linux-x86-64-relocatable-object';
  assert.equal(artifact.kind, kind); const relativePath = `${row.target}/${stem}.${extension}`;
  if (ownership) { assert.equal(artifact.filename, `${stem}.${extension}`); ownershipMetadata(artifact, row); }
  else assert.equal(artifact.path, relativePath);
  const artifactPath = `${posix.dirname(row.expectedResponse.manifest)}/${relativePath}`;
  const actual = files.find(pin => pin.path === artifactPath); assert(actual, 'declared artifact exists in copied tree');
  assert.equal(artifact.bytes, actual.bytes); assert.equal(artifact.sha256, actual.sha256);
  assert.deepEqual(paths, [row.expectedResponse.manifest, artifactPath].sort(), 'every retained artifact is declared');
  if (generic) {
    if (row.target !== 'native') assert.equal(m.native, null);
    else {
      exact(m.native, ['target', 'templateSha256', 'headerSha256', 'renderedSha256', 'ordinaryObjectSha256', 'requiredTlsBytes', 'execution'], 'generic native');
      assert.equal(m.native.target, 'x86_64-unknown-linux-gnu'); assert.equal(m.native.ordinaryObjectSha256, artifact.sha256);
      for (const key of ['templateSha256', 'headerSha256', 'renderedSha256']) hash(m.native[key]);
      assert([0, 33554448].includes(m.native.requiredTlsBytes));
      if (row.phase === 'build') assert.equal(m.native.execution, null);
      else {
        exact(m.native.execution, ['ordinaryObjectSha256', 'runtimeObjectSha256', 'invocationObjectSha256',
          'harnessSha256', 'executableSha256', 'dependencyReceiptSha256'], 'generic execution');
        assert.equal(m.native.execution.ordinaryObjectSha256, artifact.sha256); for (const value of Object.values(m.native.execution)) hash(value);
      }
    }
  }
  if (row.phase === 'build') { assert.equal(m.invocation, null); if (!generic) assert.deepEqual(m.results, []); }
  else {
    exact(m.invocation, ['export', 'arguments', ...(generic ? ['outcomes'] : [])], 'frozen invocation');
    assert.equal(m.invocation.export, family.export); assert.deepEqual(m.invocation.arguments, family.arguments);
    let results = generic ? m.invocation.outcomes : m.results;
    if (!generic && !ownership) results = results.map(result => {
      exact(result.outcome, ['kind', 'type', 'value'], 'scalar manifest outcome');
      return { target: result.target, outcome: { kind: result.outcome.kind, value: { type: result.outcome.type, value: result.outcome.value } } };
    });
    assert.deepEqual(results, row.expectedResponse.results, 'manifest and public typed results agree');
  }
  return { status: 'fixed-manifest-projection-checked', sourceAuthenticated: false, performanceQualified: false,
    baselineAuthority: null, thresholds: null };
}
