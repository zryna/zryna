# M2 deterministic JavaScript backend

Status: implemented as a sealed backend composed by the public driver only for explicit
`--profile control-flow-v1`. The backend itself cannot select protocol v3, publish a manifest, or
claim cross-target equivalence.

## Authority boundary

`zryna_backend_javascript::emit_control_flow` accepts only
`zryna_ir::control_flow_v1::VerifiedProgram`. Raw control-flow IR, syntax snapshots, source names,
paths, and provider identities cannot cross this API. The existing M1 `emit` API and public M1
driver remain unchanged.

Modules and functions are visited once in sealed canonical order. Private implementation names are
derived only from dense verified module and declaration identities. Dependency exports and
unexported functions remain private. Only `public_export()` entry functions receive scalar ABI v1
wrappers and export aliases. Because every local implementation and wrapper name contains `$`, it
cannot collide with the verified public ABI grammar, including public names such as `Math`,
`Number`, or `Object`.

## Exhaustive lowering

The backend lowers every current `ControlFlowV1` instruction without a wildcard fallback:

| Verified instruction | Deterministic ECMAScript behavior |
| --- | --- |
| `BoolLiteral`, `I32Literal` | primitive Boolean or exact signed integer literal |
| `I32Add`, `I32Sub`, `I32Neg` | signed low 32 bits using explicit bitwise normalization |
| `I32Mul` | private 16-bit decomposition helper producing the exact low 32 bits without an ambient intrinsic |
| `Eq`, `Ne` | strict `===` or `!==` over equal verified scalar types |
| signed comparisons | direct signed Number comparisons producing primitive Boolean values |
| `DirectCall` | private call derived only from sealed `FunctionIdentity`, with arguments in verified order |

Each function uses dense function-scoped value slots and a block dispatcher. `Return` returns one
verified value. `Jump` first reads all edge arguments into private scratch slots, then assigns all
target block parameters, preserving parallel SSA edge semantics even for loop-carried swaps.
`Branch` tests `condition === true`, never JavaScript truthiness, and performs the same simultaneous
edge transfer on the selected arm. Loops therefore reevaluate their verified header instructions
on every visit without duplicating source expressions.

The generated module contains no dynamic import, `eval`, `Function`, `require`, filesystem,
network, environment, clock, randomness, or target runtime helper. It has LF line endings and one
final newline. The same linear renderer first counts every exact output byte, including every call
and edge argument, then runs once into a string reserved for exactly that count. The count is capped
at 32 MiB; reservation failure and the first byte beyond the selected budget fail with
`ZRYNA-J2003`, and no partial artifact is returned.

## Typed Node boundary

Entry wrappers check exact arity. An `i32` argument or result must be a primitive Number equal to
its signed 32-bit normalization and must not be negative zero. A `bool` argument or result must be
a primitive Boolean. There is no coercion, truthiness carrier, fractional Number, NaN, infinity,
or out-of-range fallback. Internal functions exchange only values already proved by the verifier
or constructed by exhaustive exact operations.

Driver integration tests import the public named exports and execute the sealed module through the
existing revalidated Node runtime capability on Linux and Windows CI. That path clears the
environment, caps output, imposes a deadline, and confirms process-tree cleanup. It encodes a
primitive Boolean result as exactly one byte `0` or `1` and an `i32` result as exactly four
little-endian bytes before typed scalar normalization. Tests cover exact overflowing
multiplication, arithmetic and comparisons, primitive
Boolean branches and results, wrong arity and carriers, diamond merges, parallel loop backedges,
direct same-module and cross-module calls, private dependency functions, adversarial public names,
repeated byte-identical emission, LF framing, noncanonical result frames, and the exact rendered
artifact-budget boundary. Process discovery, timeout containment, publication, and public profile
selection remain driver-owned; this backend does not create a second process runner or widen
runtime capabilities.

## Remaining M2 gates

Direct M2 core WebAssembly is documented by
[the separate sealed backend contract](M2_WEBASSEMBLY_BACKEND.md). The
[M2 native MIR profile](M2_NATIVE_MIR.md) and
[M2 Linux x86-64 native backend](M2_NATIVE_BACKEND.md) now independently reseal and execute the
same verified authority. The explicit-profile driver and
[manifest-v2 transaction](M2_MANIFEST_V2.md) are implemented. Fixed-oracle three-target
conformance and authenticated website/live closure remain Issues #56 and #57.
