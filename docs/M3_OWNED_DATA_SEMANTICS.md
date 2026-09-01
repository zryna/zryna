# M3 owned data semantics design contract

Status: implementation in progress for Issue #81. This document freezes the internal semantic and
verified-IR contract for uniquely owned `String` and `Vec<T>` values. The bounded checkpoint below
is implemented, but the complete closure target is not. Nothing here is executable or reachable
from a public compiler profile. No runtime, backend, driver route, CLI selector, manifest profile,
or public aggregate ABI is activated here.

The normative language behavior remains defined by
[`DATA_OWNERSHIP_V1.md`](../spec/language/DATA_OWNERSHIP_V1.md). This document fixes the smaller
implementation boundary that Issue #81 must prove before later target work may consume it.

## Current implementation checkpoint

The implemented semantic producer is deliberately narrower than the complete contract in this
document. Its private owned String/Vec route currently proves:

- String creation from UTF-8 literals, explicit clone, checked concatenation, local moves, return
  with reverse-order cleanup, and replacement of one initialized mutable root-local String;
- canonical Vec construction, explicit clone of exact `Vec<bool>`, `Vec<i32>`, and `Vec<String>`,
  local moves, return, push, checked indexing that yields a Copy element, and replacement of one
  initialized mutable supported exact Vec root;
- private zero-argument producers and one-argument owned identity calls with atomic result,
  storage, transition, and cleanup reservation before argument ownership changes;
- one bounded top-level no-phi `if`/`else` for String and exact Vec functions, using a bool literal
  or Copy bool parameter, canonical entry/then/else/join blocks, empty typed edges, reverse drops
  of branch-local owners, and exact restoration of every incoming owner before the join;
- one bounded terminal owned `if`/`else` for private String and exact Vec results, where both arms
  return one owned-producing expression through a canonical one-parameter join; the join owns the
  selected value exactly once and excludes it from return-site cleanup;
- one bounded top-level no-carried-owner `while` for private String and exact Vec functions, with
  pre-loop declarations, condition evaluation in a canonical loop header, reverse drops of every
  iteration-local owner before the backedge, exact restoration of incoming ownership state on the
  backedge and false exit, and one final return after the loop; the admitted stable-place mutation
  subset replaces one mutable outer String after complete RHS preparation or pushes a Copy element
  into one mutable outer exact Vec without introducing an owned loop-header phi;
- branch-local exact Vec/String/bool/i32 declarations and push into a branch-local Vec, while a
  push or replacement of an incoming Vec is rejected before its right-hand side is evaluated;
- bounded construction, whole-value local moves, return, and reverse-order survivor cleanup for
  parameter-free private straight-line owned Struct and FixedArray graphs with bool, i32, or String
  leaves, and owned Enums with payloadless or supported Copy, String, Struct, or FixedArray payloads;
- explicit structural clone of supported non-Copy Struct, FixedArray, and root Enum values with
  String leaves, retaining the source and creating one distinct result owner; recursive String-clone
  failure derives its exact fallible-leaf count and root-enum active variant from sealed authorities,
  reverse-drops only the initialized result prefix, and then cleans pre-existing roots;
- mutable whole-root assignment for the same supported Struct, FixedArray, and root Enum graphs;
  the complete right-hand side is prepared while the old destination remains live, direct
  self-consumption is rejected, and `ReplacePlace` commits the prepared owner with the sealed
  recursive old-value drop shape;
- canonical static struct-field and constant fixed-array projection reads, with Copy leaves retained
  and exact String leaves moved once while the enclosing root keeps its masked cleanup obligation;
- `InitializePlace`, `MoveFromPlace`, and prepare-then-commit `ReplacePlace` lowering, with
  private String use-after-move rejected as `ZRYNA-M3011`, aggregate/enum moved-owner violations as
  `ZRYNA-M3014`, unresolved binding names as `ZRYNA-M3002`, and excluded private String shapes as
  `ZRYNA-M3012`;
- one-plan/one-site cleanup roles, one exact non-Copy owner while Copy storage remains addressable
  but owner-excluded, and cumulative String-literal preflight at 8 MiB; and
