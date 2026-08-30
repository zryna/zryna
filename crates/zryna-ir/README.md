# Zryna Universal IR

This crate owns the mandatory trust boundary between Zryna semantics and every output backend.
`Program`, `Function`, and `Expr` are untrusted claims. Only `verify(program, sources)` can create
a `VerifiedProgram`, and backends receive immutable `VerifiedFunction` views instead of the raw
program.

## Current universal profile

`UniversalProfile::I32V1` admits only `i32` parameters, results, literals, and signed wrapping
addition. `bool` and `unit` exist as reserved type vocabulary but fail verification until every
backend in a future profile implements their specified representation and behavior.

Raw IR includes `BoolLiteral` so Zryna semantics can represent a valid source-level Boolean before
profile selection. The verifier checks that the literal claims `Type::Bool`, then the `I32V1`
profile rejects that type. This keeps semantic legality separate from universal backend support and
does not allow a Boolean operation to reach a current backend.

Each function is paired by declaration index with a sealed scalar ABI v1 export from `zryna-abi`.
That lower authority owns the bounded logical-name grammar, reserved names, exact and portable
collision checks, typed signature, and deterministic JavaScript, core WebAssembly, and Linux
x86-64 native target mappings. Backends receive those immutable views instead of sanitizing raw
names. The current Universal IR profile still rejects `bool`; specifying its ABI does not enable it
before every active backend implements it.

## Verified expression shape

Every function contains one canonical expression tree:

- the body references an expression in the same arena and has the declared result type;
- every operand is a distinct earlier arena entry;
- every arena entry has exactly one owner, counting the body as the root owner;
- the arena is exact left-to-right postorder, with no shared or orphan entries;
- expression depth is at most 128; and
- every expression span resolves through the exact compilation `SourceMap`.

The verifier and current JavaScript emitter are iterative, so hostile depth cannot consume the
host call stack.

## Resource contract

| Resource | Limit |
| --- | ---: |
| Functions per program | 16,384 |
| Parameters per function | 256 |
| Parameters per program | 262,144 |
| Expressions per function | 16,384 |
| Expressions per program | 262,144 |
| Expression depth | 128 |
| Logical export bytes | 128 |
| Retained diagnostics, including the terminal diagnostic | 256 |

Resource limits are checked before proportional graph verification. Diagnostics are deterministic
and bounded: `ZRYNA-I1001`–`I1008` cover body, type, operation, and graph failures; `I1009`–`I1011`
cover logical exports; `I1201` reports a resource limit; and terminal `I1202` reports diagnostic
budget exhaustion or an internal bounded-construction failure.

Successful verification proves only this documented profile. Native lowering passes explicit raw
claims through the native MIR verifier, which retains the sealed ABI mapping before fixed-target
object codegen. Linux x86-64 object emission is implemented. Driver-owned linking and execution are
separate, capability-checked trust boundaries; neither is implied by `VerifiedProgram`.
