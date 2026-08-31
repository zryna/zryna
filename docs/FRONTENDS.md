# Replaceable frontend providers

## Why TypeScript 6 is temporary

The bootstrap adapter uses the public TypeScript 6 API to avoid reimplementing parsing, source
locations, and basic declarations before the first backend slice exists. It performs no module
resolution and is selected for available API access, not runtime speed.

The adapter wrapper and its actual compiler implementation are locked, and the worker reports the
runtime compiler version in its exact protocol-v2 handshake. It may return only ZRYNA-owned
normalized data. Zryna semantics remain authoritative.

## TypeScript 7 readiness

TypeScript 7.0 does not ship a supported programmatic compiler API. Its CLI can run as a shadow compatibility check, but it is not a frontend provider. A TypeScript 7 adapter will be added only after an upstream API exposes the capabilities required by the provider conformance suite.

Provider selection is capability-based:

```text
handshake
├── provider identity and exact version
├── Zryna protocol version
├── module-resolution capability
└── semantic-diagnostics capability
```

A provider version mismatch, crash, malformed message, incomplete project file set, or reported
unsupported syntax fails closed. Until the independent native frontend exists, completeness inside
each source file is a conformance property of the exact pinned provider rather than something the
provider-neutral DTO verifier can independently reconstruct from source text.

## Source-coordinate contract

Protocol v2 `span.start` and `span.end` are zero-based, half-open UTF-8 byte offsets. The
TypeScript adapter therefore converts its native ECMAScript UTF-16 code-unit positions before
emitting a snapshot. It rejects ill-formed strings and never rounds an offset that splits a
surrogate pair. Non-ASCII adapter fixtures are required on Linux and Windows.

The compiler constructs the authoritative bounded `SourceMap`. Protocol-v2 responses deserialize
into `RawProjectSyntaxSnapshot`, whose IDs, paths, collection bounds, diagnostics, and
`UntrustedSpan` values have no compiler authority. `verify_snapshot` requires the exact complete
file set, canonical identifier/path pairs, bounded declarations and text, and source-map-valid
UTF-8 ranges before constructing `ProjectSyntaxSnapshot`. Provider transport bytes are bounded
before JSON decoding. Protocol v2 extends the original declaration boundary with executable
syntax without changing the meaning of `start` and `end`. Its raw DTOs and verified types
live in `zryna-syntax`, below all provider implementations. See
[Syntax protocol v2](SYNTAX_PROTOCOL_V2.md).

## Protocol migration state

| Capability | Protocol v1 | Protocol v2 |
| --- | --- | --- |
| Declaration signatures | retained as a legacy core contract | implemented |
| Executable function bodies | unavailable | contract and verifier implemented |
| TypeScript 6 emission | legacy declaration path | implemented and fail closed |
| Semantic lowering | unavailable | implemented for the first strict one-file subset |
| Native Zryna provider | unavailable | eventual provider target |

Selecting v2 requires an exact v2 handshake. No compiler path silently upgrades a v1 response.
Provider error diagnostics stop compilation before semantic lowering. Provider warnings remain
observable on successful source-to-verified-IR results. The pinned TypeScript 6 suite proves that
its recognized unsupported constructs produce such an error and that expression
rollback leaves no smaller accepted arena. A structurally valid provider that silently omits an
arbitrary source subtree is outside what the DTO verifier alone can prove; differential provider
conformance and the future native parser close that remaining trust assumption.

## Worker discovery and lifecycle

The host resolves the worker executable, argument list, and working directory before constructing
`WorkerSpec`; executable and directory paths must be absolute. `WorkerFrontend` passes every
argument directly to `Command` and never invokes a shell or parses a command-line string. Windows
batch wrappers are rejected. The inherited environment is cleared before launch; on Windows only
`SystemRoot` and `WINDIR` are copied when present so the operating system can start the executable.
No caller `PATH`, Node preload/search setting, credential, proxy, or adapter test override reaches
the provider implicitly.

The worker is the leader of a fresh Unix process group or the root of a Windows Job Object. The
configured executable is trusted to leave its descendants in that containment boundary. The
post-spawn session reserves part of its single deadline for explicit cleanup and drains all three
pipes on every return path. Unix cleanup signals the group and polls until it is empty. Windows
cleanup uses suspended creation with Job assignment before resume, requires a successful Job-wide
termination request, then confirms leader exit and pipe/task completion. `ZRYNA-F1108` reports any
cleanup-protocol failure. The non-blocking drop guard is only an emergency fallback; it never
performs an unbounded wait.

Each analysis uses one fresh child process and one authenticated NDJSON session:

1. send request id 1 for `handshake`;
2. require the configured provider id, exact runtime version, protocol, and complete capabilities;
3. only after that succeeds, send request id 2 with the authoritative source set;
4. require exactly one correlated analysis response, clean EOF, and successful process exit;
5. decode and verify the snapshot against the same `SourceMap` before returning it to the driver.

After the operating system returns a successful spawn, a single monotonic 30-second maximum
deadline covers the authenticated session and its cleanup. Synchronous operating-system spawn time
is outside that timer; a spawn error fails as `ZRYNA-F1101`. The handshake frame is limited to
64 KiB, an analysis response to 16 MiB, aggregate stdout to their combined fixed bound, stderr to
64 KiB, and each serialized request to 72 MiB. Callers may tighten these budgets but cannot expand
them. Timeout, overflow, malformed or extra frames, wrong ids, provider rejection, pipe failure,
and unsuccessful exit all fail closed with stable `ZRYNA-F11xx` categories. Every post-spawn
failure closes stdin, requests containment-wide termination, and confirms the platform-specific
bounded cleanup protocol before returning.

## Migration to a native Zryna frontend

The permanent provider-neutral path is:

```text
native Zryna lexer → parser → versioned raw syntax snapshot → matching Zryna verifier
```

Protocol v2 is the implemented M1 instance of this path. Protocol v3 now has a separate exact
schema, pinned TypeScript 6 syntax-only worker, opaque source-map-bound verifier, and typed worker
transport. It is not selected by the driver and does not activate an M2 compiler or CLI profile.
Because all later phases depend on ZRYNA-owned verified syntax and IR, replacing the bootstrap
provider must not modify semantic behavior or any backend.

## Protocol v3 and internal module discovery

M2 protocol v3 adds source-faithful DTOs for named imports, locals, assignment, direct calls,
exact operators, blocks, `if`, and `while`. It is a new exact protocol; protocol v2 is not modified
or implicitly upgraded. The TypeScript 6 provider will continue to advertise
`module_resolution: false`: it reports import specifiers and spans but never reads a dependency,
chooses a resolved path, or supplies a compiler symbol identity.

The driver implements an isolated protocol-v3 module-discovery boundary. Starting from one
validated entrypoint, it analyzes only each newly discovered bounded batch, resolves verified
explicit relative `.zry` imports, and safely reads unresolved files through retained no-follow
workspace capabilities. After closure it builds the immutable source map once and requests one
final complete snapshot; only that final-map-bound snapshot can be returned by the closure API.
Exact path, graph, fixed-point, race, cycle, identity, and resource rules are documented in
[M2 deterministic module closure](M2_MODULE_CLOSURE.md) and specified normatively in
[scalar control flow and modules v1](../spec/language/CONTROL_FLOW_MODULES_V1.md). The public driver
and CLI do not select this internal boundary, and it does not yet enter M2 semantics.
