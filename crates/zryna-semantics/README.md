# Zryna semantics

Permanent Zryna-owned phase boundary from verified provider-neutral syntax to Universal IR. The
legacy M1 entry returns raw IR to its verifier call site; the isolated M2 entry seals IR internally.

`SemanticInput::try_new` accepts only a verified protocol-v2 snapshot bound to the exact immutable
`SourceMap` that issued it. It rejects every snapshot containing a provider error, so parse
recovery or unsupported syntax cannot enter name resolution, type checking, or lowering as a
smaller program. Provider warnings remain non-fatal and are preserved by `zryna-driver`.

## First strict source subset (protocol v2/M1)

`lower` currently requires exactly one source file containing at least one explicitly exported
function. It accepts:

- portable scalar-ABI export names that are unique exactly and under ASCII case folding;
- explicitly annotated `i32` and `bool` parameters and results;
- unique parameter bindings and references to those parameters;
- exactly one value-returning statement per function;
- `i32` literals in the inclusive range `-2147483648` through `2147483647`;
- `bool` literals; and
- left-associative `+` expressions whose operands are both `i32`.

Missing annotations, `any`, unknown types, unresolved names, duplicate parameters or exports,
invalid or colliding export names, out-of-range literals, non-`i32` addition, mismatched return
types, empty entrypoints, multiple files, and bodies without exactly one return fail with bounded,
deterministically ordered `ZRYNA-M1xxx` diagnostics. Semantic input sizes are compile-time proven
not to exceed the corresponding Universal IR limits.

The protocol-v2 bootstrap path rejects parenthesized expressions, calls, local declarations,
control flow, and heap-backed expressions before M1 semantics instead of normalizing a smaller
program. Multi-file and module semantics remain outside this legacy `lower` gate, which rejects
multiple files. Protocol v3 and the separate M2 entry below do not inherit those M1 restrictions.

M1 semantic success returns raw `Program`; only `zryna-ir::verify` can turn it into backend-safe
`VerifiedProgram`. `bool` is valid source semantics and lowers to `BoolLiteral`, but the current
`I32V1` verifier deliberately rejects it with `ZRYNA-I1006`. Therefore the current complete
source-to-verified-IR success path is the documented `i32` subset. Enabling `bool` requires one
future universal profile implemented consistently by every active backend.

This crate owns language meaning and must never depend on a replaceable frontend provider.

## Internal M2 semantics boundary

The separate `control_flow_v1` module consumes only an exact source-map-bound verified
protocol-v3 snapshot and an entry present in that snapshot,
revalidates the complete deterministic module graph, owns module/callable/lexical names and exact
types, and lowers the frozen M2 semantic subset. It implements `i32` arithmetic and signed
comparisons, Boolean equality, initialized locals, assignment, lexical shadowing, and statically
resolved acyclic direct calls while preserving left-to-right once-only evaluation.

M2 semantic success returns only mandatory-verifier-approved
`zryna_ir::control_flow_v1::VerifiedProgram`; raw M2 IR never leaves the boundary. Canonical `if`
and `while` lowering carries definite mutable state through typed merge and loop-header parameters,
with reachability and all-path return checks. No backend or public CLI selects this profile. See
[M2 straight-line semantics](../../docs/M2_STRAIGHT_LINE_SEMANTICS.md) and
[M2 control-flow semantics](../../docs/M2_CONTROL_FLOW_SEMANTICS.md).
