# M3 normative source completion

Issue #269 is the integration proof for the complete admitted `DataOwnershipV1` source and
independently verified Universal IR surface. It does not add syntax or repair rejected ownership
states. It records that every dependency-owned capability now reaches the same sealed authority
consumed by target backends.

## Dependency-ordered closure

| Requirement | Owning issues | Integrated evidence |
| --- | --- | --- |
| Shared/Weak payload and control behavior | #83, #259–#264 | `M3_SHARED_WEAK_AUTHORITY.md`, `M3_SHARED_WEAK_EVIDENCE.md` |
| Indexed borrow authority and array/Vec producers | #254–#256, #274 | `M3_INDEXED_BORROW_AUTHORITY.md`, checked indexed source and hostile-IR suites |
| Generic aggregate/Vec composition and structural clone | #270, #277–#279 | `M3_GENERIC_OWNED_COMPOSITION_MATRIX.md`, `M3_OWNERSHIP_COMPOSITION_EVIDENCE.md` |
| Structured owned control flow and exact scope cleanup | #271, #325–#327 | `M3_STRUCTURED_OWNED_CONTROL_FLOW_MATRIX.md` |
| Internal owned calls and authenticated module identity | #272, #329–#331 | `M3_OWNED_CALL_CLOSURE_MATRIX.md` |
| Exhaustive enum matching and active payload ownership | #273, #333–#335 | `M3_COMPLETE_ENUM_MATCHING_MATRIX.md` |
| Non-indexed owned lexical borrowing | #275, #337–#340 | `M3_NONINDEXED_OWNED_BORROWING_MATRIX.md` |

The checked dependency registry remains authoritative: #269 depends on #83, #254–#256,
#270–#275 and #277–#279. The target issues #84–#86 consume #269; they do not redefine its source
semantics. The registry order is unchanged because closure changes status, not dependency identity.

## Backend-safe integration surface

`zryna_ir::data_ownership_v1::VerifiedInstruction::backend_instruction` and
`VerifiedTerminator::backend_terminator` expose closed, owner-branded operand and edge views. They
retain the verified program, layout, ownership, cleanup, borrow and CFG authority and never expose
the raw program. JavaScript, WebAssembly and native MIR lowering use this shared read-only surface,
so no target guesses operand roles or accepts producer claims directly.

## Retained exclusions

No break/continue, Vec pop, implicit clone, mismatched-state join repair, arbitrary non-Copy
container move-out, escaping borrow, recursion, user generic, raw pointer, unsafe, FFI, thread,
tracing GC, strong cycle, public aggregate ABI, driver route or public profile is introduced by
this closure. Three-target conformance, candidate integration and public activation remain owned
by #87–#90 in dependency order.
