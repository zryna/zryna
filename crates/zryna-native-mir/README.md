# Zryna native MIR

Typed straight-line native values lowered from `VerifiedProgram` or supplied as explicit raw
compiler claims.

`raw::Module` and its nested raw types are never backend-authoritative. `verify` consumes those
claims and is the only constructor of `VerifiedMirModule`; the verified wrapper exposes only
immutable function/value/operation views and cannot be recovered as raw or mutated. `lower` sees
only sealed Universal IR views, builds the same raw shape, and passes it through that verifier.

The current verified profile proves:

- bounded symbols matching `[A-Za-z_][A-Za-z0-9_]*`, with exact and ASCII-case-folded uniqueness;
- one provisional fixed, non-variadic internal `i32` convention;
- `i32` parameter, result, definition, literal, and wrapping-add types;
- explicit dense value IDs defined exactly once in canonical slot order;
- existing same-function operands that strictly precede each use, plus iterative cycle rejection;
- an existing result whose type matches the signature; and
- fixed per-function, module, symbol, and diagnostic budgets.

MIR is SSA-like rather than a Universal IR tree: repeated uses, `add(value, value)`, and bounded dead
values are valid. Raw diagnostics are global and identify only bounded function/value ordinals.

The current straight-line MIR remains a foundation representation. A separate mandatory MIR
profile must define blocks, dominance, calls, and transformed control flow when those operations are
introduced. The provisional convention is not scalar ABI v1, an FFI contract, or a final native
symbol mapping.
