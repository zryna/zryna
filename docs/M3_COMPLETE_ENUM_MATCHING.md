# M3 complete enum matching lane

Issue #273 owns authenticated exhaustive enum matching and active-payload ownership on top of the
generic ownership and structured-CFG authorities from Issues #270 and #271. This document records
the isolated match-lane contract. It does not claim target execution, runtime behavior, public
profile activation, or completion of the parent Issue #269.

## Source and ownership contract

- The scrutinee is evaluated exactly once and must have the exact sealed nominal enum identity
  named by every arm.
- There is exactly one arm for each declared variant. Source order does not change the sealed
  declaration ordinal, and an empty, missing, repeated, unknown, or foreign arm set rejects.
- Only the selected arm receives its declared payload projection. Payloadless variants receive no
  binding, and inactive payload projections are never made available.
- Every arm produces one exact common result or terminates through the existing structured-CFG
  contract. A continuing owned result transfers one owner through one typed block parameter.
- Moving the active payload masks only that payload before the remaining enum root is cleaned.
  Copy payload reads retain their source. An unused owned payload is cleaned through the refined
  enum root, and no inactive payload is read or dropped.
- Nested matches and matches used within admitted constructors, calls, indexed operations,
  String operations, lexical scopes, branches, loops, and returns reuse the same continuation and
  cleanup state. Unequal ownership, mask, refinement, or borrow state is rejected rather than
  repaired by an implicit clone or conditional drop.

## Isolated executable evidence

| Boundary | Focused evidence |
| --- | --- |
| Payloadless and heterogeneous variants | `structured_match_exhausts_payloadless_and_mixed_payload_variants` covers a three-variant enum with empty, owned String, and Copy `i32` payloads; every arm carries one exact Copy result and records only its active cleanup ordinal. |
| Nested active-payload transfer | `structured_nested_matches_transfer_only_each_active_payload` covers an owned two-level enum graph, one nested exhaustive match in each outer arm, exact owned continuation parameters, and deterministic replay. |
| Empty exhaustiveness rejection | `structured_empty_match_rejects_with_one_stable_exhaustiveness_diagnostic` authenticates the canonical empty expression graph and requires one stable `ZRYNA-M3009` diagnostic at the match span. |
| Existing owned result matrix | `structured_match_owned_payloads_join_one_result_and_continue` covers String, FixedArray, Vec, nested container, Struct, and Enum results through move and structural-clone arms. |
| Existing composition and failure | The remaining `structured_match_source` tests cover constructor, Vec, direct-call, String, indexed, formal-borrow, and deterministic invalid-arm boundaries without granting new call or lexical-borrow syntax. |

The fixture and test modules are deliberately match-owned and isolated from the shared dispatcher,
central test registry, checked issue graph, roadmap, and status documents so Issue #272 can proceed
in a separate worktree. Final integration must bind these exact test names into the checked #273
matrix, add independent hostile raw-IR evidence for discriminant/refinement/cleanup/resource
forgeries, and run the full repository and hosted platform gates.

## Retained boundaries

There are no source-selected discriminants, wildcard arms, implicit clones, inactive payload
access, stored or returned borrows, borrow-carrying CFG edges, runtime/backend behavior, public
aggregate ABI, or public `data-ownership-v1` selection. Non-indexed active-payload borrowing remains
Issue #275 and consumes this lane only after the final #273 refinement interface is integrated.
