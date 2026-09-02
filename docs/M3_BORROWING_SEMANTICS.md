# M3 bounded borrowing implementation contract

Status: Issues #113, #114, #115, and #117 complete. The internal semantic producer admits bounded
shared and exclusive Copy-root source shapes, one shared-from-shared reborrow, and one canonical
conditional whose arm-local lexical authorities are completely discharged before every edge.
Projected, call, loop, and owned-root borrowing remain dependency-ordered later checkpoints.

This document freezes the dependency-ordered implementation boundary for Issue #82. It refines the
normative borrowing rules in [`DATA_OWNERSHIP_V1.md`](../spec/language/DATA_OWNERSHIP_V1.md) without
adding a language feature, opcode, runtime representation, backend path, driver route, CLI selector,
or public ABI.

## Permanent boundary

Borrow syntax remains untrusted protocol-v4 data. Zryna semantics alone may resolve an admitted
borrow expression to one exact place and access mode. It emits raw `DataOwnershipV1`; the mandatory
IR verifier independently proves the authority before constructing opaque verified views.

```text
verified protocol-v4 syntax
    ↓ future dependency-ordered semantic slices
raw DataOwnershipV1 borrow claims
    ↓ mandatory existing verifier
opaque verified borrow authority
```

A borrow is compile-time authority, not an owned value. It has no address, storage layout, clone,
drop action, runtime lifetime token, scalar ABI carrier, or public export. Backends may erase a
borrow only after consuming the final opaque verified program in later target issues.

## Frozen v1 model

- `Shared` grants read-only access to one exact place. Overlapping shared borrows may coexist.
- `Exclusive` grants read/write access and conflicts with every overlapping shared or exclusive
  borrow.
- Parent and descendant places overlap. Distinct struct fields and distinct constant fixed-array
  elements are disjoint. Dynamic indices and Vec elements conservatively use the container root.
- The owner remains initialized while borrowed, but overlapping move, drop, replacement, mutable
  container operation, or other exclusive owner use is rejected. An exclusive borrow also blocks
  overlapping owner reads.
- A lexical borrow starts only after its referent is available, has one dense function-local
  `BorrowId`, and ends exactly once before the containing control-flow edge.
- A function borrow parameter is active on entry, cannot be ended by the callee, and must perform a
  `BorrowRead`, `BorrowWrite`, or exact direct-call borrow argument use. Unused signature metadata
  is invalid.
- `BorrowRead` and `BorrowWrite` currently carry only `Copy` referents. Owned values cannot be
  manufactured or transferred through a borrow in this slice.
- A direct call may pass an active lexical or parameter authority only to the exact referent type
  and access mode. Repeating or overlapping exclusive authority in one call is invalid.
- Lexical authority cannot cross `Jump`, `Branch`, `EnumMatch`, `WeakUpgradeBranch`, loop,
  `Return`, or `Trap`. The same statically dense borrow site may execute again on a later loop
  iteration only after the prior dynamic instance ended.

There is no lifetime inference, stored or returned reference, implicit reborrow, raw pointer,
unsafe boundary, FFI, thread, synchronization, or garbage collector in this contract.

## Existing verified-IR authority

Issue #78 already supplied the raw and verified vocabulary that Issue #82 must reuse:

| Authority | Existing form | Mandatory verifier evidence |
| --- | --- | --- |
| function input authority | `BorrowParameter` | dense identity, sealed referent type, access mode, real use |
| lexical creation | `BeginBorrow(BorrowDefinition)` | dense identity, initialized place, overlap rules |
| shared/exclusive read | `BorrowRead` | active authority and exact Copy result type |
| exclusive mutation | `BorrowWrite` | active exclusive authority and exact Copy operand type |
| lexical end | `EndBorrow` | active non-parameter authority ended exactly once |
| call transfer | `CallArgument::Borrow` | active authority, exact callee signature, no repeated/overlapping exclusive arguments |
| edge closure | ownership-flow verification | no active lexical authority at any terminator |
| owner exclusion | ownership-flow verification | overlapping owner reads/writes/moves/drops obey access mode |

No child of #82 may add a competing borrow state machine or trust source claims after this verifier.
The existing `ZRYNA-I3011` family owns dense, inactive, conflicting, duplicate, unused-parameter,
and escaping authority failures. Structural operand/type failures retain `ZRYNA-I3005`, call
signature failures retain `ZRYNA-I3009`, and forbidden owner operations retain `ZRYNA-I3010`.

## Dependency ledger

The parent closes only through this graph:

```text
#113 -> #114 -> #115 -> {#116, #117, #119, #120, #121} -> #122
```

| Issue | Implementation slice | Required boundary |
| ---: | --- | --- |
| #113 | contract and IR prerequisite audit | this document, hostile raw-IR evidence, no semantic lowering |
| #114 | straight-line shared root borrows | deterministic scope, Copy reads, owner-read compatibility |
| #115 | exclusive Copy borrows and bounded reborrowing | mutation, conflicts, exact lexical restoration |
| #116 | shared reads of owned roots | no owned transfer, clone, stored reference, or cleanup authority |
| #117 | conditional edges | every arm ends authority; no borrow-carrying block parameter |
| #119 | bounded internal calls | exact parameter modes; callee cannot retain, return, or end caller authority |
| #120 | projected disjointness | static siblings may coexist; parent/child and conservative dynamic overlap fail closed |
| #121 | loop edges | header/backedge state equality and per-iteration lexical end |
| #122 | closure, limits, regressions, and documentation | final diagnostic, exact/+1, Linux/Windows, M0/M1/M2/non-borrow M3 evidence |

