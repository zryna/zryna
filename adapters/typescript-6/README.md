# TypeScript 6 frontend adapter

This isolated worker reads a strict TypeScript-compatible bootstrap subset with the public
TypeScript 6 compiler API and returns a protocol-v2 ZRYNA-owned syntax snapshot. It does not
define Zryna semantics, construct Zryna IR, or emit target code. The handshake reports the exact
runtime compiler version; the workspace lock and override pin that implementation to `6.0.3`.

The protocol is newline-delimited JSON over standard input/output. Provider-specific numeric syntax kinds, node identities, type identities, and symbols never cross the boundary.

## Supported bootstrap grammar

- named, explicitly exported, non-default synchronous functions with bodies;
- identifier parameters without modifiers, optional markers, rest syntax, defaults, or
  destructuring;
- missing annotations, explicit `any`, and simple unqualified named annotations;
- value-return statements;
- identifier references, `true`, `false`, canonical decimal integers (including negative
  integers), and binary addition.

Everything else receives a located error diagnostic and the affected function is omitted. This
includes parse recovery trees, imports, variables, classes, overloads, generics, async and
generator functions, dynamic calls or property access, control flow, and unsupported annotation
forms. A verified provider error stops compilation before semantic lowering. Missing annotations
and `any` remain explicit syntax so Zryna-owned semantics can reject them under the universal
profile.

## Boundary guarantees and budgets

The worker rejects malformed UTF-8, duplicate or unknown JSON fields, non-portable or
case-colliding paths, ill-formed UTF-16 source, and wrong protocol versions. TypeScript UTF-16
locations are converted to exact half-open UTF-8 byte spans. Files are normalized in ordinal path
order, expression arenas use canonical postorder, and diagnostics use deterministic top-K
selection.

| Budget | Limit |
| --- | ---: |
| Request / response bytes | 72 MiB / 16 MiB |
| JSON depth / containers / fields / lexical units | 8 / 4,100 / 8,200 / 50,000 |
| Files | 4,096 |
| Source bytes per file / project | 2 MiB / 64 MiB |
| Lines per file / project | 100,000 / 1,000,000 |
| TypeScript nodes per file / project | 262,144 / 1,048,576 |
| Parser delimiter nesting | 512 |
| Functions per file / project | 4,096 / 16,384 |
| Parameters per function / project | 256 / 262,144 |
| Statements per function / project | 4,096 / 65,536 |
| Expressions per function / project | 16,384 / 262,144 |
| Expression depth | 128 |
| Integer spelling | 64 ASCII bytes |
| Diagnostics | 256, including the terminal truncation diagnostic |
| Identifier or diagnostic text | 1,024 / 4,096 Unicode scalar values |

Stable adapter errors use `ZRYNA-F1001` for malformed requests, `ZRYNA-F1002` for exceeded
budgets, `ZRYNA-F1003` for provider invariants, `ZRYNA-F2002` for unsupported syntax, and
`ZRYNA-F2003` for diagnostic truncation. TypeScript syntactic diagnostics retain their pinned
`TS` number and a source location.

## Protocol v3 worker

Protocol v2 remains frozen at `src/worker.mjs`. The separate `src/worker-v3.mjs` entrypoint
implements the provider-neutral protocol-v3 syntax boundary for the specified `ControlFlowV1`
profile. Its exact handshake adds `control_flow_v1: true` while retaining
`module_resolution: false` and `semantic_diagnostics: false`.
The package's `zryna` registration intentionally remains the immutable protocol-v2 registration;
Issue #46 exposes v3 only as this explicit, not-yet-driver-registered worker path.

The v3 worker reports source-faithful named imports, exported or internal functions, explicitly
typed `const` and `let` declarations, assignment, return, lexical blocks, `if`/`else`, `while`,
direct identifier calls, scalar literals, references, negation, and the profile's exact binary
operators. It preserves keyword, punctuation, operator, identifier, type, and module-specifier
spans as half-open UTF-8 byte ranges. Blocks and statements use canonical preorder/source order;
expressions use evaluation-order postorder.

The provider does not resolve imports, names, calls, or types. Module specifiers remain normalized
source values with their token/value spans and contain no resolved path or compiler symbol
identity. Recognized syntax outside the v3 contract, including a TypeScript parse-recovery tree,
fails the complete analysis request with `ZRYNA-F2002`; the worker never returns a reduced
snapshot containing only the supported siblings.

Run `pnpm test:v2` to verify the immutable v2 adapter, `pnpm test:v3` for the new boundary, or
`pnpm test` for both. The v3 request/response limits are 72 MiB/64 MiB and its aggregate source
limit is 8 MiB; finer syntax limits match the `ControlFlowV1` contract.
