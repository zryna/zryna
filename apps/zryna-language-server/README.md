# Zryna language server

Thin, fail-closed stdio transport for compiler-owned protocol-v2 scalar diagnostics/definition
and explicitly selected protocol-v3 M2 diagnostics. The component owns LSP framing, document lifecycle, revision correlation,
position conversion, cancellation routing, and inert rendering only. `zryna-driver` retains
frontend execution, session scheduling, diagnostic publication, and semantic query authority;
`zryna-source` validates the exact in-memory UTF-8 text and coordinates. The integration tests use
driver-owned fixture support; the transport does not call or configure a frontend provider.

The server accepts `initialize`, `initialized`, `textDocument/didOpen`, full-text
`textDocument/didChange`, `textDocument/didClose`, `textDocument/definition`, `$/cancelRequest`,
`textDocument/formatting`, `textDocument/rangeFormatting`, `shutdown`, and `exit`. It negotiates `utf-8`, `utf-16`, or `utf-32` positions, defaults to
`utf-16`, and accepts only `file:` documents strictly below the initialized root URI. Each source
mutation creates a new immutable compiler revision. Diagnostics are published both as standard
source diagnostics and as an exact `zryna/publishDiagnostics` notification carrying the complete
structured-diagnostics-v2 report and revision pair.

Run the binary with absolute compiler workspace and pinned Node.js 22.22.1 paths:

```text
zryna-language-server --compiler-root <absolute-path> --node <absolute-node-path>
```

Server 0.4.0 also accepts `--installed-root <absolute-verified-compiler-directory>`. It captures
the fixed distribution bootstrap closure, requires worker bytes equal to this build, and verifies
bundled Node/TypeScript hashes before execution. It reuses the same private staging and semantic
authority without a checkout, package manager or runtime override. Initial server authenticity
belongs to the independently verified setup, not adjacent checksums. `--version` reports version,
capability and embedded source revision. See [portable setup](../../docs/PORTABLE_SETUP.md).

The compiler root supplies the fixed registered `adapters/typescript-6/src/worker.mjs`; protocol
messages cannot select an executable, provider identity, filesystem read, network request, build,
or code execution. Without initialize options the server retains one-file protocol-v2 scalar
admission, definition, and `scalar-format-v1`. Exact
`initializationOptions: {"zrynaProfile":"control-flow-v1"}` selects one-file protocol-v3 M2
diagnostics and `control-flow-format-v1`; the initialize response echoes the selected capabilities.
M2 definition is not advertised. Each connection admits at most one open document until module
resolution has a reviewed tooling authority. Data-ownership queries, multi-file module resolution, hover, references, rename,
completion, code actions, indexing, debugging, and execution are unsupported.

The scalar and M2 formatting capabilities format only their selected verified profile. See the
[format contract and editor guide](../../docs/LANGUAGE_SERVER.md). M3 formatting and marketplace
publication remain outstanding under #409.