The five slices after #115 are independent. They must not silently broaden one another. #122 owns
the aggregate closure claim; no earlier child marks Issue #82 complete or enables a public profile.

## Parent acceptance map

| Issue #82 acceptance criterion | Owning slices | Named evidence class |
| --- | --- | --- |
| valid local borrows compile with deterministic scope and access authority | #114, #115, #116, #119, #120, #121 | source-positive semantic fixtures plus mandatory verified-IR views |
| conflicts, escape, owner move/drop misuse, and invalid join/loop state fail stably | #115, #117, #120, #121 | source-hostile diagnostic fixtures with repeated-order checks |
| forged or incomplete IR borrow claims fail | #113, retained by #122 | focused IR positives; unused, sparse, duplicate, inactive, wrong-access, overlap, call, and edge-escape negatives |
| M1, M2, and non-borrowing M3 remain unchanged | every child, aggregate in #122 | focused quick lane, documentation checks, `pnpm preflight`, `pnpm m0:check`, required Linux/Windows jobs |

## Issue #113 evidence

The focused `zryna-ir` tests name the current positive and hostile boundary:

- `dense_shared_borrow_read_and_end_is_accepted` performs a real lexical `BorrowRead` between
  dense begin/end operations;
- `borrow_parameter_is_an_authenticated_active_authority` performs a real parameter read;
- `unused_borrow_parameter_authority_is_rejected` rejects forged signature-only metadata;
- `direct_call_rejects_borrow_authority_with_wrong_access` and
  `direct_call_rejects_repeated_exclusive_borrow_authority` freeze call authority;
- `exclusive_borrow_blocks_owner_copy_read` and the String clone/concat borrowed-source tests
  freeze owner exclusion;
- the existing dense-identity, inactive/end, edge-escape, place-overlap, exact-limit, and
  first-extra verifier corpus remains part of `pnpm m3:owned:quick` and full preflight.

Issue #113 changed no protocol-v4 syntax admission and no `zryna-semantics` producer. Its hostile
raw-IR corpus remains the independent authority beneath every later semantic slice.

## Issue #114 source checkpoint

One private parameter-free straight-line function may now declare a literal-initialized `bool` or
`i32` root, enter one explicit nested block, and declare one or more const
`Borrow<bool>`/`Borrow<i32>` aliases initialized directly by `borrow(root)`. Each alias must be read
at least once into an exact const Copy local. The semantic producer assigns dense `BorrowId`s,
emits `BeginBorrow(Shared)` and `BorrowRead`, and permits exact Copy owner reads while shared aliases
are active by lowering them to `CopyFromPlace(root)`. It emits `EndBorrow` in reverse declaration
order at the block's closing brace, and only then emits the final root read and return. The owner
remains initialized throughout and is reusable both during compatible shared access and after every
alias ends.

The producer computes the complete values, places, ownership transitions, cleanup-plan count, and
simultaneously active-borrow peak before constructing raw IR. Exact limits pass; arithmetic
overflow and the first extra resource fail as `ZRYNA-M3201`. Invalid shape, referent mismatch,
mutable aliases, wrong referents, unused aliases, owner replacement/effects inside the block, and
lexical escape fail as `ZRYNA-M3017`. Portable binding collisions retain `ZRYNA-M3002`. The
mandatory existing IR verifier still independently rejects forged uninitialized, moved, inactive,
duplicate, sparse, double-end, overlap, and edge-escape claims with stable `ZRYNA-I3011`
diagnostics.

This is not general reference semantics. The checkpoint excludes parameters, exported functions,
nonliteral roots, owned referents, exclusive borrows, projections, assignment, calls, branches,
loops, nested borrow blocks, stored aliases, returned aliases, lifetime inference, runtime
tracking, ABI changes, backend operations, driver routes, CLI selection, and target artifacts.

## Issue #115 source checkpoint

The same private parameter-free one-root shape now accepts `BorrowMut<bool>` and
`BorrowMut<i32>` aliases initialized by `borrowMut(root)` when the literal-initialized root was
declared with `let`. The frozen assignment syntax is deliberately:

```text
const alias: BorrowMut<i32> = borrowMut(root);
alias = 9;
```

The assignment is a write through the exclusive authority, not rebinding of the const alias. The
TypeScript 6 worker records only the source-faithful const declaration and assignment syntax;
Zryna semantics alone assigns that write-through meaning. An exact Copy alias read emits
`BorrowRead`; an exact literal write emits the literal followed by `BorrowWrite`. Reverse
`EndBorrow` still occurs at the one nested-block exit, after which the owner is readable and its
written value remains stored.

