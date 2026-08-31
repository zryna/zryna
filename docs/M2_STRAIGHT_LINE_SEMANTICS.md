# M2 straight-line semantics

Status: implemented as an internal compiler boundary. This does not enable the public
`control-flow-v1` profile, a backend, manifest v2, or a CLI command.

## Authority boundary

`zryna-semantics::control_flow_v1` accepts only a verified protocol-v3 snapshot bound to its exact
immutable `SourceMap` and one independently selected entry `FileId`. Provider errors, a
different source-map instance, an unknown entry, an incomplete graph, an import cycle, or an
unreachable extra module prevents semantic success and verified IR exposure.

The semantic phase rebuilds dense module and function tables in normalized path and source
declaration order. Module discovery and semantics use the same provider-neutral
`zryna_source::resolve_explicit_zry_import` grammar, so a path cannot change meaning between graph
closure and name resolution. The driver exposes one internal closure-to-semantics method, but the
semantic crate never depends on the driver or trusts provider-selected symbols.

Semantic success is atomic. Lowering creates raw `ControlFlowV1` only inside the function and
immediately invokes the mandatory IR verifier. The public result contains only
`zryna_ir::control_flow_v1::VerifiedProgram`; callers cannot obtain raw or partially valid IR.

## Implemented source subset

The internal straight-line slice accepts:

- exact `i32` and `bool` parameter, result, and local annotations;
- signed `i32` literals from `-2147483648` through `2147483647` and Boolean literals;
- wrapping `+`, `-`, `*`, and unary `-` on `i32`;
- exact `===` and `!==` on equal `i32` or equal `bool` operands;
- signed `<`, `<=`, `>`, and `>=` on `i32`;
- initialized `const` and `let` bindings, exact-type assignment to `let`, and lexical blocks;
- same-module and named cross-module direct calls with exact signatures; and
- one exact typed return on every straight-line function path.

Parameters are immutable. A root local cannot redeclare a parameter, and one block cannot declare
the same local twice. A nested block may shadow a parameter or outer local. Its initializer is
evaluated before the new name enters scope; assignments update the nearest mutable binding, and
the outer value becomes visible again when a shadowing block exits.

Functions are predeclared in source order, so forward calls are valid. Imported aliases and
same-module functions share one callable namespace, separate from lexical values. An import must
name an explicitly exported dependency function. Only exports in the selected entry module enter
scalar ABI v1; dependency exports remain internal. An entry module with zero exports is valid at
this internal stage because the frozen M2 contract defines no minimum export count.

The complete call graph is acyclic, has at most 65,536 source call sites, and has static depth at
most 128. Multiple distinct call sites from one caller to the same callee are valid and counted
separately. Direct, mutual, and cross-module recursion are rejected before IR verification.

## Evaluation and IR mapping

Initializers, operands, and arguments evaluate left to right exactly once. Lowering consumes the
verified protocol-v3 expression arena iteratively in canonical postorder at each executed
statement root. A reference aliases the current SSA-like `ValueId` and emits no instruction.
Every literal, operator, and call emits exactly one dense instruction result. Parameters allocate
the first value IDs; instruction results follow execution order. Local declarations and
assignments update semantic state rather than creating target-dependent storage.

Every accepted #49 function has one raw entry block and one `Return` terminator. Nested lexical
blocks flatten without reordering evaluation. Instruction spans use the full source expression,
parameter values use parameter-name spans, function spans use declarations, and return terminators
use return-statement spans. The IR verifier rechecks every identity, type, call, span, ABI claim,
budget, and the complete call graph before exposing opaque views.

`if` and `while` syntax is intentionally rejected with `ZRYNA-M2014`; canonical control-flow and
definite-state lowering belongs to Issue #50. No M2 backend accepts this program yet.

## Stable diagnostics

M2 semantic diagnostics are capped at 256 and ordered by normalized portable path bytes, primary
start offset, code, end offset, message, and guidance. The first item beyond the retained budget
adds one terminal `ZRYNA-M2201` and prevents later semantic action.

| Code | Rejection family |
| --- | --- |
| `ZRYNA-M2001` | duplicate module function |
| `ZRYNA-M2002` | missing or unsupported exact type |
| `ZRYNA-M2003` | duplicate parameter or illegal local redeclaration |
| `ZRYNA-M2004` | value absent from lexical scope |
| `ZRYNA-M2005` | unknown or immutable assignment target |
| `ZRYNA-M2006` | local or assignment exact-type mismatch |
| `ZRYNA-M2007` | invalid operator operand types |
| `ZRYNA-M2008` | out-of-range `i32` literal |
| `ZRYNA-M2009` | wrong, missing, or unreachable return behavior |
| `ZRYNA-M2010` | invalid module closure, import, or dependency export |
| `ZRYNA-M2011` | callable namespace collision |
| `ZRYNA-M2012` | unknown callable, arity, or argument-type mismatch |
| `ZRYNA-M2013` | cyclic direct-call graph |
| `ZRYNA-M2014` | control flow reserved for Issue #50 |
| `ZRYNA-M2022` | invalid entry scalar ABI export |
| `ZRYNA-M2201` | deterministic semantic resource exhaustion |

## Verification evidence

Checked-in source/request and exact provider-result fixtures are replayed against the real
TypeScript 6 v3 adapter. They cover multi-file syntax verification, every frozen straight-line
operator, minimum and maximum `i32`, locals, nearest-binding assignment, nested shadow restoration,
forward, same-module and cross-module calls, repeated call sites, and left-to-right nested
arguments. Adversarial fixtures cover every semantic rejection family, callable collisions,
illegal entry exports, module and call cycles with authoritative spans, deterministic rendered
diagnostics, and source-map mismatch. Programmatic boundary tests prove the exact 65,536/+1 call
site, 128/129 static depth, and 256-diagnostic limits. The driver integration test proves that its
final authenticated closure can enter this boundary; independent callers must supply an equally
complete source-map-bound verified snapshot, which semantics revalidates. Existing protocol-v2/M1
tests remain unchanged.
