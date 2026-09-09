# Zryna frontend contract

Versioned, provider-neutral boundary for replaceable TypeScript and native Zryna frontends.

Provider output is untrusted. Protocol-v1 adapters retain their declaration-only legacy contract.
The protocol-v2 and protocol-v3 process runners launch an absolute executable directly without a
shell, perform an exact identity/version/protocol/capability handshake, and only then send the
authoritative `SourceMap` contents for analysis. Their typed expectations and verified result APIs
are separate, so neither transport can reinterpret or silently upgrade the other. After the
operating system returns a successful spawn, one
monotonic deadline covers handshake, analysis, pipe drains, process exit, and reserved cleanup.
The worker starts in a fresh Unix process group or Windows Job Object with a cleared environment;
only Windows system-root variables required to start the executable are retained. NDJSON messages,
aggregate stdout, and stderr all have fixed byte limits; request IDs, response count, clean EOF,
successful exit, and bounded cleanup are mandatory. Unix cleanup polls for an empty process group;
Windows cleanup requires a successful Job-wide termination request plus leader and I/O completion.

The core verifies fixed item budgets and the exact canonical file-id/path set against `SourceMap`,
and converts every raw UTF-8 range into an opaque, map-bound `Span`. The driver-facing API returns
only the resulting verified project. Raw provider bytes and DTOs do not cross that boundary.

`native_lexer` is the first internal native-frontend stage. It walks canonical `SourceMap` files,
retains a lossless ordered stream of tokens and whitespace/comment trivia, and issues only
source-map-authenticated UTF-8 spans. Its ASCII identifier boundary and protocol-v4 punctuation,
keyword, decimal, and unescaped string inventory are deterministic; malformed scalars, strings,
and comments recover at character boundaries with stable diagnostics. Fixed token, trivia,
project, diagnostic, and protocol-v4 aggregate-source budgets fail atomically as `ZRYNA-F1502`;
recoverable malformed input is reported as `ZRYNA-F1501`. The source layer admits only valid UTF-8
before it can construct the required `SourceMap`; the lexer never decodes, normalizes, or repairs
raw bytes. Identifiers are ASCII and at most 128 bytes, strings are single- or double-quoted with
no escapes or line terminators, and `//` and `/* ... */` comments remain lossless trivia. The
lexical inventory covers the v4 keywords plus braces, brackets, parentheses, `: ; , .`, `< <= >
>=`, `= => === !==`, and `+ - *`. Per-file token and trivia limits are 65,536 each; the project
retains at most 262,144 combined lexemes, 256 diagnostics, and 8 MiB of source. This stage does not
parse, create protocol-v4 snapshots, implement a provider, or change bootstrap/public selection.
Run `cargo test --locked -p zryna-frontend --test native_lexer --test native_lexer_fuzz` for the
ordinary corpus. Add `-- --include-ignored` to execute the two proportional production-limit token,
trivia, and project-lexeme proofs without lowering their limits.

Protocol v1 intentionally carries declarations and diagnostics only. Protocol v2 is a separate
executable-syntax contract owned by `zryna-syntax`; it does not change v1 semantics in place. The
TypeScript 6 adapter implements the protocol-v2 executable-syntax contract. Protocol v3 has its own
syntax-only worker and source-map-verifying transport, including the exact
`control_flow_v1: true`, `module_resolution: false`, and `semantic_diagnostics: false`
capabilities. It is not connected to the driver, semantics, backends, or CLI.
