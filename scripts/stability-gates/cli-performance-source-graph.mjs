// Static known-fixture import graph, not a general Zryna parser or source admission issuer.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { portable, sha256 } from './input.mjs';
import { validateMethodology } from './cli-performance-methodology-v4.mjs';
export function checkKnownFamilyGraph(family, readOriginal) {
  const destinations = new Map();
  const sources = new Map(family.sources.map(pin => [pin.path, pin]));
  assert.equal(family.moduleGraph.length, sources.size);
  for (const row of family.moduleGraph) {
    portable(row.original); portable(row.destination);
    assert(!destinations.has(row.destination), 'one physical module per destination');
    const pin = sources.get(row.original); assert(pin, 'original source pin');
    const bytes = readOriginal(pin.path, pin.bytes);
    assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256);
    const text = bytes.toString('utf8');
    const imports = [...text.matchAll(/^import\s+\{[^}]*\}\s+from\s+"([^"\n]+)";$/gmu)]
      .map(match => match[1]);
    assert.equal((text.match(/^import\b/gmu) ?? []).length, imports.length,
      'all imports in these exact four frozen fixture forms recognized');
    assert.deepEqual(imports, row.imports.map(edge => edge.specifier), 'actual source imports');
    destinations.set(row.destination, row);
  }
  assert(destinations.has(family.entry), 'entry is an actual mapped module');
  for (const row of family.moduleGraph) for (const edge of row.imports) {
    assert(edge.specifier.startsWith('./'), 'known closed relative import');
    const actual = posix.normalize(posix.join(posix.dirname(row.destination), edge.specifier));
    portable(actual); assert.equal(actual, edge.resolvesTo, 'actual lexical resolved filename');
    assert(destinations.has(actual), 'import resolves to a preserved source module');
  }
  return { modules: destinations.size, sourceAuthenticated: false, sourceConformanceQualified: false };
}
export function checkKnownGraphs(document, readOriginal) {
  validateMethodology(document);
  return document.families.map(family => ({ family: family.id,
    ...checkKnownFamilyGraph(family, readOriginal) }));
}
