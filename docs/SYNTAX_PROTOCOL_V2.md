# Syntax protocol v2

Protocol v2 is the provider-neutral executable-syntax boundary between replaceable source readers
and Zryna-owned semantic analysis. The checked-in JSON Schema is
`schemas/zryna-syntax-v2.schema.json`; the Rust decoder and verifier remain authoritative for
properties that JSON Schema cannot prove.

## Versioning and migration

Protocol v2 is a new exact version, not a reinterpretation of protocol v1. A provider must
advertise version `2`, return a v2-shaped response, and pass verification before its output can
enter semantic analysis. The TypeScript 6 bootstrap adapter emits this exact contract. Existing
v1 providers are never upgraded implicitly.

The initial executable subset represents:

- explicitly exported functions;
- parameter names and optional named type annotations;
- named or missing result types;
- return statements;
- references, Boolean literals, decimal `i32` literal spellings, and addition;
- one source span for every declaration, type, statement, expression, keyword, name, and operator.

The wire contract preserves a named spelling such as `any` and a missing annotation. The semantic
phase, not a provider, will reject unsupported or dynamic types with Zryna diagnostics.

## Wire shape

A response contains the exact project file set and bounded provider diagnostics. Each function
body contains a statement list and a flat expression arena. Statements and compound expressions
refer to arena entries by zero-based integer id.

Expression arenas use canonical postorder: every child id must be smaller than its parent id. Each
expression must have exactly one owner, either a statement root or one parent expression. This
single-owner rule rejects cycles, self edges, forward edges, shared nodes, and orphans without
recursing through provider-controlled data.

All structs reject unknown fields. Tagged variants use explicit `kind` values. File ids are dense
and must match the authoritative `SourceMap` path order exactly. All ranges are untrusted
zero-based, half-open UTF-8 byte offsets until the verifier resolves them through that exact source
map.

## Runtime invariants

The verifier additionally proves properties the schema cannot express:

- response files are the complete source-map file set, with exact ids and normalized paths;
- every range belongs to the claimed file, is in bounds, and ends on UTF-8 boundaries;
- nested ranges are contained and source-ordered;
- function and statement ranges are canonical and non-overlapping;
- aggregate counts use checked arithmetic;
- expression ownership, canonical postorder, and depth are valid;
- decimal `i32` spellings are canonical ASCII integers (range checking belongs to semantics);
- validation diagnostics are deterministic and retain a terminal budget error.

Only the opaque verified `ProjectSyntaxSnapshot` exposes compiler-consumable nodes. Raw DTOs never
carry source authority. A snapshot containing a verified provider error is reportable but cannot
construct `SemanticInput`; this is the compiler-owned stop gate for errors the provider reports.
The verifier proves the exact project file set and structural fidelity of returned nodes, but it
does not independently parse source text to prove that a provider returned every subtree. M0
therefore treats intra-file completeness as a pinned-provider conformance assumption and does not
claim that a silently omitting hostile provider is structurally impossible.

## Fixed limits

| Budget | Limit |
| --- | ---: |
| Serialized response | 16 MiB |
| Functions per file / project | 4,096 / 16,384 |
| Parameters per function / project | 256 / 262,144 |
| Statements per function / project | 4,096 / 65,536 |
| Expressions per function / project | 16,384 / 262,144 |
| Expression depth | 128 |
| Identifier or type spelling | 1,024 Unicode scalar values |
| Decimal literal spelling | 64 ASCII bytes |
| Provider diagnostics | 256 |
| Diagnostic message or guidance | 4,096 Unicode scalar values |
| Retained validation diagnostics | 256, including the terminal budget diagnostic |

Bounded sequence deserializers reject the first item over each per-container limit without storing
it. The serialized response byte cap is enforced before JSON decoding. Project-wide aggregate
limits are enforced during verification.

## Schema and conformance

`tests/fixtures/syntax-v2-valid.json` is shared by Rust serialization tests and the Ajv Draft
2020-12 suite. Negative fixtures cover unknown and missing fields. Rust tests additionally cover
duplicate fields, trailing JSON, oversized transport, exact file identity, UTF-8 boundaries,
expression ownership and depth, deterministic diagnostic selection, and bounded sequences.
The neutral `typescript-adapter-v2-{request,result}.json` pair is emitted byte-for-byte by the
bootstrap adapter and accepted by the authoritative Rust decoder and source-map verifier.

The schema is an interoperability aid, not a trust boundary. It mirrors field shapes, per-container
limits, canonical decimal spellings, and the portable path rules expressible in ECMAScript regular
expressions. The runtime additionally rejects Windows device stems, path components over 255
bytes, case-folded path collisions, wrong file identities, and all graph/source invariants. A
provider must therefore pass the Rust decoder and source-map-backed verifier even after schema
validation. Adapter emission has its own schema, adversarial, Unicode, determinism, and platform
conformance suite. Semantic lowering and target execution remain separate gates.
