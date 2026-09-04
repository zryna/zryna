# M3 bounded borrowing implementation contract

Status: bounded compiler-boundary implementation complete for Issue #82.

Issues #113, #114, #115, #116, #117, #119, #120, and #121 complete. Issue #116 adds one
bounded shared-read shape for a whole owned root after independent verification and required merge
gates.
The internal semantic producer admits bounded
shared and exclusive Copy-root source shapes, one shared-from-shared reborrow, and one canonical
conditional plus one canonical bool-root loop whose local lexical authorities are completely
discharged before every edge. It also admits static recursively Copy Struct-field and constant
fixed-array projected borrows with exact prefix overlap. Issue #119 adds bounded private
straight-line whole-root direct calls with exact recursively Copy signatures and nonescaping
borrow authority. Nested/repeated control-flow borrowing remains a dependency-ordered later
checkpoint.

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
    ↓ dependency-ordered semantic slices
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
  elements are disjoint. Normative v1 rules make dynamic indices and Vec elements conservatively
  overlap the complete container. The narrower Issue #120 source checkpoint rejects those forms;
  that rejection does not narrow the normative rule.
- The owner remains initialized while borrowed, but overlapping move, drop, replacement, mutable
  container operation, or other exclusive owner use is rejected. An exclusive borrow also blocks
  overlapping owner reads.
- A lexical borrow starts only after its referent is available, has one dense function-local
  `BorrowId`, and ends exactly once before the containing control-flow edge.
- A function borrow parameter is active on entry, cannot be ended by the callee, and must perform a
  `BorrowRead`, `BorrowWrite`, or exact direct-call borrow argument use. Unused signature metadata
  is invalid.
- `BorrowRead` and `BorrowWrite` currently carry only `Copy` referents. They cannot produce or
  transfer an owned value. Issue #116 instead reuses existing owned clone/concatenation operations
  while shared authority is active: each result has a distinct owner, without cloning the borrow
  or transferring its source owner.
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
| #116 | shared reads of owned roots | distinct owned read results; no source-owner transfer, cloned borrow, stored reference, or borrow cleanup authority |
| #117 | conditional edges | every arm ends authority; no borrow-carrying block parameter |
| #119 | bounded internal calls | exact parameter modes; callee cannot retain, return, or end caller authority |
| #120 | projected disjointness | static siblings may coexist; overlapping exclusive authority fails; dynamic/Vec source forms remain rejected |
| #121 | loop edges | header/backedge state equality and per-iteration lexical end |
| #122 | closure, limits, regressions, and documentation | final diagnostic, exact/+1, Linux/Windows, M0/M1/M2/non-borrow M3 evidence |

The five slices after #115 are independent. They must not silently broaden one another. #122 owns
the aggregate closure claim; no earlier child marks Issue #82 complete or enables a public profile.

## Parent acceptance map

| Issue #82 acceptance criterion | Owning slices | Named evidence class |
| --- | --- | --- |
| valid local borrows compile with deterministic scope and access authority | #114, #115, #116, #117, #119, #120, #121 | source-positive semantic fixtures plus mandatory verified-IR views |
| conflicts, escape, owner move/drop misuse, and invalid join/loop state fail stably | #115, #117, #120, #121 | source-hostile diagnostic fixtures with repeated-order checks |
| forged or incomplete IR borrow claims fail | #113, retained by #122 | focused IR positives; unused, sparse, duplicate, inactive, wrong-access, overlap, call, and edge-escape negatives |
| M1, M2, and non-borrowing M3 remain unchanged | every child, aggregate in #122 | focused quick lane, documentation checks, `pnpm preflight`, `pnpm m0:check`, required Linux/Windows jobs |

### Named closure evidence

The following tests make the parent map inspectable. Issue #122 consolidates the bounded
implementation, resource and regression evidence; source presence alone is not execution proof.
The closure change requires independent review and successful integrated Linux/Windows gates.
Semantic fixtures authenticate syntax snapshots and inspect verified IR;
they do not establish public CLI or target-runtime support.

Semantic test paths below are relative to
`crates/zryna-semantics/src/data_ownership_v1/tests/`.

