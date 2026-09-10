# Language server protocol v1

Status: public, bounded stdio transport for the existing protocol-v2 scalar diagnostic session and
definition query. This does not expand the language profile, semantic query inventory, compiler
execution surface, or editor-extension distribution.

## Start and initialize

Run `zryna-language-server --compiler-root <absolute-path> --node <absolute-path>`. The compiler
root must contain the registered TypeScript 6 adapter and `--node` must identify the exact pinned
Node.js 22.22.1 runtime. Both paths are configuration established before protocol input; LSP
messages cannot select a worker, process, provider, filesystem root, build target, network endpoint,
or executable action.
Pre-protocol root/adapter configuration failures use `ZRYNA-D3001`; existing pinned-runtime
failures retain their `ZRYNA-R3xxx` codes. They are written as inert standard error text before
the server accepts protocol input.

The server uses LSP/JSON-RPC 2.0 over standard input/output. Each message has one required ASCII
`Content-Length` header, an optional exact UTF-8 `Content-Type` header, `\r\n` separators, and a
complete payload of at most 16 MiB. Duplicate, unknown, malformed, oversized, or truncated headers
terminate the connection. JSON-RPC objects reject malformed versions, IDs, method names, duplicate
fields, trailing values, and method-specific shape errors without publishing compiler results.

`initialize` requires one `file:` `rootUri` and client capabilities. The server selects the first
client-listed `utf-8`, `utf-16`, or `utf-32` position encoding and otherwise uses the LSP default
`utf-16`. Document URIs must be exact descendants of that root. Percent decoding is strict;
traversal, non-portable paths, case aliases, invalid UTF-8, queries/fragments, NUL, backslashes, and
source-limit violations fail closed through URI or `zryna-source` validation.

## Supported methods

| Method | Contract |
| --- | --- |
| `initialize`, `initialized` | Negotiate one connection and its exact position encoding. |
| `textDocument/didOpen` | Admit one `zryna` full-text overlay with a nonnegative version. |
| `textDocument/didChange` | Require exactly one full-text replacement and a strictly increasing version. |
| `textDocument/didClose` | Remove the overlay, invalidate its revision, and clear published diagnostics. |
| `textDocument/definition` | Resolve the selected scalar function/parameter identifier through the semantics-owned definition index. |
| `$/cancelRequest` | Cancel one admitted definition query by its exact JSON-RPC ID. |
| `shutdown`, `exit` | End the connection in order without executing project code. |

The protocol-v2 scalar profile admits at most one open document per connection. Full-text sync is
deliberate; incremental range edits are not advertised or accepted. Hover,
references, rename, completion, code actions, formatting, symbols/indexing, debugging, builds,
execution, module resolution, control-flow/data-ownership queries, and workspace mutation are not
implemented. Unknown requests receive `Method not found`; unknown notifications have no effect.

## Revisions, diagnostics, and definitions

Every accepted open/change/close source set builds a new immutable `SourceMap` and driver-owned
session revision. Versions must advance even for same-length edits, edit-and-undo, or identical
replacement bytes. In-flight work retains the exact snapshot/revision/source authority. The driver
rechecks that authority immediately before publication; replacement, close, foreign URI, eviction,
deadline, cancellation, malformed input, or resource exhaustion cannot publish a partial or old
result. A stale definition receives LSP `ContentModified`; cancellation receives
`RequestCancelled`. Absent supported symbols return `null` only after the active semantic authority
reports `absent`.

Each ready revision emits two notifications:

- `zryna/publishDiagnostics` contains the opaque `snapshot`, monotonic `revision`, exact document
  URI/version list, and the complete structured-diagnostics-v2 report unchanged. This is the
  revision-bearing authority, including global/workspace locations and terminal `ZRYNA-D2001`.
- `textDocument/publishDiagnostics` contains only source-located diagnostics for each open document.
  It preserves code/severity/message, derives the exact negotiated range from the retained text,
  and retains original guidance/location in inert `data`. It never invents a document range for a
  global or workspace label.

Definition requests use the active document and negotiated position to derive one exact UTF-8 byte
offset. Token ends, whitespace, comments, and EOF return `null`; invalid lines/columns, UTF-8 scalar
splits, UTF-16 surrogate splits, and non-round-trippable CRLF interiors reject. A successful result
contains the declaration URI and exact converted range issued by the retained semantic index.

## Limits and compatibility

The transport adds a 16 MiB frame limit and otherwise preserves the query-session ceilings:
65,536-byte logical requests, 1,048,576-byte responses, depth 64, 100,000 work units, 10,000 results,
two revisions, 64 MiB retained cache, 32 in-flight requests, 128-byte IDs, and a 30-second deadline.
Source/provider limits remain independently enforced. Cache hits do not reduce logical charges.

The component depends only on the driver orchestrator and source foundation. Driver-owned fixture
support authenticates a fixed in-memory syntax snapshot to prove nonzero cancellation and
stale-revision behavior at valid offsets; it does not move frontend execution into the transport.
Provider identity stays behind the driver boundary and is absent from LSP messages. Future field,
method, sync-mode, coordinate, diagnostic, or limit changes require a separately reviewed compatible
extension or protocol revision; they do not silently activate query forms specified but not yet
implemented in the semantic-query v1 design.

Focused verification is `cargo test --locked -p zryna-language-server`. It covers framing,
initialization, lifecycle, same-length revision races, cancellation/recovery, URI and coordinate
hostiles, unsupported methods, exact diagnostics, definition positions, and the real stdio process
on the supported CI operating systems. Complete repository gates remain required before merge.
