# Zryna native MIR

Typed straight-line native values lowered from `VerifiedProgram` or supplied as explicit raw
compiler claims. This root API is the complete implemented M1 profile.

`raw::Module` and its nested raw types are never backend-authoritative. `verify` consumes those
claims and is the only constructor of `VerifiedMirModule`; the verified wrapper exposes only
immutable function/value/operation views and cannot be recovered as raw or mutated. `lower` sees
only sealed Universal IR views, builds the same raw shape, and passes it through that verifier.

The current verified profile proves:

- bounded symbols matching `[A-Za-z_][A-Za-z0-9_]*`, with exact and ASCII-case-folded uniqueness;
- one fixed, non-variadic scalar ABI v1 Linux x86-64 System V convention;
- `i32` parameter, result, definition, literal, and wrapping-add types;
- explicit dense value IDs defined exactly once in canonical slot order;
- existing same-function operands that strictly precede each use, plus iterative cycle rejection;
- an existing result whose type matches the signature; and
- fixed per-function, module, symbol, and diagnostic budgets.

MIR is SSA-like rather than a Universal IR tree: repeated uses, `add(value, value)`, and bounded dead
values are valid. Raw diagnostics are global and identify only bounded function/value ordinals.

The verifier also constructs and retains the authoritative scalar ABI v1 module. Verified function
views expose only its exact Linux symbol and public convention, so object codegen cannot create a
competing name mapping. The current straight-line MIR remains a foundation representation.

The separately versioned [M2 native MIR profile](../../docs/M2_NATIVE_MIR.md) implements an internal
raw-to-verified block, call, symbol, Boolean, and terminator boundary without changing these root
types. Its lowering accepts only sealed Universal `ControlFlowV1`, maps every identity and operation
one-for-one, and independently reseals the complete program. M2 object emission, public Boolean
wrappers, linking, execution, FFI, and product linking remain later gates.
