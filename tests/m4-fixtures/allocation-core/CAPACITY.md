# Allocation capacity evidence

#377 was developed on the reviewed [#384](https://github.com/zryna/zryna/issues/384)
publication candidate. The integrated fixtures and probes have been executed
in the pinned Linux proof environment described below. Hosted Linux and Windows
checks remain separate merge evidence.

## Governing contract and inspected revision

The local proof base is the reviewed #384 candidate
`ef3863f78ffe144c3145ece7f03c9ee09d5164da`. The #377 patches were verified
unchanged after replaying them onto that base. Publication still requires a
byte-identical alignment with the resulting main revision.

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

## Observed #377 evidence

| Requirement | Evidence and remaining work |
| --- | --- |
| Q4–Q8 results, bytes and independent owners | The focused driver suite passed for JavaScript, WebAssembly and Linux native. The WebAssembly Q4 genuine artifact passed its actual drop-handle/payload inspection, while a real clone-helper alias mutant was rejected. |
| Failure atomicity, source retention, reverse cleanup | Bounded first-allocation, operation, growth, replacement and initialized-prefix cases passed across the selected targets. String clone/concat recovery and Vec recovery were each observed after failure; small traced rows bind cleanup order and retained sources. |
| Bounds before later computation | Negative, first-extra and empty cases passed with the bounds trap winning before the armed later allocation failure. |
| N7/N10 before publication | Fixed source diagnostics/spans and no-publication assertions passed before target dispatch. |
| Exact/first-extra capacity | The source resource rows and bounded target probes passed with the corrected #384 classification. Maximum-size source rows run with tracing disabled because their derived temporary cleanup exceeds the fixed observation-frame capacity; they prove exact outcomes, not a full-limit cleanup trace. Raw allocator probes do not prove String operations at their maximum. |
| No new public ABI/package/support | Static inspection: changes remain private fixture/harness documentation and test integration. No selector, library package, public ABI or support claim is added. |

The pinned local Linux proof used Rust 1.97.1, Node 22.22.1 and pnpm 11.18.0
with a frozen install, at most three build/test jobs, and serial execution for
the focused resource fixtures. The static fixture corpus passed 6/6, the
focused driver suite passed 10/10, the complete driver package passed 139/139
plus 2/2 doctests, and preflight passed all 12 ordered checks. Publication must
still bind the final main-aligned revision to the required M0/M2/M3 and hosted
Linux/Windows results. Lightweight fixture checks alone do not execute the
compiler or establish target conformance.
