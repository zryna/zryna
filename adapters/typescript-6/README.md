# TypeScript 6 frontend adapter

This isolated worker reads TypeScript-compatible source with the public TypeScript 6 API and returns a versioned ZRYNA-owned syntax snapshot. It does not define Zryna semantics, construct Zryna IR, or emit JavaScript.

The protocol is newline-delimited JSON over standard input/output. Provider-specific numeric syntax kinds, node identities, type identities, and symbols never cross the boundary.
