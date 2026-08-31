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

Protocol v1 intentionally carries declarations and diagnostics only. Protocol v2 is a separate
executable-syntax contract owned by `zryna-syntax`; it does not change v1 semantics in place. The
TypeScript 6 adapter implements the protocol-v2 executable-syntax contract. Protocol v3 has its own
syntax-only worker and source-map-verifying transport, including the exact
`control_flow_v1: true`, `module_resolution: false`, and `semantic_diagnostics: false`
capabilities. It is not connected to the driver, semantics, backends, or CLI.