- an internal bounded fault/drop-trace oracle for every admitted String construction/clone/concat
  and Vec allocation/reserve failure status plus checked Vec bounds failure. It consumes the
  authenticated runtime-status disposition and trap identity, proves pre-commit operand retention,
  excludes the uncommitted result, and pins exact reverse cleanup, deterministic replay, and
  exact/first-extra/overflow event limits; and
- a sealed semantic `VerifiedProgram` retaining mandatory-verifier-approved IR together with the
  exact verified ownership-runtime ABI authority.

General structural Vec clone beyond String elements, nested aggregate clone graphs containing Enum,
Vec, Shared, or Weak values, aggregate-subobject and enum-payload moves, dynamic or Vec-element
projections, projected clone and general non-String projected assignment, whole-partial-owner transfer, general owned phi joins,
owned loop-carried phi joins, repeated or nested branches or loops, general lexical scope exits,
runtime/backend lowering, CLI
selection, and public owned values remain unavailable. Owned String/Vec signatures remain bounded
to zero arguments or one exact owned/bool argument. Their no-phi branch must leave incoming owners
unchanged, while the terminal owned join accepts only one owned-producing return expression per
arm. The bounded loop preserves the exact incoming owner stack. Its stable-place String replacement
and Copy-element Vec push retain the same outer place identity across the backedge; all other
incoming owners remain unchanged. Vec replacement, `Vec<String>` push, `break`, `continue`, body
returns, and effects after the loop remain excluded. The aggregate route remains parameter-free,
private, and straight-line. Its partial-move subset is limited to exact String leaves reached
through static StructField or FixedArrayConstant paths. Nested enums, Vec members, recursive graphs,
aggregate-subobject moves, and aggregate match
are also excluded. Those are closure work, not properties of the current checkpoint.

## Issue #81 implementation ledger

| Dependency-ordered slice | State | Current proof boundary |
| --- | --- | --- |
| verified DataOwnershipV1 IR and runtime ABI authority | complete | mandatory sealed verification and exact declaration authority |
| String/Vec construction, moves, replacement, calls, and reverse return cleanup | complete | bounded private functions with exact resource ledgers |
| owned Struct/FixedArray/Enum construction and whole-value moves | complete | bounded private straight-line graphs |
| no-phi branches and terminal owned block-parameter join | complete | canonical bounded String/exact-Vec CFGs |
| no-carried-owner loop/backedge cleanup | complete | one top-level loop with exact incoming-state restoration |
| stable-place loop mutation | complete | String replacement and Copy-element Vec push retain one exact outer place across repeated execution |
| general loop-carried values and scope exits | pending | owned header phi, Vec replacement/owned-element push, and arbitrary exits remain excluded |
| exact `Vec<bool>`/`Vec<i32>`/`Vec<String>` clone | complete | distinct result owner, retained source, authenticated allocation and element-clone failures, prefix-safe reverse cleanup, and exact resource rollback |
| supported String-bearing aggregate clone | complete | distinct result owner, retained source, sealed layout/variant-derived fallible leaves, authenticated prefix-safe recursive failure cleanup, and atomic resource rollback |
| supported whole-root owned aggregate assignment | complete | prepare-before-commit replacement with direct self-consumption rejection, recursive old-value drop authority, and exact transition reservation |
| static owned projection reads and String-leaf moves | complete | canonical StructField/FixedArrayConstant places, disjoint leaf moves, root-relative cleanup masks, and precise repeat/overlap rejection |
| general structural Vec clone, nested aggregate clone, aggregate/enum subobject moves, and non-String projected assignment | pending | recursive Vec/Enum/Shared/Weak clone, whole-partial-owner transfer, dynamic projection, and broader replacement capability |
| controlled allocation/capacity/bounds/UTF-8 fault closure | in progress | authenticated internal fault/drop traces, including Vec<String> and aggregate-clone partial initialization, are complete for admitted operations; executable target fault injection remains pending |
| full Issue #81 limits, regressions, cross-platform CI, and merge | pending | complete preflight plus Linux and Windows required checks |

