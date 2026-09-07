# Private WASI command self-check

Issue [#389](https://github.com/zryna/zryna/issues/389) implements a private command
proof arrangement over `zryna:capability-profiles/command@0.1.0`. Its bridge revision
is `zryna.command-self-check.v1`. Public WASI target selection remains unactivated.
The source implementation and test fixtures require the focused and full verification
lanes before their behavior can be accepted as execution evidence.

## Source and artifact authority

`prepare_command_self_check` compiles the supplied immutable source map through the
authenticated frontend and the existing verified `I32V1` path. It retains that source
map, matching verified program and scalar ABI, independently verified one-node
composition, and explicit empty host requests and grants. The selected host policy
is `CommandHostPolicy::deny_all()`.

The WebAssembly backend emits the original scalar core without rewriting it. A
separate private core bridge calls the selected i32 export with constant arguments,
compares the result with its constant expectation, and produces the discriminant of
`result<_, _>`. The component exposes exactly `wasi:cli/run@0.2.12`.

`ValidatedCommandComponent` has private fields and a distinct type from the existing
core-only `ValidatedWebAssemblyArtifact`. Its constructor authenticates the pinned
WIT sources and runs the complete final-byte audit before sealing. The full resolved
command import closure, including its synthesized IO/type imports, remains present.
Those imports convey no host grants.

The driver binds source identity, verified composition identity, scalar ABI, exact
core and component digests, bridge revision, WIT observations and host policy. It
revalidates the binding before creating an engine or store. No raw component bytes
can enter the production session constructor.

## Independent audit and limits

The auditor checks final bytes, exact retained core identity, every private bridge
instruction, instantiation/alias/lift/export topology, complete interface versions
and function/value types, and resource identity across aliases. Value data compares
structurally; resources require a bijection between canonical nominal identities.
The selected validator features are explicitly WASM1 and the component model.

A predecoder checks syntax counts and earlier-index references before recursive
validation or WIT decoding. It computes dependency depth in one pass, rejects
forward/cyclic references, and accounts for declarations, aliases, type exports and
repeated interface uses before allocating prospective decoded identities. The 4096
identity accounting ceiling is a conservative upper bound; it can reject a component
whose eventual decoded arena would contain fewer than 4096 types.

| Boundary | Production command | Test-only denial probe |
| --- | --- | --- |
| Complete bytes | At most 1 MiB | At most 1 MiB |
| Core modules / actual module instances | Exactly 2 / 2 | Exactly 2 / 2 |
| Core instance index entries | Exactly 2 | Exactly 3, including one synthetic single-function binding |
| Guest memories / tables | 0 / 0 | One fixed 64 KiB memory / 0 |
| Execution | At most 100,000 fuel; five-second epoch deadline | Same |

The production predecoder also bounds type dependency and alias depth to 32, type
identity accounting to 4096, aggregate type syntax entries to 16384, individual names
to 256 bytes, outer payloads to 256 and aliases to 4096. Nested component definitions
and starts are rejected. Only the reviewed run lift is admitted as a canonical operation.
The private bridge admits at most 32 i32 arguments, a 256-byte export name and 1024 bytes.
Diagnostics at the component/runtime boundary use bounded fixed messages.

Guest/fuel limits do not bound compiler/JIT memory or total process RSS. The epoch
deadline depends on the runtime and operating-system scheduler; loop fixtures must
also demonstrate deterministic fuel interruption in the execution lane.

## Denied host and teardown

Each session creates a fresh store. Every import callback and resource destructor
checks its store owner and traps before constructing a result or resource. There is
no external WASI provider or ambient context. Distinct resource definitions receive
distinct dynamic host resource types; aliases reuse only the same resource identity.

A call consumes the session's store before looking up the typed run function. Success,
typed error, lookup failure, denied call or runtime trap all destroy the store and join
the deadline watchdog. Reusing the session then fails as invalidated. `TypedFunc::call`
in the pinned runtime includes canonical post-return handling.

The `cfg(test)` probe route admits only the independently authored fixture families.
Its memory/realloc module precedes canonical lowering, an alias-only core instance
binds the single lowered function, and a second module makes the actual guest call.
The synthetic binding allocates no module instance. Probe tests are lower-boundary
controls; admission to the complete production WIT world is established separately
by the authenticated production baseline.

## Verification entry points

Focused Rust filters are `component_command` in `zryna-backend-webassembly` and
`command_runtime` in `zryna-driver`. Driver source tests require the repository-pinned
Node/TypeScript frontend setup. The fixtures cover real add source, wrapping, a wrong
expected result, source/policy binding rejection, binary/type/resource mutations,
predecode exact/first-extra controls, actual denied environment/filesystem calls,
invalid discriminants, loop interruption, store invalidation and fresh recovery.

Run the WIT audit, structure/documentation/preflight checks and applicable full M0,
M2 and M3 lanes after focused tests. Required hosted Linux and Windows checks must
refer to the final reviewed revision. A source review or an unrun fixture is not a
passing test result.
