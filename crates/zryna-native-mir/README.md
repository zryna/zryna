# Zryna native MIR

Concrete native control-flow and value operations lowered from `VerifiedProgram`.

Universal-IR lowering sees only sealed backend-safe function views. The resulting `MirModule` has
private storage and read-only views, and its only constructor lowers a `VerifiedProgram`; callers
cannot inject raw symbols, operations, or value references into the current native backend.

The current straight-line MIR remains a foundation representation. A separate mandatory MIR
verifier must seal control flow, transformed value definitions, ABI types, and resource bounds
before production transformations or additional MIR producers are introduced.
