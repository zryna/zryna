# Bounded generics public source-checkout profile

The exact selector `--profile bounded-generics-v1` adds the admitted generic, Option and
Result source surface. Both `--profile VALUE` and `--profile=VALUE` work. Omission keeps
the existing scalar default; existing control-flow and data-ownership selectors keep
their separate versions. See the [complete beginner programs](M7_GETTING_STARTED.md) and
[conformance evidence](M7_GENERIC_CONFORMANCE.md).

## Source and ABI

Definitions use the reviewed [bounded generic language contract](../spec/language/BOUNDED_GENERICS_OPTION_RESULT_V1.md)
and [decision catalogue](../spec/language/BOUNDED_GENERICS_DECISIONS_V1.md): explicit complete
arguments, bounded closed instances, generic structs/enums, Option/Result construction and
exhaustive matching, source-checked moves/loans, and deterministic controlled cleanup.
Original template bodies are checked even when unused. Unsupported arity, constraints,
identity, recursion, resources, ownership and match coverage fail in their owning phase.

Source-module template exports provide compile-time visibility. They do not become host
exports. Select a nongeneric scalar wrapper; selecting a visible template or specialization
fails with `ZRYNA-M7005` at the original exported function name. Parameters/results at the
host boundary remain exactly `bool` and `i32`. Generic/aggregate values, Option, Result,
borrow carriers and their instances have no public JavaScript object API, native C ABI,
Component/WIT representation or serialization permission. The [ABI exclusions](../spec/abi/GENERIC_VALUE_BOUNDARIES_V1.md)
remain authoritative. No inferred/partial specialization, exceptions or additional library
surface is admitted.

A template-only library with no scalar exports may build, including `--target all`.
Run requires a nongeneric scalar export.

The entrypoint roots one deterministic graph of explicit relative lowercase `.zry`
imports. File ordering does not change the selected main module. Source handles, provider
identity and source hashes remain checked through emission, execution and final commit.

## Commands and hosts

```text
zryna build ENTRY --profile bounded-generics-v1 --target javascript|webassembly|native|all --name FRESH
zryna run ENTRY --profile bounded-generics-v1 --target javascript|webassembly|native|all --export NAME [--arg=i32:DECIMAL|--arg=bool:true|--arg=bool:false ...] --name FRESH [--node ABSOLUTE_NODE]
```

The native frontend is the default. Native-frontend build, including all-target build,
needs no Node or linker. `--provider bootstrap` and `--provider compare` require the exact
Node.js 22.22.1 executable and pinned TypeScript bootstrap. Comparison rejects differing
complete verified syntax/advisories without falling back. JavaScript/WebAssembly run also
requires explicitly selected Node.js 22.22.1. Arguments use canonical signed decimals and
`bool:true`/`bool:false`, in declaration order.

| Selection | Linux x86-64 | Windows x64 |
| --- | --- | --- |
| JavaScript/core Wasm build and run | Node required for run | Node required for run |
| Linux native relocatable object build | No linker required | Cross-emitted object, no linker required |
| Native run | Documented system GNU toolchain, captured libc and loader | Rejected before target execution |
| `all` | All three ordinary targets | All three builds; run rejects native host admission |

Native run uses the existing [Linux GNU toolchain bounds](M2_NATIVE_BACKEND.md). It retains
fixed private copies of captured `libc.so.6` and `ld-linux-x86-64.so.2`; a global
`/etc/ld.so.preload` configuration is unsupported. Selected runtime hosts and every target
invocation artifact are prepared before any guest runs. Native/Node guest deadlines remain
five seconds; native compile/link retains its thirty-second bound. Host failure or timeout
produces no completed run bundle. These host limits do not reinterpret source resource traps.

Only source-checkout admission is selected here. Standalone package projects and installed
beta distributions retain their published three-profile compatibility. Components, WASI
commands, servers and foreign-library admission retain separate contracts and reject this
selector. No historical distribution is relabeled or widened.

## Outputs and outcomes

Publication creates `.zryna/out/NAME.build` or `.zryna/out/NAME.run` atomically. A collision
never overwrites an existing bundle. Source/provider drift or failed execution prevents
publication; ordinary cleanup removes the known private staging files. Unconfirmed cleanup
is reported as a cleanup failure and never as a successful command.

The separate `zryna-generic-manifest-v1.json` uses format `zryna.generic-bundle.v1`, profile
`bounded-generics-v1` and syntax protocol5. It records the actual entry module, canonical
source paths/byte counts/hashes and named import edges; instance/type/layout/runtime
fingerprints; scalar exports; target paths/kinds/lengths/hashes; and completed invocation
arguments/outcomes for run. Native records bind the ordinary relocatable object, exact
runtime template/header/rendered source and required TLS. Native run additionally records
compiled runtime, invocation object, harness, final executable and dependency-receipt hashes.
A decoded manifest is plain metadata; build/run always authenticate the original source.

Ordinary artifacts are `javascript/NAME.mjs`, `webassembly/NAME.wasm` and `native/NAME.o`.
The native artifact is a Linux relocatable object, not an executable. Private invocation
transports are not published source exports or a generic FFI.

JSON responses retain version1: `ok`, `command`, portable `manifest`, ordered `results`
and authenticated `diagnostics`. Build results are empty. A controlled source trap is a
completed observation with `ok: true` and exit0, using exactly one of:

- `zryna.trap.bounds-v1`
- `zryna.trap.allocation-v1`
- `zryna.trap.capacity-v1`
- `zryna.trap.refcount-v1`
- `zryna.trap.utf8-v1`

Unknown exceptions, engine traps, native signals, malformed status/scalar frames and host
invariants are execution failures. They are never inferred to be one of those source traps.
Wrong export/arity/type retains B2101/B2102/B2103. Opaque unsupported Clone retains M7002;
source-template host selection and aggregate ABI boundaries retain M7005. Existing phase
exit categories remain request2, source3, preparation4, execution5 and cleanup6.

Private infrastructure diagnostics C3401 and C3410–C3413 identify source-scope, metadata,
artifact/publication or typed completion inconsistencies. C4160–C4169 identify generic
native material, source/object bounds, runtime/image/provider audit, frame, staging, GNU
compile/link and host-process failures. Retain the exact source and bounded diagnostics
when reporting one. They do not grant a source resource failure or change the five trap codes.
