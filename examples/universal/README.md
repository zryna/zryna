# Universal examples

Small programs used to prove behaviorally equivalent JavaScript, WebAssembly, and native
artifacts one governed slice at a time.

`add.zry` currently passes the authenticated frontend, strict semantics, verified `I32V1` IR, and
direct JavaScript pipeline. Its generated `.mjs` module is imported under Node.js and checked for
ordinary values, signed 32-bit wrapping boundaries, strict host carriers, exact arity, and stable
exports. Direct WebAssembly and native executable coverage remain later M1 gates; this directory
does not imply those targets are already implemented.