This ledger records implementation evidence, not public language availability. Every row remains
inside the private internal gate until the complete issue closes and later milestones authorize a
runtime, backend, driver, and CLI profile.

## Authority boundary

The owned-data semantic gate extends the internal
`zryna-semantics::data_ownership_v1` path. It consumes the same authenticated protocol-v4 syntax,
exact `SourceMap`, expected entry file, independently derived type graph, and sealed `Linear32V1`
and `LinuxX8664V1` layouts used by the Copy aggregate slice. Semantics may construct raw
`DataOwnershipV1`, but success returns only the result of mandatory independent IR verification:

```text
authenticated protocol-v4 syntax + exact SourceMap + expected entry
    -> semantic type, scope, ownership, and CFG lowering
    -> untrusted DataOwnershipV1
    -> mandatory zryna-ir verification
    -> sealed semantic program retaining verified IR + exact verified runtime ABI authority
```

Provider symbols, inferred types, provider node identities, target storage, runtime handles, and
raw ownership claims never become authority. Layout still owns nominal structure, field and
variant order, fixed-array length, element and payload types, `Copy`/drop classification, and
target layout. Semantics owns source names, lexical scopes, expression evaluation order, exact
types, place selection, move intent, and deterministic CFG construction. The IR verifier owns the
final proof of unique ownership, edge transfer, path state, cleanup completeness, and drop order.

`RuntimeContractIdentity::OwnershipRuntimeV1` remains an identity claim in verified IR. The
separate ownership-runtime ABI component authenticates declarations and pure transitions, but
does not execute them. The semantic result retains that exact ABI authority beside verified IR and
the same two layout authorities; raw declarations remain unavailable. Issue #81 must not make any
of these components a runtime implementation.

## One ownership authority: places

Places are the only authority for non-`Copy` ownership. There is no independent SSA ownership
lattice whose answer can disagree with place state.

- Every non-`Copy` function parameter has exactly one root `Parameter` place.
- Every non-`Copy` block parameter and instruction result has exactly one root
  `Temporary(ValueId)` place.
- A local declaration has exactly one root `Local` place.
- A non-`Copy` `ValueId` resolves to exactly one owner place for its complete live interval.
- A `Copy` value may have addressable parameter, local, or temporary storage, but it is excluded
  from the non-`Copy` owner map and never enters a cleanup obligation.
- Root identities are unique. Two places cannot claim the same parameter, local, or temporary.
- A value cannot be ownerless, have two owner aliases, or be consumed without transferring its
  one owner.

Static struct-field, active-enum-payload, and constant fixed-array projections refine one root;
they never create an independent second owner. A vector element and a dynamically indexed array
element remain part of the complete container place. Parent and child state must agree, and two
overlapping projections cannot be moved independently.

The verifier derives `ValueId -> PlaceId` from canonical roots and resolved types before ownership
dataflow. Raw IR does not carry a competing available/consumed transition arena. Verified views
may expose the sealed value-owner association, but cannot recover or mutate the raw program.

## Ownership state and pending-drop stack

Each reachable program point has a normalized ownership state for every relevant place and one
ordered pending-drop stack. The state inventory for this Issue #81 slice is `uninitialized`,
`initialized`, `partially initialized`, `partially moved`, `moved`, and `dropped`. Borrowed states
are reserved for Issue #82 and are rejected by the owned-data semantic gate.

The pending-drop stack contains each currently live non-`Copy` root exactly once in successful
completion order. It is the authority for inter-root cleanup ordering; source declaration order,
place ID, block ID, target address, and instruction-number arithmetic are not substitutes.

- Completing initialization or an owner-producing operation pushes its new root.
- Dropping, moving out of the function, or consuming an owner removes that root.
- Transferring ownership renames the root in its existing stack slot and preserves relative order.
- Constructing an aggregate removes its owned operand roots and pushes the completed aggregate.
- Replacement fully evaluates the right-hand side first. On commit, it removes the old destination
  and renames the prepared right-hand-side owner at its completion position to the destination.
- Successful `VecPush` removes the prepared element owner while retaining the vector root in its
  existing stack position.

