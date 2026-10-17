import assert from 'node:assert/strict';
// Read the exact simple declaration form in the selected owning Rust source. Not a Rust evaluator.
export function providerPins(source) {
  return ['WRAPPER_MANIFEST', 'WRAPPER', 'TYPESCRIPT_MANIFEST', 'TYPESCRIPT'].map(name => {
    const arrays = [...source.matchAll(new RegExp('const ' + name + ': &\\[&str\\] =\\s*&\\[([\\s\\S]*?)\\];', 'g'))];
    assert.equal(arrays.length, 1);
    const body = arrays[0][1], components = [...body.matchAll(/"([^"\\]+)"/g)].map(m => m[1]);
    assert(components.length > 0 && body.replace(/"[^"\\]+"/g, '').replace(/[\s,]/g, '') === '');
    assert(components.every(v => v && !/[\\/]/.test(v) && v !== '.' && v !== '..'));
    const digests = [...source.matchAll(new RegExp('const ' + name + '_SHA256: &str =\\s*"([a-f0-9]{64})";', 'g'))];
    assert.equal(digests.length, 1);
    return { name, components, sha256: digests[0][1] };
  });
}
