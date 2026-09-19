# Language server protocol v1

Status: public, bounded stdio transport for the existing protocol-v2 scalar diagnostic session and
definition query, with the opt-in scalar-format-v1 presentation surface. This does not expand the language profile, semantic query inventory, compiler
execution surface. The initial editor package is described below; marketplace publication remains pending.

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
| `textDocument/formatting` | Format a verified scalar document with the canonical style. |
| `textDocument/rangeFormatting` | Format only complete verified functions inside the selection. |
| `$/cancelRequest` | Cancel one admitted definition or formatting query by its exact JSON-RPC ID. |
| `shutdown`, `exit` | End the connection in order without executing project code. |

The protocol-v2 scalar profile admits at most one open document per connection. Full-text sync is
deliberate; incremental range edits are not advertised or accepted. Hover,
references, rename, completion, code actions, symbols/indexing, debugging, builds,
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

## Scalar format v1

This initial formatter supports only the existing one-file protocol-v2 scalar profile: exported
functions with explicit i32 parameters/results and the admitted return, reference, integer and
addition expressions. It consumes the exact verified snapshot only after successful scalar semantic
admission. M2 control-flow/modules and M3 data/ownership formatting remain unsupported; their
compiler support does not imply editor support. Parenthesized expressions, classes, imports,
incomplete syntax and semantic errors receive no edits. Issue #409 remains open for the broader
admitted surface and marketplace publication.

The canonical style uses two spaces inside function bodies, one space between words and around
addition, no space before commas/colons/semicolons or inside parameter parentheses, a space after
commas/colons, and LF after opening/closing braces and semicolons. Nonempty documents end with LF.
Token spellings/order, comment bytes (including newlines inside block comments), integer spelling
and declaration identity remain unchanged. The formatter never inserts/removes tokens, sorts
imports, evaluates code, or writes files. Line-comment terminators become LF; comments remain in
token order, although a trailing comment after a semicolon occupies the following line.
Formatting options are parsed for LSP compatibility; tabSize must be positive, but the canonical
style does not depend on editor indentation preferences.

Document formatting returns either one whole-document edit or an empty list for canonical text.
Range formatting requires exact negotiated coordinates and accepts complete function spans plus
surrounding trivia. It does not expand the selection: intersecting a partial function rejects the
whole request, and trivia-only/empty selections return no edits. Each returned edit covers only
one complete function, preserves text before/after it byte-for-byte, and omits a new trailing LF
outside that function. All edits derive from the retained active revision; cancellation, close,
replacement and expiry reject pending work. Applying the edits belongs to the editor.

The prepared document is capped at 131,072 UTF-8 bytes; unavailable/over-limit preparation cannot
produce edits. Its exact text, path and function-boundary bytes count against the existing session
cache. JSON results remain capped at 1 MiB and 10,000 edits; definition and formatting share the
32-request queue. No text is sent over a network.

| Stable code | Meaning |
| --- | --- |
| ZRYNA-D4001 | No admitted formatting state: invalid/unsupported syntax or semantics, unavailable diagnostics, unsupported whitespace or preparation limit. |
| ZRYNA-D4002 | Requested revision is no longer active (LSP ContentModified). |
| ZRYNA-D4003 | Invalid coordinate, reversed selection or partial function. |
| ZRYNA-D4004 | Request/result limit exceeded. |

Failures carry the code in LSP error.data.code and never contain result edits. Existing compiler
diagnostics remain authoritative. Some incomplete provider reports cannot be represented by the
existing diagnostic-v2 contract; their analysis remains unavailable and formatting still fails
closed with D4001. Cancelled requests use LSP RequestCancelled; expired requests fail without edits.

## Initial editor installation and compatibility

The VS Code/Open VSX package lives in editors/vscode-zryna. It is a local installable Developer
Preview, not a marketplace publication. It provides diagnostics, definition, document formatting
and range formatting for one active local file at a time. Switching files starts a fresh bounded
connection; it does not enable module resolution. It has no runtime package dependencies, telemetry,
download/update behavior, debugging or general filesystem write service. A separate explicit
editor Run command is described below; it does not add execution to the language-server protocol.
Diagnostic messages render as plain text, and edits/definitions are validated against the same
document and version before returning them to VS Code.