Initialization-site numbers are not drop-order authority. In particular, two branch arms may
initialize different temporary roots and transfer either one to the same successor block
parameter. After edge transfer, both paths may have the same normalized state and stack even
though their source instruction positions differ.

At a reachable merge, all incoming edges must agree on the normalized stack and on the state,
active variant, initialized prefix, and moved mask of every live-in place. A branch-local place
that has been dropped and has no successor use normalizes to `dead`; it does not invalidate a
later merge. If the place remains a pending obligation or is used after the merge, all incoming
paths must agree exactly.

A loop backedge must reproduce the loop header's normalized live-place state and pending-drop
stack. Loop-body locals are dropped before the backedge or loop exit. The compiler does not add an
implicit clone, conditional drop, or path-dependent repair to make unequal states join.

## CFG edge ownership transfer

Raw CFG edges continue to carry typed value arguments. The verifier combines each non-`Copy` edge
argument with the unique owner of the corresponding target block parameter and derives a sealed
edge transfer:

```text
source owner place -> target block-parameter owner place : exact verified type
```

The transfer occurs on that edge, renames the pending-drop stack entry, leaves the source moved,
and initializes the target owner. It is rejected if a source is unavailable, consumed twice on
one edge, sent to two owned parameters, mismatched in type, aliased with another target, or used
after transfer. Copy edge arguments remain ordinary value flow and create no ownership effect.

Branch, enum-match, and loop successors are verified independently before a join. Edge transfer
facts are derived authority exposed through opaque verified edge views; raw code cannot supply a
second transition list or forge a verified transfer.

## Return, call, and scope exits

A non-`Copy` return transfers its owner out of the function before exit cleanup is derived. The
returned root must therefore be absent from that function's cleanup plan. Every other live root
must still appear exactly once in reverse pending-stack order.

By-value call arguments evaluate left to right. At call entry their owners transfer to the callee,
whose parameter places then own them. If the callee produces a normal result, a non-`Copy` result
initializes one caller temporary. If the callee produces a controlled trap, the callee cleans its
parameters and locals; the caller cleans only owners it retained across the call. Neither side may
drop an argument twice or leave ownership between the two frames.

Lexical scope exit is explicit in IR. Semantics inserts `DropPlace` operations for live
scope-local roots in reverse pending-stack order before a normal fallthrough, branch edge, loop
backedge, or loop exit. A return or controlled trap instead uses its site-bound cleanup plan. A
place already moved, transferred, or dropped has no action.

## Site-bound cleanup authority

A raw cleanup plan is only an ordered claim. Verification binds it to one exact program point and
one closed role:

- `PrepareFailure`, for one potentially failing checked prepare operation;
- `VecCloneElementFailure`, for the separately authenticated failure of one exact String element
  clone after a runtime-recorded destination prefix has initialized;
- `AggregateCloneElementFailure`, for the separately authenticated failure of one exact String leaf
  after a verifier-derived aggregate destination prefix has initialized;
- `CallTrap`, after by-value arguments have transferred into a called function;
- `Return`, after the returned-owner transfer; or
- `ControlledTrap`, before reporting one exact trap identity.

The verified site identity includes the containing function, block, instruction or terminator
position, and role. Each cleanup-bearing site names exactly one plan, and one raw plan belongs to
exactly one site. Even byte-identical action lists at different sites produce distinct verified
site authorities. Missing, orphan, reused, cross-function, or role-mismatched plans fail closed.

For its exact site, the verifier independently snapshots the normalized state and pending-drop
stack, derives the required actions in reverse order, and compares the complete raw list. Missing,
extra, duplicate, reordered, inactive, already moved, already dropped, or wrong-root actions are
invalid. Success exposes site-specific derived actions, not raw cleanup recovery.

An explicit `DropPlace` instruction is a deterministic drop point for one root, not a cleanup-plan
site. Its verified instruction view exposes the exact pre-drop root state and projection mask so a
consumer can combine them with the program's retained sealed layouts.

## Prepare, commit, and controlled failure

Every fallible owned-data operation has a strict prepare/commit boundary:

