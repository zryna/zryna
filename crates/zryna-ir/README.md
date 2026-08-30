# Zryna Universal IR

This crate owns the mandatory trust boundary between Zryna semantics and every output backend.
`Program`, `Function`, and `Expr` are untrusted claims. Only `verify(program, sources)` can create
a `VerifiedProgram`, and backends receive immutable `VerifiedFunction` views instead of the raw
program.

## Current universal profile

`UniversalProfile::I32V1` admits only `i32` parameters, results, literals, and signed wrapping
addition. `bool` and `unit` exist as reserved type vocabulary but fail verification until every
backend in a future profile implements their specified representation and behavior.

Each logical export is sealed as a `LogicalExportName`. Its spelling is 1–128 ASCII bytes matching
`[A-Za-z_][A-Za-z0-9_]*`, is not an ECMAScript strict/module reserved binding or a selected
prototype-sensitive name, and is never sanitized. Names must be unique both exactly and under
ASCII case folding so later target symbol mapping starts from a portable collision-free identity.
Target ABI spelling remains the responsibility of each backend contract.

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

Successful verification proves only this documented profile. Native lowering converts it to raw
typed value definitions and passes those claims through the native MIR verifier before codegen.
Concrete public calling-convention mapping, object emission, and linking are later trust boundaries
and are not implied by `VerifiedProgram`.
