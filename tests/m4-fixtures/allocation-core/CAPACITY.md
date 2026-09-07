# Allocation capacity evidence

#377 is stacked on the unpublished [#384](https://github.com/zryna/zryna/issues/384)
prerequisite. The probes below are prepared but have not been executed on this
integrated #377 revision. They therefore record expected independent oracles,
not passing conformance evidence.

## Governing contract and inspected revision

The integration base is #384 candidate
`79900617164aad690e2a4eb4e0c789ca0dae8f24`, whose parent is current main
`2a4ccc06b82f29a00983c2dd2356ed9d767f247f`. The #377 corpus was replayed from
the preserved `823a76d4792460b478ae239140aab6cfe92f1f9a..707ea3ec7cb897d911a6e2a35bec38ef58124062`
slice. Publication still requires #384 to merge first and this branch to align
with the resulting main before final execution proof.

| Evidence | Meaning |
| --- | --- |
| `spec/abi/OWNERSHIP_RUNTIME_V1.md:33–47` | Universal allocation/String maximum is 2,147,483,647 bytes. Target inability within that maximum is `ALLOCATION`; a language/profile violation is `CAPACITY`. |
| `crates/zryna-ownership-runtime-abi/src/lib.rs:37–41` | Compiler authority declares that byte maximum and the 1,048,576-element Vec maximum. |
| `runtime/native/ownership_runtime_v1.c` | The 64 MiB target budget returns `RT_ALLOCATION`; only requests above the universal byte maximum return `RT_CAPACITY`. |
| `crates/zryna-backend-webassembly/src/data_ownership_v1/encode.rs` | The allocator compares against the universal maximum before reporting fixed-arena exhaustion as trap 2 (`allocation`). |
| `crates/zryna-backend-webassembly/src/data_ownership_v1/encode/values.rs` | String size checking distinguishes universal overflow from the smaller target arena available after its result record. |
| `crates/zryna-backend-javascript/src/data_ownership_v1/runtime.rs` | String concat distinguishes universal capacity failure from the private 64 MiB target allocation budget. |
| `docs/M3_TARGET_BACKENDS.md:38–47` | The fixed 256-page WebAssembly arena is an intentional target budget. No ownership authority inspected defines 64 MiB as a universal maximum. |

The #384 prerequisite also owns bounded normal-success, malformed arithmetic,
repeat-after-failure recovery and former-cutoff/exact/first-extra regressions.
Those tests and previously reported #384 executions do not verify the additional
#377 source corpus on the final integrated revision.

## Bounded reproductions for #384

The native probe needs only `native-capacity.c` at its existing relative path
and the target revision's `runtime/native/ownership_runtime_v1.c`. It includes
the actual runtime and injects failure before `malloc` for every admitted
request. It checks zero output and an empty allocation registry. From a
supported Linux x86-64 repository checkout with the pinned toolchain:

```sh
probe_dir=$(mktemp -d)
gcc -std=c11 -pedantic -Wall -Wextra -Werror -O2 -fno-common \
  tests/m4-fixtures/allocation-core/native-capacity.c -o "$probe_dir/probe"
"$probe_dir/probe"
```

The WebAssembly probe needs only `capacity-inspect.mjs`, `capacity-cases.json`
and `wasm-inspection.mjs`, plus an ordinary emitted DataOwnershipV1 module.
It preserves code and instantiates a fresh fixed 16 MiB memory per row; it
passes numeric sizes to the existing allocator without creating or touching
payloads. From the repository root with pinned Node, supplying the artifact:

```sh
node tests/m4-fixtures/allocation-core/capacity-inspect.mjs "$artifact"
```

With the #377 driver harness available, this focused command prepares a tiny
source module and invokes the WebAssembly probe through the current private
driver route:

```sh
cargo test --locked -p zryna-driver --lib allocation_core_capacity_webassembly_private_allocator_boundaries -- --test-threads=1
```

The Linux runtime owns `native-capacity.c` through
`universal_capacity_and_native_exhaustion_keep_distinct_statuses`; #377 does not
compile the same probe a second time. The probes use their target's numeric
status representation: native ABI `ALLOCATION=1` and `CAPACITY=2`;
WebAssembly private trap `allocation=2` and `capacity=3`. Neither probe requires
the source resource cases or the remaining Q4–Q8 corpus.

## Remaining #377 acceptance

| Requirement | Evidence and remaining work |
| --- | --- |
| Q4–Q8 results, bytes and independent owners | Fixtures and independent oracles are integrated; JavaScript, WebAssembly and Linux native execution are `UNRUN` on this revision. |
| Failure atomicity, source retention, reverse cleanup | Bounded first-allocation, operation, growth, replacement and initialized-prefix cases are integrated; all target executions are `UNRUN` on this revision. |
| Bounds before later computation | Negative, first-extra and empty cases arm a later allocation failure; execution is `UNRUN` on this revision. |
| N7/N10 before publication | Fixed source diagnostics/spans and no-publication assertions are integrated; execution is `UNRUN` on this revision. |
| Exact/first-extra capacity | Four source resource cases and bounded target probes use the corrected #384 classification. Complete source/runtime/resource execution is `UNRUN`; raw allocator probes alone do not prove String operations at their maximum. |
| No new public ABI/package/support | Static inspection: changes remain private fixture/harness documentation and test integration. No selector, library package, public ABI or support claim is added. |

After #384 lands and this branch is aligned with the resulting main, run the full
focused `allocation_core` driver suite on each supported host, relevant M3
ownership/runtime checks and required exact-limit/resource tests. Complete
preflight, M0 and hosted Linux/Windows gates remain mandatory. Lightweight
fixture checks do not execute the compiler or establish target conformance.
Report the final revision and actual counts; earlier binaries do not verify
these new Rust tests.