| Boundary | Positive test | Rejection or replay test |
| --- | --- | --- |
| shared and exclusive root scope (`straight_root_borrows.rs`) | `shared_root_aliases_read_copy_values_end_in_reverse_and_restore_owner_access`; `exclusive_root_borrow_reads_writes_and_restores_owner_access` | `complete_root_alias_conflict_matrix_fails_before_ir_construction`; `exclusive_lowering_and_conflict_diagnostics_are_deterministic` |
| owned-root source preservation (`owned_root_borrow_reads.rs`) | `owned_root_shared_reads_reuse_existing_operations_and_restore_each_owner` | `owned_root_borrow_faults_retain_the_source_and_exact_cleanup_authority`; `owned_root_borrow_exclusions_are_ordered_source_faithful_and_deterministic` |
| conditional discharge (`conditional_root_borrows.rs`) | `conditional_root_borrows_use_canonical_blocks_and_discharge_each_arm`; `conditional_root_borrow_accepts_exclusive_authority_in_both_arms` | `conditional_arm_conflicts_and_owner_access_fail_before_ir_construction`; `conditional_root_borrow_lowering_is_deterministic` |
| loop discharge (`loop_root_borrows.rs`) | `loop_root_borrows_discharge_before_the_canonical_backedge`; `loop_shared_root_borrow_keeps_owner_copy_reads_inside_the_body` | `loop_root_borrow_exclusions_are_source_faithful_ordered_and_stable` |
| exact private calls (`borrow_call_conformance.rs`) | `accepted_borrow_call_fixture_snapshots_authenticate_and_lower` | `rejected_borrow_call_fixtures_freeze_diagnostics_spans_and_recovery` |
| unchanged forwarded authority (`borrow_forwarding_calls.rs`) | `lexical_authority_is_forwarded_unchanged_and_ended_only_by_its_caller` | `post_preflight_argument_failure_restores_the_full_lowerer_snapshot_before_replay` |
| static projected disjointness (`projected_borrows.rs`) | `projected_borrows_preserve_exact_static_paths_and_disjoint_authority`; `overlapping_shared_parent_and_child_keep_independent_verified_authority` | `projected_borrow_exclusions_are_exact_ordered_and_deterministic`; `projected_borrow_lowering_replays_the_complete_place_and_authority_trace` |

The independent verifier's tests in `crates/zryna-ir/src/data_ownership_v1/tests.rs`
include real accepted authority in `dense_shared_borrow_read_and_end_is_accepted` and
`borrow_parameter_is_an_authenticated_active_authority`. Forged or incomplete claims are covered
by `unused_borrow_parameter_authority_is_rejected`,
`sparse_and_duplicate_borrow_parameter_metadata_is_rejected`, and
`sparse_duplicate_and_inactive_lexical_borrow_authority_is_rejected`. Edge and callee exclusions
are covered by `lexical_borrow_cannot_escape_return_or_trap`,
`lexical_borrow_cannot_cross_branch_or_jump_edges`,
`lexical_borrow_loop_rejects_backedge_escape_inactive_header_end_and_state_mismatch`, and
`borrow_parameter_cannot_be_ended_or_exported`.

Issue #248 adds fully verified dense exact/first-extra resource programs in
`crates/zryna-ir/src/data_ownership_v1/tests/borrow_resource_boundaries.rs`:

- `dense_lexical_active_borrow_exact_and_first_extra_are_fully_verified`;
- `parameter_and_lexical_authorities_share_the_authenticated_active_limit`;
- `sequential_dense_lexical_sites_may_exceed_the_active_borrow_limit`.

These tests distinguish simultaneously active authorities from total lexical sites. Other
resource-formula and raw-preflight tests prove their counters and rejection order; an exact
synthetic count alone does not prove that a complete program authenticates and verifies. Fixed
source shapes may hit another resource limit before a nominal maximum is reachable. Loop trace
walks over verified views prove scope topology, not execution by a JavaScript, WebAssembly, or
native runtime. The final closure report must preserve these evidence distinctions.

Normative indexed borrowing remains separately tracked: #254 owns the verified element-access
and complete-container conflict authority; #255 and #256 own dynamic fixed-array and Vec-element
source producers. They must preserve the specification's referent, overlap, evaluation, bounds,
and cleanup rules before complete target support and #89/#90 public activation. Closing the
bounded #82 checkpoint does not implement or waive that chain.