1. evaluate source operands left to right, producing any temporary owners;
2. prepare checked size, capacity, UTF-8, allocation, or clone work without changing logical
   ownership;
3. on failure, run the site's cleanup against the pre-commit state and retain the original trap
   identity; or
4. on success, perform one infallible logical commit and update owner places and the pending stack.

No partial public ownership state escapes a failed prepare. Runtime status is explicit; a host
exception, JavaScript throw, WebAssembly engine trap, native signal, abort, or allocation side
effect is not a Zryna controlled trap.

The operation-specific consequences are:

- String construction validates decoded or external bytes before creating an owner.
- String clone and concatenation create a result only after successful allocation and checked
  length/capacity arithmetic. Their sources remain owned on failure.
- Vec construction evaluates all elements first. Allocation failure leaves all element
  temporaries owned; commit moves them into one initialized Vec.
- Exact `Vec<bool>` and `Vec<i32>` clone reserves the result owner, storage, ownership transition,
  and cleanup before emission. Allocation failure retains the source and excludes the result;
  success creates one distinct temporary result owner.
- Exact `Vec<String>` clone additionally reserves a distinct per-element failure plan and its action
  sum before emission. Elements clone in ascending order; failure reverse-drops only the completed
  destination prefix, releases its storage, then cleans pre-existing roots while retaining the
  source and excluding the uncommitted result.
- General structural Vec clone beyond String elements remains a closure target with the same
  prefix-safe rule derived recursively from the element clone capability.
- Supported String-bearing Struct, FixedArray, and root Enum clone reserves a distinct result owner
  and a separate aggregate-element failure plan before emission. The verifier derives the exact
  fallible String-leaf count from retained Linear32 layout and derives a root Enum's active variant
  from source ownership state. Failure reverse-drops only the completed destination prefix and then
  pre-existing roots while retaining the source and excluding the uncommitted result.
- `VecPush` evaluates its value first. Reserve failure retains both vector and argument; commit
  moves the argument into the new final element.
- Checked fixed-array or Vec indexing performs no ownership transfer on bounds failure.
- Assignment evaluates and prepares the replacement before modifying the destination. Failure
  leaves the old destination initialized; successful commit drops the old value and installs the
  prepared owner.

Direct calls are not allocator-style prepare operations. By-value arguments transfer when the
callee is entered, and callee trap cleanup owns those parameters as described above.
`ReplacePlace` is likewise the infallible commit after its right-hand side has been prepared: it
carries no prepare-failure plan and exposes the derived drop shape of the old destination. Plain
struct, enum, and fixed-array construction commits already prepared operands without a failure
plan. Allocation-bearing Vec construction and clone steps retain their exact prepare-failure sites.
The current semantic checkpoint emits `ReplacePlace` for private root-local String, supported exact
Vec roots, supported String-bearing Struct, FixedArray, and root Enum values, and mutable available
static StructField or FixedArrayConstant String leaves. Other projected destinations, owned calls,
and CFG replacement remain outside that checkpoint. It resolves
canonical static StructField and FixedArrayConstant source places for Copy reads and exact
String-leaf moves, preserving the enclosing owner's masked pending cleanup.

The verified IR prerequisite for projected replacement is already sealed: a static projection
commit exposes the old subobject's exact pre-state recursive drop action and transplants only the
prepared source subtree's state and active enum variants. The enclosing owner remains pending and
sibling masks are unchanged. The semantic producer supplies canonical static projection resolution,
overlap rejection, and projection-aware owner-state tracking for Copy reads, String-leaf moves, and
prepare-before-commit String-leaf replacement. Replacement preparation retains the enclosing root;
commit drops only the exact old leaf and leaves sibling masks unchanged. It does not yet replace
non-String projections or transfer a whole partially moved aggregate.

The retained runtime ABI authority also closes contextual transition evidence: atomic failure is
validated against one exact `LogicalOperation`, and Vec allocation/reserve validation consumes a
sealed verified element layout whose positive stride is used for checked `capacity * stride` byte
amplification. A context-free failure claim or count-only Vec claim is not accepted as proof.

