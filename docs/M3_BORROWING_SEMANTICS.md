# M3 bounded borrowing implementation contract

Status: Issue #113 contract and verified-IR prerequisite complete. Source-level semantic borrowing
is not implemented by this checkpoint.

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

Issue #113 changes no protocol-v4 syntax admission and no `zryna-semantics` producer. Borrow type
nodes remain rejected by the current aggregate gate and every current semantic function continues
to emit an empty borrow-parameter inventory. That gap is intentional until #114.

## Resource and verification boundary

The verifier limit remains 16,384 simultaneously active borrows per function. Function borrow
parameters count as active on entry. Lexical peak accounting follows instruction order, accepts the
exact limit, rejects the first extra with `ZRYNA-I3201`, and does not treat repeated sequential
begin/end sites as simultaneously live.

The edit loop is `cargo test --locked -p zryna-ir borrow`. The checked Issue #113 gate additionally
requires the complete DataOwnershipV1 IR suite and doctests, M3 contract/documentation tests,
formatting and strict Clippy, `pnpm preflight`, and `pnpm m0:check`. A quick lane cannot substitute
for proportional exact/+1 or cross-platform merge evidence.

## Deliberately unavailable

M1 and explicit `control-flow-v1` M2 remain the only public profiles. This checkpoint supplies no
allocator, runtime helper, JavaScript representation, WebAssembly memory operation, native MIR,
object code, linker input, driver request, CLI flag, manifest-v3 bundle, website support claim, or
public `data-ownership-v1` execution. Those capabilities remain dependency-ordered later work.