Issue #250 adds the following tests in
`crates/zryna-ir/src/data_ownership_v1/tests/borrow_loop_nesting.rs`:

- `authenticated_borrow_loop_nesting_accepts_exact_and_rejects_first_extra`;
- `authenticated_nested_borrow_loops_replay_the_header_latch_and_scope_trace`;
- `authenticated_nested_loop_rejects_a_borrow_carried_to_its_latch`.

They use real reducible nested headers/latches and a shared begin/read/end sequence. Full M3 IR
verification accepts depth 128, rejects 129, and rejects an active borrow carried to the latch.
This does not admit nested source borrowing or prove runtime loop execution.

Issue #251 adds `owned_root_shared_read_drop_budget_is_authenticated_exact_and_first_extra` in
`crates/zryna-semantics/src/data_ownership_v1/tests/owned_root_borrow_reads.rs`. Authenticated
source/snapshot lowering accepts exactly 262,144 combined inserted drop actions, rejects the first
extra action at return cleanup, and verifies deterministic recovery and source-owner preservation.
The proportional test is ignored by ordinary test invocation and must run in the include-ignored
preflight lane; an ordinary suite pass alone is not its execution evidence.

Issues #250 and #251 are merged in the implementation checkpoint
`834ca0ef0697694b9fd7aee8ef68215892af85fe`. The pre-merge candidate passed its required
Linux/Windows checks, and this merge preserves its complete tree. Issue #122
adds the named evidence map and documentation guards without changing compiler behavior. Its own
closure commit must also pass the integrated gates; earlier child results do not waive them.

### Resource evidence boundaries

| Resource | Named authority | What the evidence proves |
| --- | --- | --- |
| active authorities | #248 dense lexical, parameter-plus-lexical and sequential tests above | complete raw programs and opaque verified traces, not duplicate-ID counter-only fixtures |
| M3 loop nesting | #250 exact/first-extra and hostile latch tests above | complete reducible IR graph verification; source nested loops remain outside this checkpoint |
| inserted drop actions | #251 authenticated owned-root boundary above | source/semantic/IR cleanup authority at the exact and first-extra frontier; no allocator or backend execution |
| values, places and transitions | `root_borrow_resources_enforce_exact_value_place_and_transition_limits`; `borrow_call_resource_preflight_accepts_exact_limits_and_rejects_first_extra_in_order` | resource planning and rejection order; not a claim that every nominal maximum is reachable by an admitted source shape |
| fixed branch blocks/edges | `root_borrow_resources_enforce_exact_block_and_edge_limits` | exact/first-extra resource planning; current canonical branch/loop shapes retain four blocks and four edges |
| call edges/depth and arithmetic overflow | `borrow_call_program_edge_and_depth_boundaries_are_exact`; `borrow_call_resource_overflow_precedes_limit_selection_and_preserves_authority_cost` | program-budget arithmetic and precedence; independent call-graph verification separately proves real depth-128 acceptance and depth-129 rejection |
| combined owned-root wrapper costs | `owned_root_borrow_authority_budget_is_exact_saturating_and_atomic` | checked wrapper instruction/drop/borrow cost accounting before rewrite; not a second claim of a fully verified maximum-size source program |

The ordinary semantic resource tests live in `conditional_root_borrows.rs`,
`lexical_borrow_calls.rs`, and `owned_root_borrow_reads.rs`. The complete IR suite retains its
nominal type, construction operand, place, transition, cleanup-plan, drop and diagnostic-cap tests.
Synthetic preflight collections authenticate count rejection, not all IDs or graph structure.
Final closure must run the whole relevant suites and proportional cases, retaining these
distinctions instead of treating every helper test as an executable target program.

## Issue #122 closure scope

The bounded internal borrowing implementation is complete; Shared/Weak source production is the
next dependency-ready work in #83, not an implemented capability. The tracked #254–#256 indexed
borrowing chain and all runtime, target, driver, conformance and public-profile gates remain open.
This checkpoint does not certify the entire normative M3 profile or enable a public CLI profile.

