# Code navigation by task

Reviewed against main `9e31e6249aee44c93579c82c2e87fdafdb4b8b7d`.
Paths and commands below are navigation pointers, not a second specification or proof of execution.
Start with [CONTRIBUTING](../CONTRIBUTING.md), then the selected component's README and scoped guidance.
[zryna.workspace.json](../zryna.workspace.json) owns registration/dependencies;
[ARCHITECTURE](ARCHITECTURE.md) owns phase boundaries and [STRICT_WORKSPACE](STRICT_WORKSPACE.md) owns enforcement.
Resolve disagreements there, rather than changing this index into another authority.

Public execution is default M1 `I32V1` or explicit M2 `--profile control-flow-v1`.
M3 `DataOwnershipV1` source/layout/IR/runtime-ABI work is internal: it does not activate a public CLI profile, allocator, or target runtime.
Use [GETTING_STARTED](GETTING_STARTED.md) for running existing programs and [CLI](CLI.md) for exact command/platform contracts.

## 1. Syntax recognition, source spans, or frontend transport

- Start: [adapter README](../adapters/typescript-6/README.md), [FRONTENDS](FRONTENDS.md), and [v2](SYNTAX_PROTOCOL_V2.md), [v3 control-flow](../spec/language/CONTROL_FLOW_MODULES_V1.md), or [v4](SYNTAX_PROTOCOL_V4.md).
- Parser-side entry: `adapters/typescript-6/src/worker.mjs`, `worker-v3.mjs`, or `worker-v4.mjs`; choose the protocol explicitly.
- Trust boundary: `crates/zryna-syntax/src/v4.rs::{decode_snapshot,verify_snapshot}` (or matching older protocol); process isolation/handshake in `crates/zryna-frontend/src/worker.rs`.
- Focus: `pnpm adapter:check`, `pnpm adapter:test`, `pnpm protocol:test`; v4 changes: `pnpm m3:syntax:quick` including `worker_process` integration tests.
- Keep provider output syntax-only; name/type/module resolution belongs downstream. Finish with the full gates below.

## 2. M1/M2 names, locals, calls, branches, or module discovery

- Start: [semantics README](../crates/zryna-semantics/README.md), [M2 control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md), [module closure](M2_MODULE_CLOSURE.md).
- Entries: `crates/zryna-semantics/src/lib.rs::{SemanticInput::try_new,lower}` for M1; `src/control_flow_v1.rs::lower` for M2.
- Filesystem/module authority: `crates/zryna-driver/src/module_closure.rs::discover_module_closure`; do not put resolution into the adapter or backend.
- Focus: `cargo test --locked -p zryna-semantics`; closure tests in driver `module_closure_tests.rs`; `pnpm m2:quick` for cross-phase M2 checks.
- Source legality and backend profile acceptance are separate. Finish with the full gates below.

## 3. Internal M3 ownership, constructors, borrowing, or cleanup

- Start: [M3 composition](M3_OWNERSHIP_COMPOSITION.md), its [evidence map](M3_OWNERSHIP_COMPOSITION_EVIDENCE.md), and [Copy aggregate semantics](M3_COPY_AGGREGATE_SEMANTICS.md).
- Entry: `crates/zryna-semantics/src/data_ownership_v1.rs::{SemanticInput::try_new,lower}`; follow the selected private lowering route, not every child module.
- Aggregate preparation: `src/data_ownership_v1/owned_aggregate_lowering/{driver,constructor_preparation,constructor_resources}.rs`; Vec route: `owned_vec_lowering/{driver,constructors}.rs`.
- Preparation lifecycle: `owned_aggregate_lowering/preparation_value.rs` builds the bound plan; `preparation_execution.rs` consumes it and `preparation_local_commit.rs` commits the local destination. `constructor_preparation.rs` owns the expression walk.
- Shared typed constructor authority: `owned_constructor_plan.rs`; relevant tests live under `src/data_ownership_v1/tests/` and are registered by its parent tests module.
- Focus: `pnpm m3:data:quick`; for authority changes also `pnpm m3:owned:quick` and `pnpm m3:contract`. Find the exact neighboring constructor/borrow/cleanup test before selecting a filter.
- Mixed-construction preparation and its bounded evidence are described in the composition map above; this is internal work, not public M3 activation. Preserve both legacy and mixed diagnostic schedules, failure state and resource evidence; finish with full gates.

## 4. Type/layout identity, raw IR, or hostile verification

- Start: [layout README](../crates/zryna-layout/README.md), [IR README](../crates/zryna-ir/README.md), [M3 IR contract](M3_DATA_OWNERSHIP_IR.md).
- Layout authority: `crates/zryna-layout/src/lib.rs::{verify,VerifiedLayouts::type_by_id}`. Raw graphs are not sealed layouts.
- IR authority: `crates/zryna-ir/src/lib.rs::verify`, `src/control_flow_v1.rs::verify`, or `src/data_ownership_v1.rs::verify`; select one profile, preserving the others.
- Focus: `pnpm m3:layout`; `cargo test --locked -p zryna-ir` and `cargo test --locked -p zryna-ir --doc`; inspect the matching profile's hostile/raw fixtures.
- Include forged authority, resource boundaries, deterministic replay, and opaque-view tests as applicable. Full verification is not replaceable by producer checks.

## 5. ABI carriers or internal ownership-runtime declarations