The current test-only fault oracle additionally consumes the ABI authority's sealed status
declarations. `ALLOCATION`, `CAPACITY`, `REFCOUNT`, and `UTF8` retain their exact controlled-trap
identities, `ABI_VIOLATION` remains a host failure, and `EXPIRED` remains a non-trapping branch
outcome. The oracle rejects an operation/status mismatch, a success status, an unauthenticated
disposition, a missing prepare-failure cleanup site, a cleanup that omits a retained owned operand,
or one that includes the uncommitted result. Bounds failure is modeled separately as the verified
IR's `BoundsV1` trap rather than being relabeled as a runtime status. This is compiler evidence over
sealed declarations and verified IR; it does not inject a failure into an allocator or execute a
target runtime.

For exact `Vec<String>` clone, allocation failure authenticates `VecAllocate`, while an element
failure separately authenticates `StringClone`. The modeled source length must be within the sealed
Vec element bound, and the completed prefix must be strictly shorter than that source length before
trace allocation. The oracle then emits exactly the completed indices in reverse order, followed by
storage release and the pre-existing owner cleanup; zero, middle, last-valid, first-extra, and
arithmetic/event-limit boundaries are deterministic.

For supported aggregate clone, element failure separately authenticates `StringClone` and requires
the exact `AggregateCloneElementFailure` site. The completed prefix must be strictly below the
fallible String-leaf count derived from sealed layout and the authenticated active root-enum
variant. The oracle emits the completed String leaves in reverse structural order, then the
pre-existing root cleanup; caller-supplied leaf counts or enum variants are not accepted.

Cleanup and release are infallible logical effects, do not allocate, and cannot replace the
original trap. A release-contract violation is an internal runtime violation for later executable
work, not a source-catchable second trap.

## Recursive drop shapes

Lowering must not infer recursive cleanup from a raw root ID. Each verified derived action exposes
the site's proven root, initialized/moved projection mask, and active variant. The verified program
retains the matching sealed layouts and runtime ABI; together these authorities derive one closed
logical drop traversal:

- `String`: release the one initialized String allocation;
- `Struct`: drop initialized, non-moved fields in reverse declaration order;
- `Enum`: drop only the exact active payload, then the container if applicable;
- `FixedArray`: drop the initialized prefix from highest index to zero, excluding moved elements;
- `Vec`: read the verified runtime logical length, drop elements from `length - 1` to zero, then
  release vector storage; and
- nested aggregate: recurse by the same sealed shape and layout rules.

The derived traversal therefore binds the root's exact verified type, active variant where
applicable, initialized prefix, moved subobject mask, element policy, and final String/Vec storage
release. Backends must consume the verified state plus retained layout/runtime authorities and must
not reconstruct those facts from raw transitions.

Partial struct and fixed-array construction is prefix-only in deterministic declaration or index
order. Holes, duplicate completion, out-of-order completion, an inactive enum payload, or a moved
child still listed for drop are verifier errors. Vec elements are not represented as one static
place per possible index; runtime logical length is the initialized prefix authority. This is
required because vector length can exceed the per-function place budget.

Moving a whole aggregate transfers its complete partial state and pending obligation. Moving an
allowed static projection removes only that subobject from later recursive cleanup while retaining
the parent root's remaining obligation. Overlapping, dynamic, or vector-element partial moves are
not admitted in this slice. The current semantic producer implements the latter rule only for
String leaves under canonical static Struct/FixedArray paths; whole partial-owner transfer and
aggregate/enum subobject moves remain closure work even though verified IR can represent their
cleanup masks.

Mutable available String leaves under those same canonical paths also admit assignment. The right
hand side is fully prepared before mutation, failure cleanup retains the enclosing root, and the
infallible `ReplacePlace` commit drops only the exact old leaf. Consuming the destination root while
preparing its replacement, assigning through an immutable or moved projection, and reinitializing
an already moved leaf remain rejected.

## Issue #81 closure target

The complete Issue #81 target admits uniquely owned `String`, `Vec<T>`, and structs, enums, and
fixed arrays containing them when their complete type graph is otherwise admitted. The following
inventory is the closure target, not a claim about the narrower current checkpoint:

- UTF-8 String literals, explicit String clone, checked concatenation, moves, assignment, internal
  by-value parameters and results, and deterministic release;
- Vec construction, explicit structural clone when `T: Clone`, push, checked indexing that returns
  `Copy` elements, moves, assignment, internal by-value parameters and results, and deterministic
  element/storage release;
- non-`Copy` aggregate construction, active enum payloads, prefix-safe aggregate clone and cleanup,
  statically resolved place projections, and fixed-array checked indexing;
- local declarations, mutable assignment, internal direct calls, structured blocks, `if`/`else`,
  `while`, exhaustive enum match, return, and controlled bounds/allocation/capacity/UTF-8 traps; and
- existing scalar and Copy aggregate behavior without an ownership obligation.

There is no implicit clone at assignment, call, return, branch, loop, construction, or push.
`clone(value)` uses the protocol-v4 reserved clone expression. `concat(left, right)` is resolved by
semantics from the exact ordinary-call callee and arity; binary `+` remains numeric addition.
String indexing, slicing, normalization, locale behavior, capacity observation, and implicit
coercion remain unavailable. Vec pop, insert, remove, slice, iterator, capacity observation,
element references, and non-`Copy` indexing results remain unavailable.

Issue #81 does not admit `Shared<T>`, `Weak<T>`, borrow expressions, escaping references, interior
mutation, user-defined destructors, exceptions, finalizers, threads, host reentrancy, garbage
collection, public owned parameters or results, or target-visible pointers and handles. Borrowing
is Issue #82; Shared and Weak ownership are Issue #83.

## Diagnostics and deterministic failure

Protocol-v4, layout, runtime-declaration, and verified-IR failures retain their owning diagnostic
families. Semantics must not relabel a later verifier failure as semantic success. Every diagnostic
uses the narrowest authoritative source span available and sorts deterministically before bounded
retention.

The Issue #81 semantic allocation is:

| Code | Source-semantic rejection |
| --- | --- |
| `ZRYNA-M3002` | unresolved, wrong-case, duplicate, or colliding module-local binding name |
| `ZRYNA-M3011` | unavailable or already moved binding in the private String route |
| `ZRYNA-M3012` | invalid String construction, clone, concatenation, UTF-8, or excluded String operation |
| `ZRYNA-M3013` | invalid Vec construction, clone, push, index, element type, length, or capacity operation; invalid aggregate assignment target shape |
| `ZRYNA-M3014` | unavailable, duplicate, or already moved aggregate/enum owner; invalid initialization, assignment, or replacement |
| `ZRYNA-M3015` | incompatible branch join, loop-carried state, scope exit, or return ownership |
| `ZRYNA-M3016` | ownership-bearing operation deliberately outside the Issue #81 slice |
| `ZRYNA-M3201` | semantic amplification exceeds an inherited verified-IR or layout budget |
| `ZRYNA-M3202` | terminal semantic diagnostic-budget exhaustion |

Independent IR proof continues to use `ZRYNA-I3010` for invalid ownership transitions,
`ZRYNA-I3012` for cleanup/site/action mismatch, `ZRYNA-I3013` for invalid partial aggregate state,
`ZRYNA-I3014` for trap or runtime-contract identity mismatch, `ZRYNA-I3201` for resource exhaustion,
and `ZRYNA-I3202` for terminal bounded construction failure. Source-valid but forged raw IR must be
rejected by the IR family even when semantics would never emit it.

Failure returns no partial verified program, layout authority, cleanup plan, runtime capability, or
artifact. Diagnostics are capped at 256 including the terminal diagnostic.

## Resource closure

All protocol-v4, M2 CFG, layout, DataOwnershipV1, and ownership-runtime declaration limits continue
to be independently enforced by their owning boundaries. Issue #81 does not duplicate
layout-owned type, field, variant, fixed-array-length, or dependency-edge counters.

Owned-data semantic preflight must use checked aggregate arithmetic before proportional CFG,
ownership-state, or cleanup construction. It must prove that lowering cannot exceed:

| Derived resource | Maximum |
| --- | ---: |
| String literal UTF-8 bytes per program | 8 MiB |
| Ownership places per function | 65,536 |
| Ownership state transitions per function | 262,144 |
| Cleanup sites/plans per function | 65,536 |
| Derived cleanup/drop actions per function | 262,144 |
| Retained M3 diagnostics, including terminal diagnostic | 256 |

Existing DataOwnershipV1 module, function, parameter, block, value, CFG-edge, call-edge,
call-depth, loop-depth, nominal, and aggregate-operand budgets also bound this lowering. Drop-action
accounting is the checked sum across all sites, not merely the length of distinct or shared plan
bodies. Edge-transfer work is included in the existing checked edge, block-parameter, value, place,
and ownership-transition totals. Exact-limit and first-extra cases are required for every newly
enforced counter.

Runtime String and Vec lengths, capacities, element byte sizes, and allocation sizes remain bound
by the sealed ownership-runtime ABI rules and checked target layouts. Semantics proves representable
static amplification; it does not allocate memory or claim an observed runtime length.

## Required verification lanes

The implementation edit loop must retain a seconds-scale focused lane. `pnpm m3:owned:quick` must
cover ordinary owned-data semantic and IR tests, source-faithful String/Vec fixtures, deterministic
diagnostics, and efficient negative boundaries. Its explicit Rust doc-test commands cover the
verified-view opacity compile-fail examples. Proportional exact
limit cases may be individually named and skipped only by that quick command.

The full repository `pnpm preflight` gate must run every Issue #81 unit, integration, compile-fail,
fault-model, exact-limit, and first-extra test, including proportional cases omitted from the quick
lane, plus explicit ownership IR and semantic doc-tests. The merge proof also requires the existing
locked formatting, strict Clippy, workspace tests, architecture validation, documentation contract tests, and unchanged M1/M2
regressions. A fast lane is never evidence that ignored or skipped full-boundary tests pass.

Focused acceptance evidence must include at least:

- return transfer before cleanup and reverse-order cleanup of remaining locals;
- alternative branch owners transferred into one block parameter and dropped exactly once;
- rejected unequal branch stacks and leaked or reordered loop-backedge obligations;
- explicit scope-exit drops on fallthrough, branch, loop backedge, and loop exit;
- String and Vec allocation failure with pre-commit operands retained;
- exact `Vec<String>` element-clone failure at zero, middle, and last-valid completed prefixes, plus
  rejection of the first impossible prefix before trace allocation;
- supported aggregate element-clone failure at zero, middle, and last-valid completed prefixes,
  including active root-enum selection and first-impossible-prefix rejection before trace allocation;
- `VecPush` failure retaining both vector and argument, followed by exact cleanup;
- partial aggregate, active enum, fixed-array prefix, Vec length, and moved-subobject drop shapes;
- use-after-move, double consumption, duplicate root aliases, missing/extra/reordered cleanup,
  wrong-site plan reuse, and returned-owner double-drop rejection;
- stable source locations and repeated deterministic diagnostic order; and
- exact and first-extra evidence for every Issue #81-owned resource counter.

Fault models and test-only drop traces may observe logical order and failure atomicity. They are
not production interpreters, allocators, runtimes, backends, or public execution paths.

## Deliberately unavailable

Implementation of this document is still in progress. The present repository must not advertise
an executable owned-data profile based on this design alone.

No JavaScript helper, WebAssembly import, native symbol body, allocator, runtime object, linked
artifact, backend lowering, driver dispatch, CLI flag, manifest selector, host invocation, or
public conformance profile is authorized. Scalar ABI v1 remains byte-for-byte unchanged and entry
module exports remain scalar-only. M1 and explicit M2 remain the only public compiler profiles.

Later runtime and backend issues must consume opaque verified cleanup sites, edge transfers,
recursive drop shapes, both sealed layouts, and the separately verified ownership-runtime ABI.
They may not infer ownership from source, replay raw transition claims, omit controlled-trap
cleanup, use garbage collection as a substitute, or expose target storage as a Zryna value.
