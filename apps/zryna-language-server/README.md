# Zryna language server

Thin, fail-closed stdio transport for the compiler-owned protocol-v2 scalar diagnostic session
and definition query. The component owns LSP framing, document lifecycle, revision correlation,
position conversion, cancellation routing, and inert rendering only. `zryna-driver` retains
frontend execution, session scheduling, diagnostic publication, and semantic query authority;
`zryna-source` validates the exact in-memory UTF-8 text and coordinates. The integration tests use
driver-owned fixture support; the transport does not call or configure a frontend provider.

The server accepts `initialize`, `initialized`, `textDocument/didOpen`, full-text
`textDocument/didChange`, `textDocument/didClose`, `textDocument/definition`, `$/cancelRequest`,
`shutdown`, and `exit`. It negotiates `utf-8`, `utf-16`, or `utf-32` positions, defaults to
`utf-16`, and accepts only `file:` documents strictly below the initialized root URI. Each source
mutation creates a new immutable compiler revision. Diagnostics are published both as standard
source diagnostics and as an exact `zryna/publishDiagnostics` notification carrying the complete
structured-diagnostics-v2 report and revision pair.

Run the binary with absolute compiler workspace and pinned Node.js 22.22.1 paths:

```text
zryna-language-server --compiler-root <absolute-path> --node <absolute-node-path>
```

The compiler root supplies the fixed registered `adapters/typescript-6/src/worker.mjs`; protocol
messages cannot select an executable, provider identity, filesystem read, network request, build,
or code execution. The current public semantic slice is the one-file protocol-v2 scalar profile,
so one connection admits at most one open document until module resolution has a reviewed tooling
authority. Control-flow/data-ownership queries, multi-file module resolution, hover, references, rename,
completion, code actions, formatting, indexing, debugging, and execution are unsupported.