- Start: [scalar ABI component](../crates/zryna-abi/README.md), [ownership ABI component](../crates/zryna-ownership-runtime-abi/README.md), [runtime-ABI contract](M3_OWNERSHIP_RUNTIME_ABI.md).
- Scalar authority is `crates/zryna-abi/src/`; M3 sealed declarations/transitions are `crates/zryna-ownership-runtime-abi/src/`, with its registered `include/` header.
- Focus: `cargo test --locked -p zryna-abi`; `pnpm m3:runtime-abi:quick`; use existing shared carrier/transition fixtures rather than target-local competing rules.
- Declaration/transition verification is not an implemented allocator or runtime. Finish with the full gates below.

## 6. JavaScript or core WebAssembly output

- Start: [JavaScript README](../crates/zryna-backend-javascript/README.md) or [WebAssembly README](../crates/zryna-backend-webassembly/README.md).
- Entries: each backend's `src/lib.rs::{emit,emit_control_flow}` consumes the corresponding sealed IR, never source syntax.
- Focus: `cargo test --locked -p zryna-backend-javascript` or `cargo test --locked -p zryna-backend-webassembly`; then `pnpm m2:quick` for executed cross-phase behavior.
- Publication and runtime invocation belong to the driver. Preserve byte/capability audits, scalar carriers, and deterministic output; finish with full gates.

## 7. Native lowering, object audit, linking, or process execution

- Start: [native MIR README](../crates/zryna-native-mir/README.md), [native backend README](../crates/zryna-backend-native/README.md), [M2 native contract](M2_NATIVE_BACKEND.md).
- MIR `src/lib.rs::{lower,verify}` independently seals claims; backend `src/lib.rs::{select_object_target,emit_object}` emits/audits objects. M2 has separate profile modules.
- Linking/execution: `crates/zryna-driver/src/native.rs::{discover_linux_native_toolchain,compile_native_invocation,run_native_invocation}`; process failures belong here, not in code generation.
- Focus: `cargo test --locked -p zryna-native-mir`, `cargo test --locked -p zryna-backend-native`, then the relevant driver native tests and `pnpm m2:quick`.
- Object emission and Linux GNU link/run have different prerequisites. Do not infer Windows native support from Windows Rust tests; finish with full gates.

## 8. CLI options, manifests, or create-only publication

- Start: [CLI reference](CLI.md), [driver README](../crates/zryna-driver/README.md), [manifest v2](M2_MANIFEST_V2.md).
- CLI parsing/rendering: `apps/zryna/src/main.rs::main`; orchestration: `crates/zryna-driver/src/lib.rs::compile_to_verified_ir` and `src/pipeline.rs::{build_workspace,run_workspace,build_control_flow_workspace,run_control_flow_workspace}`.
- Focus: `cargo test --locked -p zryna --test cli`, `cargo test --locked -p zryna-driver`; use pipeline fault/publication tests for transaction changes.
- Keep architecture validation first, one verified program per request, and create-only whole-bundle commit. Finish with full gates.

## 9. Workspace layout, dependency rules, CI, or gate scheduling

- Start: [STRICT_WORKSPACE](STRICT_WORKSPACE.md), `zryna.workspace.json`, and the relevant registered component README.
- Enforcement: `crates/zryna-architecture/src/lib.rs::validate_workspace`; CI: `.github/workflows/ci.yml`; local gate entrypoints: `scripts/run-preflight.mjs`, `scripts/run-m0-conformance.mjs`.
- Focus: `cargo test --locked -p zryna-architecture`; `cargo run --locked -p zryna -- architecture check`; `node --test tests/preflight.test.mjs tests/m0-conformance.test.mjs` for gate changes.
- Gate predicate/timing helpers are imported by those tests. Preserve required checks, exact commands, pins, failure propagation, and security settings; timing headroom is not a performance claim.
- Inspect actual current CI policy before editing: timeout values are intentionally not duplicated here. Finish with full gates and required hosted checks.

## 10. Guides, roadmap/contracts, or website documentation bundles

- Start: [DOCUMENTATION_BUNDLES](DOCUMENTATION_BUNDLES.md), [ROADMAP](ROADMAP.md), and the specific contract being documented.
- Source docs are under `docs/` and `spec/`; export registration is `docs/website-bundle-v1.json`; implementation is `scripts/docs/{bundle,export,check}.mjs`.
- Focus: `pnpm docs:check`; for M3 authority changes also `pnpm m3:contract`. Inspect `tests/docs-bundle.test.mjs` and the relevant contract test.
- Export is an explicit whitelist/provenance operation, not implicit inclusion of every Markdown file. CI artifact success is not evidence of website deployment.
- Keep supported behavior separate from future milestones. Finish with the required contribution gates, not a docs-only substitute.

## Required completion checks for every route

The focused commands above are editing aids, not submission evidence by themselves. Follow current CONTRIBUTING and the checked gate registries:

```sh
pnpm install --frozen-lockfile
pnpm preflight
pnpm m0:check
```

Use the repository-pinned toolchains. Keep Linux and Windows M0/required hosted checks mandatory before merge.
Quick filters omit some expensive/ignored boundaries; retain the complete gate's required ignored-test execution and doctests.
For a custom filter, first inspect `-- --list` and verify actual nonzero matching tests; never report discovery as execution.
Record the exact revision, command, exit status, and executed/ignored counts. Do not reuse stale binaries as evidence for changed source.

## Keeping this index current

When moving a file or changing an entrypoint, update its route and relative links in the same change.
Recheck the component's manifest registration/dependencies, existing README, neighboring tests, and gate script references.
Prefer stable entry symbols over line numbers and dynamic test counts. Search within the selected component before expanding to callers.
This file guides navigation; it grants no new source admission, dependency edge, public profile, or reduced verification requirement.
