# Core WebAssembly runtime boundary

The current `wasm-web` core `I32V1` artifact is a deterministic, import-free core WebAssembly 1.0
module. It needs
no bundled runtime, heap, garbage collector, filesystem, network, clock, randomness, or
environment capability. A conforming standard WebAssembly engine can validate and instantiate it
with an empty import object.

Exact Node.js 22.22.1 is the pinned conformance engine and the host for
`zryna run --target webassembly`. It uses the standard browser-compatible WebAssembly API, but
this is not evidence of execution in a browser or DOM. The raw JavaScript-to-WebAssembly boundary
performs host coercions; the public CLI validates its typed `I32V1` invocation first, but
applications must not treat raw API behavior as Zryna scalar ABI validation. A generated general
strict host wrapper is later work.

WASI, the Component Model, WIT, Canonical ABI types, memory-bearing modules, and host imports are
separate versioned profiles. They are not silently added to this capability-free core artifact.
