# M7 bounded generic conformance requirements

## Current #416 candidate evidence

The explicit source-checkout `bounded-generics-v1` implementation is described by the
[complete programs](M7_GETTING_STARTED.md) and [public reference](M7_PUBLIC_PROFILE.md).
The [M7 candidate matrix](M7_SUPPORT.md) separates implemented host admission from execution
qualification. The earlier #415 reference packet below remains specification evidence, with
its original states, fixtures and digests preserved. Neither that packet nor a private
checkpoint substitutes for the following current implementation gates.

| Owning gate | Current acceptance evidence required |
| --- | --- |
| Source/provider scope | Actual native/bootstrap/compare observations under v5; retained module/source identities; source/provider drift rejection through terminal commit |
| Original semantics and instances | All original bodies including unused templates; exact closed keys/substitutions and instance/edge/resource boundaries; original diagnostic codes and UTF-8 spans |
| Layout/runtime/IR | Independent both-target layout/runtime declarations and hostile wire/typed/state checks; source-bound match regions, loan lifetimes, ownership order and cleanup; exact/first-extra/overflow/pristine-retry controls |
| Three target consumers | Same new execution seal; ordinary and observed outputs remain separate; fixed scalar/trap/cleanup/fault oracles; native SSA/ELF and core-Wasm byte audits; bounded deep Drop/Clone witnesses |
| Public build/run | Native frontend all-target build without Node/linker, zero-export library build, typed scalar invocation, module entry identity, provider replay, separate strict manifest, collision/cleanup and negative admission checks |
| Integration/platform | Unchanged canonical M0 workspace suite and Linux preflight; nonzero exact public packet on both hosts; additional Windows portable-deep and shared ignored ABI controls; every branch-required hosted check |

The reviewed finite public packet is registered in
[`tests/m7-generic-acceptance-v1.json`](../tests/m7-generic-acceptance-v1.json) and run by
[`scripts/run-m7-generic-acceptance.mjs`](../scripts/run-m7-generic-acceptance.mjs) in the
dedicated both-host `m7-platform` matrix after both Rust jobs succeed. Canonical M0 remains in
each upstream Rust platform job; corroborate its same-source matching-host logs separately. Its required names total **11 on Linux**
and **18 on Windows**: 12 public CLI, 3 portable-deep and 3 shared ignored ABI tests on Windows. These are
required packet counts, not recorded passing results. Workspace/preflight totals are separate;
the packet does not rerun or relabel them. Linux preflight already includes the shared ABI
ignored controls. The Windows command must execute those three controls, not accept a
zero-test or ignored result. Historical ignored tests remain reported as ignored.

Each receipt retains actual host/tool versions, commands, status, signal/error, elapsed time,
exact executed names, passed/failed/ignored/filtered counts and bounded log lengths/hashes.
It binds actual clean HEAD and tree before/after to the tested workflow `github.sha`. A PR
head is recorded separately: GitHub merge-checkout evidence is not called exact-PR-head
evidence. Closure additionally requires corroborating both platform receipts, canonical M0
logs, Linux preflight and final required hosted checks, and comparing the tested merge tree
with the current PR-head tree before merge.

Finite local Linux suites have been exercised during integration; this page is not their
revision-bound receipt. Hosted Windows acceptance for this candidate is **pending**. No
all-platform pass, website publication, release or #416 closure is asserted here.

## Historical #415 specification evidence