At the implementation checkpoint above, ordinary semantics has 344 passing tests and two ignored
proportional tests. The include-ignored M3 lane runs 324 tests, including both proportional cases;
together the lanes cover all 346 semantic tests. The M3 IR lane runs 141 tests, including the
proportional String boundary; ordinary IR has 170 passing tests and one ignored case. These are
distinct lanes, not a claim of 346 or 171 ordinary passes. The 44 named references above also have
actual passing execution records. Reproduction remains `pnpm preflight` plus `pnpm m0:check` and
the required Linux/Windows CI, with exact commit and artifact provenance recorded by publication.

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

## Issue #116 owned-root shared-read checkpoint

One private parameter-free straight-line function may declare one initialized non-Copy root,
enter one explicit nested lexical block, declare exactly one const shared alias initialized as
`borrow(root)`, perform one or more admitted read-only operations through that alias, end the
authority at the block close, and finally return the original root. The alias must name the exact
whole root; projected referents and reborrows are not admitted.

The admitted operations are deliberately closed:

| Whole-root type | Read through the shared alias | Result authority |
| --- | --- | --- |
| `String` | explicit clone or checked concatenation of the alias with itself | a distinct owned `String` result |
| exact `Vec<bool>` or `Vec<i32>` | checked constant indexing | a `Copy` element result |
| supported non-Copy Struct, root Enum, or fixed array | explicit whole-aggregate clone | a distinct owned aggregate result |

The owner remains initialized and pending throughout the lexical block. Each owned clone or
concatenation result has its own temporary owner; none consumes or aliases the source owner. The
final return therefore transfers the original root only after `EndBorrow`. Lowering reuses the
existing String, Vec-index, aggregate-clone, cleanup-plan, initialized-prefix cleanup, and fault
authorities. It adds only one dense `BeginBorrow(Shared)`/`EndBorrow` authority around those
existing operations. Owned read-result locals are explicitly dropped in reverse declaration order
before `EndBorrow`; they are removed from final-return cleanup. After the existing owned producer
has built its sealed straight-line candidate, the wrapper checks the combined instruction count,
the added lexical drops, the two authority transitions, and one active authority before applying
the scope rewrite and submitting the final raw function to verification.

This does not widen the IR `BorrowRead` instruction: `BorrowRead` remains Copy-only. The producer
resolves each admitted owned read to the borrowed root and emits the already verified owned
operation while shared authority is active. No owned value, cleanup obligation, stored reference,
or return authority is transferred through the borrow itself.

The checkpoint excludes multiple aliases, mutable or exclusive access, projections, mutation,
moves, replacement, explicit drops, calls, parameters, public functions, CFG, stored or returned
aliases, alternate return values, new runtime operations, ABI changes, backend lowering, driver or
CLI routes, artifacts, and public-profile activation. Unsupported shapes fail before raw-IR
construction; the mandatory existing verifier remains the final authority for owner exclusion,
distinct result ownership, cleanup, and lexical end.

Existing semantic evidence is
`owned_root_shared_reads_reuse_existing_operations_and_restore_each_owner` (distinct results and
post-end root return), `owned_root_borrow_faults_retain_the_source_and_exact_cleanup_authority`
(source retention on failure), and
`owned_root_borrow_exclusions_are_ordered_source_faithful_and_deterministic` (rejected shapes).
The independent IR test `borrow_read_and_write_reject_non_copy_string_referents` preserves the
Copy-only instruction boundary; owned operations do not widen it.

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

## Issue #119 bounded internal call checkpoint

One private straight-line function may declare a recursively Copy result and an exact signature
whose recursively Copy value parameters and one or more shared or exclusive borrow parameters are
interleaved in source order. An admitted caller passes either active whole-root lexical authority
or the same active borrow-parameter authority to exactly the declared referent and access mode.
Source arguments evaluate once from left to right; lowering then materializes the verified call's
value arguments followed by borrow arguments without changing their source evaluation order. A
borrow-parameter callee may read shared or exclusive authority, write only exclusive authority, or
forward the same authority through another bounded exact internal call.

Each admitted lexical borrow block contains one direct call. The call graph is private, static,
acyclic, and bounded. The callee cannot end, return, store, or capture caller authority. The caller
retains responsibility for reverse lexical `EndBorrow`, and the direct call retains its exact
`CallTrap` cleanup site. The mandatory existing IR verifier remains final authority for active
identity, exact referent and access, overlapping exclusive arguments, real parameter use,
ownership-flow nonescape, recursion, and call depth. Its retained evidence accepts static depth
128, rejects depth 129 with `ZRYNA-I3009`, and rejects mutual recursion.

