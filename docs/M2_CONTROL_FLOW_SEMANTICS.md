# M2 control-flow semantics

Status: implemented as an internal compiler boundary. This does not enable the public
`control-flow-v1` profile, any backend, manifest v2, or a CLI command.

## Authority and result boundary

The control-flow semantic phase extends the authenticated, source-map-bound protocol-v3 module
snapshot described by [M2 module closure](M2_MODULE_CLOSURE.md). It owns name and exact-type
checking, definite mutable state, canonical CFG construction, and source spans. Provider claims do
not become semantic or IR authority. Success is atomic: the public result contains only
mandatory-verifier-sealed `zryna_ir::control_flow_v1::VerifiedProgram`; raw or partially verified
IR never leaves the phase.

## Accepted control flow

The internal subset accepts lexical blocks, initialized `const` and `let`, assignment, `return`,
`if` with an optional `else`, and `while`. Every condition must have exact `bool` type. The
condition of an `if` is evaluated once; the condition of a `while` is evaluated once on every
visit to its header. An omitted `else` is the exact empty false path.

Evaluation remains left to right and once-only. `break`, `continue`, labeled statements, `for`,
`switch`, exceptions, async control flow, recursion, and implicit truthiness remain outside this
profile.

## Canonical CFG and definite state

Functions, blocks, parameters, values, and edges receive dense deterministic identities in source
and lowering order. An `if` creates explicit true and false successors and, when a continuation is
reachable, one merge block. Only live mutable outer bindings cross that merge as exact-typed block
parameters in stable binding order. A branch that returns emits no merge edge. Branch-local names
do not escape, and shadowed outer bindings are restored after the lexical block.

A `while` lowers to a preheader jump, a non-entry header, body, and exit. Lowering conservatively
admits at most 256 in-scope mutable bindings before liveness pruning; each retained live binding is
a header parameter. The header evaluates the condition and branches to body or exit; a reachable
body fallthrough carries updated values on the backedge. A body return emits no backedge. Mutable
source bindings therefore become SSA-like values and block arguments, never target-dependent
storage claims.

Every reachable function path must return the declared exact type. Statements after a guaranteed
return are rejected as unreachable. For return completeness, `while (true)` is still treated as
potentially falling through: divergence is language behavior, not an implicit trap or proof of a
return. Runtime deadline containment remains a later executable-backend responsibility.

## Diagnostics and limits

- `ZRYNA-M2014` rejects a non-`bool` `if` or `while` condition.
- `ZRYNA-M2004`, `ZRYNA-M2005`, and `ZRYNA-M2006` retain lexical reference, mutability, and
  exact-assignment authority across branches and backedges.
- `ZRYNA-M2009` rejects wrong, missing, or unreachable return behavior.
- `ZRYNA-M2201` rejects deterministic semantic resource exhaustion, including excessive live
  mutable bindings at a merge or loop header.

The independent IR verifier still proves block identity, edge arity and types, dominance,
reachability, reducibility, return completeness, call-graph constraints, source authority, and all
frozen IR budgets before opaque views are created.

## Verification evidence

Focused semantic and IR tests cover true and false branches, omitted `else`, nested control flow,
zero- and multiple-iteration loops, assignment carried through merges and backedges, early returns,
shadow restoration, stable diagnostics, deterministic repeated lowering, exact Boolean conditions,
missing returns, unreachable statements, and resource boundaries. These tests authenticate the
internal compiler component only. Cross-target execution, divergence containment, manifest v2,
the explicit public profile, and website support remain gated by Issues #51 through #57.