Each prospective alias is resolved and conflict-checked before receiving its dense planned
`BorrowId`. No raw function, instruction, or program is materialized until the complete plan and
resource preflight succeed. The admitted matrix is exact:

| Active authority on the root | requested shared from root | requested exclusive from root | shared from alias | exclusive from alias |
| --- | --- | --- | --- | --- |
| none | allowed | allowed for a `let` root | not applicable | not applicable |
| one or more shared | allowed | rejected | allowed only from a shared alias | rejected |
| one exclusive | rejected | rejected | rejected | rejected |

Thus shared/shared aliases coexist and a bounded shared-from-shared reborrow resolves to the same
sealed root. Mutable-from-shared reborrow and every reborrow from an exclusive alias fail as
`ZRYNA-M3017`. Exclusive access hides owner reads and ordinary owner assignment until the alias
ends. Shared writes, mismatched alias/initializer modes, wrong literal types, immutable-root
exclusive creation, conflicting direct aliases, unresolved sources, and unused authorities also
fail before raw-IR construction. The mandatory verifier independently requires exclusive
`BorrowWrite` direction and exact Copy operand type as `ZRYNA-I3005` structural evidence.

The producer preflights values, places, transitions, cleanup plans, and peak active borrows using
both read and write counts before materialization. It does not shorten a lifetime after last use:
all admitted aliases remain active until the reverse lexical end at the block close. This slice
adds no mutable-from-shared reborrow, reborrow from exclusive, projections, calls, CFG, nested
blocks, non-Copy referents, runtime borrow flag, ABI, backend, driver, CLI, or public profile.

## Issue #117 conditional-edge checkpoint

The same private parameter-free producer now admits one literal-initialized `bool` root, one
top-level `if`/`else` branching directly on that root, exactly one explicit nested lexical scope
in each arm, and one final root return. At least one arm must contain a complete admitted borrow;
the peer arm may contain an empty nested scope. A complete borrow in only one arm is valid.

Lowering is canonical and contains no borrow phi state:

```text
entry: initialize root; read bool condition
    Branch ───────────────┐
      │                   │
then: Begin/use...        else: Begin/use...
      EndBorrow reverse         EndBorrow reverse
      Jump ───────────────┬──── Jump
                          │
join:                 read root; Return
```

| Property | Conditional contract |
| --- | --- |
| blocks and edges | exactly four dense blocks and four edges in entry/then/else/join order |
| block parameters | none; borrow authority is never an `OwnershipFlow` value or edge argument |
| borrow identities | dense then-arm identities followed by dense else-arm identities |
| lexical end | every admitted arm emits reverse `EndBorrow` before its `Jump` |
| mutually exclusive access | shared/shared, exclusive/exclusive, and mixed shared/exclusive arms are valid |
| join authority | the root has identical ordinary state and zero active lexical authorities |
| resource accounting | arm-local values and transitions sum; one root place and four blocks/edges are fixed; active peak is `max(then, else)` |

Arm-local Copy read results are ephemeral verifier-visible values because their source bindings
cannot escape or be referenced after the admitted statement. They do not create branch-only places
whose initialization would enter join state. The complete two-arm plan, dense identities, exact
resources, and access conflicts are validated before any raw IR instruction or block is built.

The independent IR verifier rejects a lexical authority active at `Branch`, `Jump`, `Return`, or
`Trap`; an end in a successor, an end-only arm, and unequal ordinary state at the join. Its
ownership-flow worklist processes ready blocks by canonical dense block identity, while retaining
the original edge index for enum refinement, so reversing hostile branch targets cannot select a
different join-mismatch diagnostic.

This checkpoint excludes nested or repeated conditionals, loops, calls, parameters, public
functions, projections, owned roots, borrow-carrying block parameters, edge arguments, lifetime
shortening, runtime flags, ABI changes, backends, drivers, CLI selectors, artifacts, and public
profiles.

## Resource and verification boundary

The verifier limit remains 16,384 simultaneously active borrows per function. Function borrow
parameters count as active on entry. Lexical peak accounting follows instruction order, accepts the
exact limit, rejects the first extra with `ZRYNA-I3201`, and does not treat repeated sequential
begin/end sites as simultaneously live.

The borrow edit loop runs `cargo test --locked -p zryna-ir borrow` plus the focused conditional
join/edge tests for the retained verifier, and focused `borrow`, `exclusive_`, `conflict_matrix`,
`reborrow`, and `conditional_` semantic filters for the #114/#115/#117 producer. The checked gate
additionally requires the complete DataOwnershipV1 IR and semantic suites plus doctests, M3
contract/documentation tests, formatting and strict Clippy, `pnpm preflight`, and `pnpm m0:check`.
A quick lane cannot substitute for proportional exact/+1 or cross-platform merge evidence.

## Deliberately unavailable

M1 and explicit `control-flow-v1` M2 remain the only public profiles. This checkpoint supplies no
allocator, runtime helper, JavaScript representation, WebAssembly memory operation, native MIR,
object code, linker input, driver request, CLI flag, manifest-v3 bundle, website support claim, or
public `data-ownership-v1` execution. Those capabilities remain dependency-ordered later work.