| Extension | Editor engine | Required compiler | Source profile |
| --- | --- | --- | --- |
| 0.3.0 | VS Code-compatible API >=1.82.0 | Server 0.3.0 advertising scalar-format-v1 and portable-setup-v1 | One-file scalar-v2 |
| 0.3.0 | Same | Public immutable v0.2.3 language server | Incompatible: no formatting capability |

The semantic package version alone is insufficient: the extension verifies server name/version,
UTF-16 positions, both formatting methods and the exact scalar-format-v1 capability before sending
document contents. No new compiler release/tag is created by this work; distributing this server
in a future compiler release is a separate release dependency.

From the matching reviewed source checkout, use the pinned toolchains:

~~~text
pnpm install --frozen-lockfile
pnpm preflight
pnpm m0:check
cargo build --locked -p zryna-language-server
pnpm editor:check
pnpm editor:package
code --install-extension /absolute/compiler/checkout/.zryna/out/zryna-0.3.0.vsix
~~~

Set zryna.serverPath, zryna.compilerRoot and zryna.nodePath in USER settings to absolute paths.
The first names the built target/debug/zryna-language-server executable (.exe on Windows); the
second names that trusted compiler checkout with its pinned adapter dependencies, and the third
names the exact Node.js 22.22.1 executable. Workspace-provided overrides are ignored. The extension
is disabled in untrusted or virtual workspaces. Open the edited project's folder and a .zry file,
then use the editor's Format Document/Format Selection or Go to Definition commands. This does
not add a zryna fmt CLI command or any compiler execution flag.

Packaging uses pinned @vscode/vsce 4.0.0 without dependencies or signing. Its optional signing
executable installer is explicitly disabled; the VSIX contains only its manifest, client/Run
modules, lexical grammar, README, changelog and license. Publication requires reviewed exact-package provenance,
a configured marketplace publisher/namespace and its credentials. None are provisioned or embedded
by this package. See the package changelog for the initial release notes.

## Explicit editor Run

The [portable setup candidate](PORTABLE_SETUP.md) adds a standalone installation path without
changing Run's source/target limits. User settings select an installation and independently
supplied manifest digest; complete file verification precedes server startup. The server's exact
source revision must match the manifest before the client transmits document contents. Installed
mode uses fixed authenticated Node/provider bytes from the unchanged compiler 0.2.3 distribution.
The outer candidate is not authenticated by that compiler's signature and is not a public release.

Issue #457 adds lexical highlighting and an explicit Zryna: Run Saved File command. This newly
authorized command supersedes #409's original no-project-execution scope only for a user-triggered
run in a trusted local workspace. Opening/saving, diagnostics, definition and formatting never
execute project code. The language-server protocol and its execution exclusions remain unchanged.

Set zryna.compilerPath in user settings to the absolute official installed v0.2.3 compiler
executable, separately from the source-built formatting server. Workspace overrides are ignored.
Run prompts for one exported function with explicit i32 parameters/result, JavaScript/WebAssembly,
and every required canonical signed 32-bit argument. There are no hardcoded invocation values.
Lexical discovery is advisory; compiler admission remains authoritative. The standalone i32-v1
package source limit is 1024 UTF-8 bytes; M2/M3, modules, bool invocation and native Windows
executables are outside this command's scope.

Unsaved input is rejected, and source changes during selection require a fresh invocation. Each
saved snapshot goes to a new compiler-created project in extension global storage. The extension
updates only its generated source inventory size/hash, invokes package resolve in update mode,
then invokes the installed compiler directly with an argument array and no shell or root/runtime
override. It does not bypass package authentication or insert executable helpers. Picker
cancellation starts no processes; progress cancellation and bounded subprocess deadlines stop
waiting. Run directories are retained for inspection and manual cleanup. No source files are
rewritten. The output channel identifies the snapshot and reports actual compiler results/errors.

Open Generated JavaScript and Reveal Run Output use the latest successful invocation's actual
artifact after checking the manifest source/invocation and emitted bytes/hash. JavaScript opens
as source; Wasm is revealed in the OS file explorer. This is an editor package update, with no
compiler release/tag or marketplace publication. Broader formatting and publication remain #409.