The `tests/m3-contract-v1.json` `borrowCallConformance` registry authenticates exactly 36 source and
protocol-v4 snapshot files, 5 accepted cases, and 13 excluded cases. Implementation plus fixture
provenance is merged-main commit `32e3f0607389dd1274c21770088456c765ee4fb7` from PR #184. That
immutable tree has registry SHA-256
`d61d1ec50005bbed7d86f029fa6ece5efa7517d495b6aed6e9b0f1c15f69e20f` and canonical
`borrowCallConformance` section SHA-256
`ca7ca013771f8ebb0ddc3f7791bc46db6378892e89f3e8e570a44e42e687fc20`.

- Accepted: `borrow-forwarding-exclusive`, `borrow-forwarding-shared`,
  `borrow-parameter-mixed-order`, `lexical-borrow-call-exclusive`, and
  `lexical-borrow-call-shared`.
- Excluded: `borrow-call-owned-shape`, `borrow-call-public-abi`,
  `borrow-call-repeated-exclusive`, `borrow-call-result-escape`, `lexical-borrow-call-cfg`,
  `lexical-borrow-call-inactive`, `lexical-borrow-call-projected`,
  `lexical-borrow-call-repeated`, `lexical-borrow-call-wrong-access`,
  `lexical-borrow-call-wrong-arity`, `lexical-borrow-call-wrong-borrow-kind`,
  `lexical-borrow-call-wrong-referent`, and `lexical-borrow-call-wrong-value-kind`.

Resource planning uses checked arithmetic before limit selection and reserves values, places,
ownership transitions, blocks, edges, active borrows, cleanup plans, call edges, and static call
depth before raw-IR materialization. Exact limits pass and the first extra fails in canonical
resource order. The required focused and closure evidence is:

```text
pnpm m3:contract
cargo test --locked -p zryna-semantics 'data_ownership_v1::tests::borrow_call_conformance::'
cargo test --locked -p zryna-semantics 'data_ownership_v1::tests::lexical_borrow_calls::'
cargo test --locked -p zryna-ir 'data_ownership_v1::tests::'
pnpm m3:syntax:quick
pnpm m3:owned:quick
pnpm docs:check
pnpm preflight
pnpm m0:check
```

Protocol v4 is consumed unchanged. This checkpoint adds no syntax contract, runtime lifetime
state, ABI carrier, JavaScript/WebAssembly/native lowering, driver route, CLI selector, artifact,
website support claim, or public-profile activation. Projected or derived forwarding, multiple
calls in one lexical block, CFG crossing, recursion or indirect calls, owned aggregate call shapes,
non-Copy mutation, public borrow signatures, and retained or escaping authority remain excluded.

## Issue #120 projected-disjointness checkpoint

The private parameter-free straight-line producer now also accepts one literal-initialized,
recursively Copy Struct or fixed-array root. A borrow place is canonicalized as the root followed
by a finite sequence of `StructField` ordinals and `FixedArrayConstant` indices. Every prefix is
materialized once, identities remain dense, and `BeginBorrow` names the exact final place rather
than collapsing static siblings to their common root.

For these admitted static source paths, overlap is exactly prefix-based:
the same path and every ancestor/descendant pair overlap, while
distinct static siblings are disjoint. Consequently overlapping shared/shared parent and child
authorities may coexist; shared/exclusive, exclusive/shared, and exclusive/exclusive overlaps are
rejected; disjoint exclusive struct fields or fixed-array elements may coexist. An overlapping
exclusive alias hides only overlapping owner reads, so a Copy owner read of a disjoint sibling
remains valid. Shared-from-shared reborrow retains the same canonical place.

| Projection form | Issue #120 result |
| --- | --- |
| declared recursively Copy Struct field | exact static place |
| in-range nonnegative fixed-array literal index | exact static place |
| same, ancestor, or descendant path | overlapping |
| distinct static Struct fields or fixed-array constants | disjoint |
| dynamic or negative/out-of-range fixed-array index | deterministic fail-closed diagnostic |
| Vec element, enum root/payload, non-Copy root, or non-Struct field continuation | rejected before raw IR construction |

