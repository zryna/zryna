# M3 complete enum matching

Issue #273 owns authenticated exhaustive enum matching and active-payload ownership on top of the
generic ownership and structured-CFG authorities from Issues #270 and #271. The integrated #333
source, #334 independent-IR and #335 resource/evidence work makes it a checked compiler-only
closure candidate. It does not claim target execution, runtime behavior, public profile
activation, or independent completion of the parent Issue #269; the later integration proof now
records the combined child authority.

## Source and ownership contract

- The scrutinee is evaluated exactly once and must have the exact sealed nominal enum identity
  named by every arm.
- There is exactly one arm for each declared variant. Source order does not change the sealed
  declaration ordinal, and an empty, missing, repeated, unknown, or foreign arm set rejects.
- Only the selected arm receives its declared payload projection. Payloadless variants receive no
  binding, and inactive payload projections are never made available.
- Every arm produces one exact common result and continues through one typed result block
  parameter. Terminating match arms are not represented by the current protocol and remain
  excluded.
- Moving the active payload masks only that payload before the remaining enum root is cleaned.
  Copy payload reads retain their source. An unused owned payload is cleaned through the refined
  enum root, and no inactive payload is read or dropped.
- Nested matches and matches used within admitted constructors, calls, indexed operations, String
  operations, formal-authority continuations, and direct returns reuse the same continuation and
  cleanup state. Unequal ownership, mask, refinement, or borrow state is rejected rather than
  repaired by an implicit clone or conditional drop. Complete multi-arm matches nested directly in
  branch or loop bodies require separate evidence and are not claimed by this closure.

## Executable evidence

| Boundary | Focused evidence |
| --- | --- |
| Payloadless and heterogeneous variants | `structured_match_exhausts_payloadless_and_mixed_payload_variants` covers a three-variant enum with empty, owned String, and Copy `i32` payloads; every arm carries one exact Copy result and records only its active cleanup ordinal. |
| Nested active-payload transfer | `structured_nested_matches_transfer_only_each_active_payload` covers an owned two-level enum graph, one nested exhaustive match in each outer arm, exact owned continuation parameters, and deterministic replay. |
| Canonical ordering and once-only scrutinee | `structured_match_arm_order_is_canonical_and_scrutinee_is_consumed_once` reverses a three-variant source order, requires sealed declaration ordinals, and observes one fallible constructed scrutinee plus one materialized dispatch operand. |
| Exhaustiveness and payload diagnostics | `structured_match_invalid_arm_sets_reject_exactly_replay_and_recover` covers empty, missing, duplicate, unknown, payloadless-binding, and missing-binding cases with the exact protocol `ZRYNA-Y4002` or semantic `ZRYNA-M3009` boundary diagnostic, deterministic replay, and subsequent valid recovery. |
| Existing owned result matrix | `structured_match_owned_payloads_join_one_result_and_continue` covers String, FixedArray, Vec, nested container, Struct, and Enum results through move and structural-clone arms. |
| Existing composition and failure | The remaining `structured_match_source` tests cover constructor, Vec, direct-call, String, indexed, formal-borrow, and deterministic invalid-arm boundaries without granting new call or lexical-borrow syntax. |

The checked [complete enum matching closure matrix](M3_COMPLETE_ENUM_MATCHING_MATRIX.md) binds the
canonical source, independent-IR and resource rows to real test functions. It covers exact
discriminant/refinement dominance, payload ownership and cleanup forgeries, graph and ownership
resource boundaries, checked overflow, atomic rejection and deterministic recovery. Complete and
ignored suites, preflight, M0/M2 regressions, independent review and hosted Linux/Windows gates
remain merge requirements rather than capabilities supplied by the matrix.

## Retained boundaries

There are no source-selected discriminants, wildcard or terminating arms, implicit clones,
inactive payload access, stored or returned borrows, borrow-carrying CFG edges, runtime/backend
behavior, public aggregate ABI, or public `data-ownership-v1` selection. Non-indexed active-payload
borrowing remains Issue #275 and consumes this sealed refinement interface.
