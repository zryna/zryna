# Zryna native backend boundary

Native backend contracts and an initial textual LLVM IR proof of the MIR boundary.

The current emitter accepts only opaque, read-only `VerifiedMirModule`. Raw modules and operations
are rejected by the Rust API boundary; every accepted module has passed symbol, convention,
signature, value-definition, predecessor, cycle, result, type, and resource verification.

The unchanged textual LLVM output is a deterministic foundation proof. The provisional internal
convention currently uses LLVM's omitted/default convention but is not the final scalar ABI, FFI,
object, target, or linker contract.
