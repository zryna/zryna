# Zryna semantics

Permanent compiler phase boundary from verified provider-neutral syntax to unverified Universal IR.

`SemanticInput::try_new` accepts only a verified protocol-v2 snapshot bound to the exact immutable
`SourceMap` that issued it. It rejects every snapshot containing a provider error, so parse
recovery or unsupported syntax cannot enter name resolution, type checking, or lowering as a
smaller program. Provider warnings remain non-fatal and are preserved by `zryna-driver`.

## First strict source subset

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

The bootstrap provider rejects parenthesized expressions, calls, local declarations, control flow,
and heap-backed expressions before semantics instead of normalizing a smaller program. Multi-file
and module semantics, together with source-connected target emission, remain outside the current
gate; `lower` rejects multiple files. A source path conventionally ends in `.zry`; suffix
enforcement belongs to the future user-facing input and module layer, not this provider-neutral
semantic phase.

Semantic success returns raw `Program`; only `zryna-ir::verify` can turn it into backend-safe
`VerifiedProgram`. `bool` is valid source semantics and lowers to `BoolLiteral`, but the current
`I32V1` verifier deliberately rejects it with `ZRYNA-I1006`. Therefore the current complete
source-to-verified-IR success path is the documented `i32` subset. Enabling `bool` requires one
future universal profile implemented consistently by every active backend.

This crate owns language meaning and must never depend on a replaceable frontend provider.
