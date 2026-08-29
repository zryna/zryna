# Zryna native backend boundary

Native backend contracts and an initial textual LLVM IR proof of the MIR boundary.

The current emitter accepts only the opaque, read-only `MirModule` produced by lowering a
`VerifiedProgram`. External callers cannot construct or mutate module functions and operations.
Production transformations and additional MIR producers still require a separate native MIR
verifier and explicit target ABI mapping.