State: **specified-only** evidence map for [Issue #415](https://github.com/zryna/zryna/issues/415).
Normal integration records the complete reviewed contract and reference packet.
The [language contract](../spec/language/BOUNDED_GENERICS_OPTION_RESULT_V1.md),
[IR boundary](../spec/ir/GENERIC_INSTANTIATION_V1.md),
[layout boundary](../spec/memory-model/GENERIC_ENUM_LAYOUT_V1.md) and
[ABI boundary](../spec/abi/GENERIC_VALUE_BOUNDARIES_V1.md) describe future
behavior. This table is a requirement, not an execution receipt.

The specified [fixed bytes and digests](../spec/memory-model/generic-enum-layout-v1-fixtures.json)
are checked by `node --test tests/m7-generic-spec-fixtures.test.mjs`. This
validates reference keys and records independently of any compiler backend.
The [instantiation fixtures](../spec/language/generic-instantiation-v1-fixtures.json)
pin mixed function/type key order and the finite versus expanding data cases;
`node --test tests/m7-generic-instance-spec-fixtures.test.mjs` checks their
specified reference key bytes and ordering.

| Owning gate | Required positive and negative evidence |
| --- | --- |
| Syntax/provider | Explicit one/two-argument functions and nominal declarations; invalid bound, malformed application, omitted/extra arguments and v4 unchanged rejection; two providers agree on source-faithful spans |
| Semantics | Cross-module same-key deduplication, distinct closed identities, mixed function/type key order, finite `Box<Box<i32>>` admitted, same-key `Node<T>` through Vec admitted, generated `Nest<Vec<T>>` expansion and function recursion rejected, opaque-bound body checking; invalid operation on `T` and wrong argument |
| Resource | Exact and first-extra for 2 parameters/arguments, 4,096 generic functions and 4,096 closed generic data instances including Option/Result, 65,536 distinct ordered instance-edge pairs (non-generic roots count only as edge origins; duplicate occurrences count once), canonical type-application depth 64 including M3 containers, key bytes 4,096, inherited 65,536 types and 256 diagnostics; checked overflow and pristine replay |
| IR | Valid closed instance calls and both standard enum variants; independent forged ID, key, substitution, payload, branch and cleanup rejection |
| Layout | Both storage targets and fixed size/offset fixtures; independent reproduction of specified type-key bytes/SHA-256 and full Option, Box, Choice and Result record digests; changed family tag, argument, ordinal, target or digest rejection; synthetic arithmetic limits |
| ABI | Internal calls accepted; public generic or Option/Result signatures rejected; no accidental export or host carrier |
| Ownership | Copy and owned payload construction, by-value/borrowed exhaustive matching, inactive payload untouched, exactly once cleanup on return and every controlled failure path |
| Target | Fixed scalar oracle for all variants and nested owned values on JavaScript, core WebAssembly and admitted Linux native target; equal trap and logical drop/release trace under fault injection |

Each implementation slice records its exact revision, platform, command, exit
status and executed/ignored counts. Documentation/schema, fixed-fixture,
ambiguity, digest and dependency checks belong to specification review.
Authenticated source and independent hostile IR are separate evidence classes.
Runtime fault traces and three-target execution are later gates and cannot be
inferred from a passing documentation check. No historical digest-pinned
inventory or current public support statement is changed by this plan.

## Complete specification acceptance packet

The [decision catalogue](../spec/language/BOUNDED_GENERICS_DECISIONS_V1.md)
completes exact admitted/excluded source forms, source-template visibility versus
executable exports, Copy/Clone, shared/exclusive match, failure disposition,
diagnostic spans/order, and #357 compatibility. All companion documents are one
reviewed contract unit with specified-only state after normal integration.
Fixture format labels `proposed-fixtures` and `candidate-only` are retained as
reference-evidence identities. They prohibit interpreting fixed semantic
outcomes or encoder checks as executed compiler/runtime conformance; they do
not leave the reviewed normative decisions open.

| Issue #415 requirement | Normative decision | Specification-review evidence | Later #416 obligation |
| --- | --- | --- | --- |
| Every admitted/excluded form | Language sections 1/3 and complete decision catalogue | Closed review schema; positive/rejected source-form catalogue; pinned TS6 parser/AST ambiguity checks | Versioned DTOs, hostile syntax, native/bootstrap semantic/diagnostic parity |
| Stable identity/order/recursion | Language section 2 and IR proposal | Existing mixed-key/depth fixtures plus diamond-module packet with exact function/data keys; canonical key decoder mutations and replay | Actual complete instance graph, substitutions and independent hostile IR |
| Enforceable exact/first-extra bounds | Language section 2 and diagnostic/resource decisions | All seven numeric reference boundaries, duplicate ordered-pair synthetic graph, canonical depth/key-byte boundary fixtures, checked arithmetic | Real producer/verifier terminal rejection, inherited source/type/IR/layout limits and no partial result |
| Option/Result representation | Layout contract and ABI boundary | Scalar records plus owned/nested full record/digest fixtures on both storage targets; closed layout-review schema and mutations | Independent sealed layout authorities, forged key/record/ordinal rejection |
| Match/ownership/failure | Decision catalogue and IR/ABI proposals | Fixed all-variant observations, inactive-payload, nested ownership, constructor/match/payload fault and borrowed cleanup requirements | Executed fixed three-target scalar/trap/release observations and fault injection |
| M3/#357 compatibility/dependency-ready plan | Decision catalogue delivery table and ROADMAP M7 bullet | Four inherited contract SHA-256 pins and reviewed dependency table | Unchanged existing bytes/diagnostics; future separate language/profile/manifest admission |

Run the complete light review gate with pinned Node.js 22.22.1 after frozen install:

```sh
node --test tests/m7-generic-spec-fixtures.test.mjs tests/m7-generic-instance-spec-fixtures.test.mjs tests/m7-generic-review-spec.test.mjs tests/m7-generic-resource-spec.test.mjs tests/m7-generic-owned-layout-spec.test.mjs tests/m7-generic-key-spec.test.mjs
pnpm docs:check
pnpm structure:check
```

The source catalogue contains syntax fragments with their names/types/owners
assumed authenticated in the documented context. Its admit/reject/code fields
are fixed semantic decisions, not observed compiler diagnostics. The module
packet supplies complete source files and fixes future `i32:14`, one imported
function instance, one imported data instance and forbidden generic ABI entries.
The tests check syntax, reference encodings, arithmetic, schema closure and
oracle preservation; they do not type-check, instantiate or run those programs.
The cleanup trace vocabulary is logical `release:<owner>` and `end-loan:<role>`;
it excludes target addresses, padding and runtime-specific handles. Later gates
must reproduce these logical events, including no event for inactive payloads.

Repository-required preflight/M0 and supported-platform hosted checks remain
submission/integration requirements. They require the coordinated build slot;
passing this light packet does not waive them or establish runtime acceptance.