The complete plan preflights constructor values, Copy reads, write RHS values, unique projection
prefixes, ownership transitions, cleanup plans, and peak active authorities. Borrow syntax itself
produces no IR value; the global value preflight and the projected exact formula both count only
emitted definitions. The positive formula yields 19 values, 14 places, and 38 transitions; the
fixture freezes 14 materialized places, five active authorities, reverse lexical ends, zero return
cleanup actions, and deterministic ordered place/authority replay. Hostile fixtures freeze ordered source spans and diagnostics
for every overlap direction, invalid fields and indices, rejected dynamic access, Vec/enum/
non-Copy roots, and unsupported projection continuation. The independent IR corpus separately
proves projected move, replace, drop, direct-call, and hostile replay behavior.

`projected_borrows_preserve_exact_static_paths_and_disjoint_authority` and
`overlapping_shared_parent_and_child_keep_independent_verified_authority` prove admitted static
access; `projected_borrow_exclusions_are_exact_ordered_and_deterministic` pins the rejected dynamic
and Vec source forms. These tests do not claim implemented full-container dynamic/Vec borrowing;
that remains the normative rule in `DATA_OWNERSHIP_V1.md` sections 5 and 9, not a new static-only rule.

This checkpoint adds no runtime address, pointer, lifetime token, garbage collection, ABI,
backend, driver route, CLI selection, target artifact, or public profile. Dynamic index reasoning,
Vec/enum projected borrowing, non-Copy referents, stored references, and lifetime shortening remain
unavailable.

## Issue #121 loop-edge checkpoint

One private parameter-free function may instead use a literal-initialized `bool` root as the
condition of one top-level `while`. The loop body itself is the single lexical borrow scope: it
contains only the admitted aliases, Copy reads, and exclusive writes, then reverse-ends every
authority before the backedge. An extra nested block is not this source shape.

Lowering has exactly four dense blocks and four empty-argument edges: preheader initialization,
header `CopyFromPlace` plus branch, body borrow operations plus reverse `EndBorrow` and backedge,
and exit root read plus return. No block has parameters and no borrow authority, value block
parameter, or edge argument is carried; the exact root owner/initialization state is restored at
the header and backedge. The same static dense borrow identity and resource reservation serve
zero, one, or many structural body visits. Values are `reads + writes + 3`, places are one,
transitions are `2 * aliases + reads + 2 * writes + 4`, active capacity is the body alias count,
and cleanup-plan count is one.

The verifier rejects authority active at the header branch, body backedge, exit return/trap, or
any other edge, rejects inactive or mismatched ends, and rejects ordinary owner-state mismatch at
the backedge. Generic branch/jump and return/trap escape tests remain the shared authority for
those terminators. Public or parameterized functions, nonliteral/non-bool roots, alternate
conditions, extra nested blocks, nested/repeated loops, `break`, `continue`, body return, calls,
projections, lifetime shortening, loop-carried authority, runtime flags, ABI/backend/driver/CLI
changes, artifacts, and public profiles remain excluded.

## Resource and verification boundary

The verifier limit remains 16,384 simultaneously active borrows per function. Function borrow
parameters count as active on entry. Lexical peak accounting follows instruction order, accepts the
exact limit, rejects the first extra with `ZRYNA-I3201`, and does not treat repeated sequential
begin/end sites as simultaneously live.

The borrow edit loop runs `cargo test --locked -p zryna-ir borrow` plus the focused conditional
join/edge tests for the retained verifier, and focused `borrow`, `exclusive_`, `conflict_matrix`,
`reborrow`, `conditional_`, `borrow_call`, `projected_borrow`, and `loop_root_borrow` semantic
filters for the #114/#115/#117/#119/#120/#121 producer. The checked gate
additionally requires the complete DataOwnershipV1 IR and semantic suites plus doctests, M3
contract/documentation tests, formatting and strict Clippy, `pnpm preflight`, and `pnpm m0:check`.
A quick lane cannot substitute for proportional exact/+1 or cross-platform merge evidence.

## Deliberately unavailable

M1 and explicit `control-flow-v1` M2 remain the only public profiles. This checkpoint supplies no
allocator, runtime helper, JavaScript representation, WebAssembly memory operation, native MIR,
object code, linker input, driver request, CLI flag, manifest-v3 bundle, website support claim, or
public `data-ownership-v1` execution. Those capabilities remain dependency-ordered later work.
