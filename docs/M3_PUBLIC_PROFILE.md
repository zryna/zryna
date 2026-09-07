# Public DataOwnershipV1 profile

Exact `--profile data-ownership-v1` activates the driver verified by Issue #89 at compiler merge
`5de1464167b39c379dfb868f90316500d8529701`. The CLI calls the same build/run implementation as
the retained candidate library API. It selects protocol v4, separately verified DataOwnershipV1
IR, both sealed layout authorities, ownership-runtime ABI v1, and canonical manifest v3 with
profile `zryna-data-ownership-v1`. The former candidate manifest label is rejected by the current
strict decoder. M1 omission, explicit M2 selection, and manifest v1/v2 retain their own contracts.

This is experimental compiler availability. Publication on zryna.com is authenticated separately
from the successful merged-commit documentation workflow; Issue #90 records the final deployment
and live verification. Neither this page nor a local test pass certifies production readiness.

## Observed surface

| Area | Supported boundary and observation |
| --- | --- |
| Aggregates and layouts | Nominal structs/enums and fixed arrays, exact construction and matching, Copy projections and owned composition admitted by the sealed verifier. `Linear32V1` and `LinuxX8664V1` fingerprints are bound in manifest v3. Storage layout is compiler-private; it is not a public aggregate ABI. |
| String | Owned UTF-8 literals, explicit clone, move, checked concatenation and replacement. Public examples observe scalar continuations after these operations, not host String values or console output. |
| Vec | Compiler-known `Vec<T>` construction, explicit structural clone, move, observation, replacement and push under the verified type/ownership rules. The fixed corpus executes Copy and String-containing Vec cleanup; checked invalid indices produce typed bounds traps. |
| Ownership | Non-Copy transfer consumes its source. Explicit `clone` creates the corresponding independent owned value or handle reference. Use-after-move and overlapping invalid ownership reject before publication. |
| Drops and cleanup | Lexical scope exit, replacement, return and controlled traps use source-bound cleanup plans. Preparation precedes replacement commit; failure retains old owners and drops only initialized prefixes. Fixed injected fault traces independently check reverse cleanup and Shared/Weak release order on all three targets. |
| Borrowing | Non-escaping lexical shared/exclusive references, verified disjoint-place access and admitted direct-call forwarding. Exclusive access excludes conflicting aliases and owner access; references cannot escape into storage or the public ABI. The beginner example observes a write through `BorrowMut<i32>` after lexical end. |
| Shared and Weak | Explicit `shared`, `clone`, `downgrade`, deterministic release, and indivisible `upgradeWeak` live/expired outcomes. Refcount overflow is a typed trap; private bounded injection tests the overflow path. Weak edges support explicit cycle breaking; tracing collection is absent. |
| Public ABI | Entry exports accept and return only exact `i32`/`bool`. Owned operations can occur in private functions or imported dependency functions whose internal exports do not become public entry exports. |

The [beginner guide](M3_GETTING_STARTED.md) contains complete sources tied to the fixed executable
corpus. The detailed [language contract](../spec/language/DATA_OWNERSHIP_V1.md),
[source-completion evidence](M3_SOURCE_COMPLETION.md), and
[conformance contract](M3_CONFORMANCE.md) distinguish source/IR proofs from executed target cases.
Historical component checkpoint pages describe their original bounded implementations; use this
integrated profile and the current sealed verifier for public selection.

## Targets and prerequisites

Use Rust 1.97.1, pnpm 11.18.0 and the exact direct Node.js 22.22.1 executable. JavaScript and core
WebAssembly build/run are verified on Linux x86-64 and Windows x64. Native build emits audited
Linux x86-64 ELF objects on those hosts. Native **run**, including `--target all` run, requires
Linux x86-64 with canonical `/usr/bin/gcc`, GCC 12–15 and GNU ld 2.38–2.46. Windows native runs
fail closed with `ZRYNA-N4002`; they do not fall back to another target. macOS execution is not
part of the required host matrix. WebAssembly is core, import-free, memory-bearing output hosted
by the pinned Node runtime; no browser bindings, WASI or Component Model capability is implied.

## Traps, resources and publication

Typed language outcomes are returned scalars or `zryna.trap.bounds-v1`,
`zryna.trap.allocation-v1`, `zryna.trap.capacity-v1`, `zryna.trap.refcount-v1`, and
`zryna.trap.utf8-v1`. A complete typed trap may commit a complete run bundle and exit 0;
read the JSON outcome. Process errors, malformed frames and host exceptions cannot stand in for
these traps. Ordinary execution does not expose the private fault-injection or drop-trace API.

Source, syntax, layout, IR, allocation, graph and runtime byte/count budgets use checked limits
and fail at their owning phase. The [full gate](M3_CONFORMANCE.md) retains exact-limit and
first-extra resource cases, including ignored proportional tests. It uses bounded synthetic state
and injection for exhaustion evidence rather than exhausting the host. This is not an unlimited
heap or arbitrary-program admission claim. Manifest v3 itself is bounded to 32 MiB and requires
canonical field order, exact source/layout/runtime identity, artifact hashes and typed observations.

Build/run expose one complete `<name>.build`/`<name>.run` directory below `.zryna/out` through a
single create-only commit. Existing files or directories are preserved. Reusing a name fails;
choose a fresh name or remove only your known generated bundle before rerunning. Invalid source,
unsupported features and incomplete execution produce no advertised bundle. Process termination
may leave a private unadvertised transaction; crash durability is not claimed.

## Deferred or unsupported

No public owned/aggregate/reference ABI, implicit ownership copy, implicit conversions, `any`,
user-defined generics, escaping borrows, borrowed imports, indirect/recursive calls, exceptions,
`break`/`continue`, tracing GC, automatic strong-cycle collection, raw pointers, FFI, threads,
custom allocators, freestanding systems, ambient filesystem/network/clock/randomness imports,
Windows native execution, WASI, Components, or production/performance certification is enabled.
Programs outside the admitted syntax and verified ownership/control-flow combinations reject;
the selector never bypasses those boundaries.
