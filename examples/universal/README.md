# Universal examples

Small programs used to prove behaviorally equivalent JavaScript, WebAssembly, and native
artifacts one governed slice at a time.

`add.zry` currently passes the authenticated frontend, strict semantics, verified `I32V1` IR,
direct JavaScript pipeline, and direct core WebAssembly pipeline. Its generated `.mjs` module is
imported under Node.js and checked for ordinary values, signed 32-bit wrapping boundaries, strict
host carriers, exact arity, and stable exports. Its generated import-free `.wasm` module is
validated, inspected, instantiated, and executed through Node's standard WebAssembly API with the
same wrapping boundaries. Native executable coverage remains a later M1 gate.
