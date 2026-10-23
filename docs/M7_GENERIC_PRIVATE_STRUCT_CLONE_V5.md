# Private structural Struct Clone V5

This private continuation implements whole-value `clone(namedStruct)` through a distinct
owned-IR V5 issuer. It does not activate public generic support or complete #416. The accepted
bounded-generics specification remains the complete contract, including its remaining
containers, arrays, handles, borrowed operations, diagnostics and public conformance gates.

Original source capability is checked before generated discovery. Clone is derived from every
actual substituted stored field and every possible enum payload. An opaque stored parameter
cannot acquire Clone through a closed specialization or an inactive variant. Phantom arguments
are still validated, but do not supply stored ownership: a phantom Struct containing only i32
can be cloned without assuming Clone for its unused parameter. A fixed String member remains
affine and cloneable. The V5 Clone query has its own bounded transactional cache; V4 fact and
cache costs remain unchanged.

The wire header is `ZRYNA-GENERIC-OWNED-IR-V5\0`, with version 5. New owned tag 10 contains only
`CloneStruct { value: u32 }`. Tags 1–9 preserve their meanings. Every older codec rejects tag 10,
and old source issuers remain closed to Struct Clone. Decoding produces immutable untrusted
claims. Independent source, type, dual-layout, runtime and ownership checks must succeed before
an opaque operation-bound clone recipe can reach a backend. Layout or zero drop cost alone
cannot grant source Clone capability.

Clone reads a complete live owner without consuming it. A live shared loan is permitted; moved
or exclusively loaned sources reject. Fields are cloned in declaration order. Enum traversal
clones only the active payload, while source proof covers all alternatives. Each successful
String clone has a fresh allocation identity. Copy regions require no String allocation, and
aggregate carriers introduce no runtime allocation or new ABI symbol.

On allocation failure, the backend first releases successful destination leaves in reverse
order. It then applies the verified outer failure cleanup to pre-existing owners and loans.
The incomplete result never becomes a complete owner. Runtime cursor/log state records actual
successful initialization and cannot be supplied as a wire or source proof. Wasm and native
check the String call status before reading its output record.

Seven fixed source cases authenticate complete native and TypeScript provider packets. One
retained V5 program per case feeds JavaScript, core Wasm and Linux x86-64 emission. The driver
checks 147 runtime observations: 49 per target, including allocation failures, exact source and
destination identities, reverse release order and pristine retries. Native has 21 separately
spawned controlled failure cases. These totals are separate from the existing V4 suites.

Compact generic Struct/nominal-enum DAG fixtures reach the genuine V5 seal before backend
qualification. N4 emits on all three targets. N12 emits JS/Wasm and rejects with native N7001.
N16 independently rejects with J2003, W4001 and N7001 at the respective backend phases. These
are emission and resource-phase checks, not additional runtime observations. The existing
owner, traversal, wire, target and artifact ceilings are unchanged.

The fixed inputs and expectations live in `tests/m7-generic-struct-clone-v5` and
`tests/m7-generic-struct-clone-compact-dag`. The driver tests are
`generic_struct_clone_v5` and `generic_struct_clone_compact_dag`. Full supported-platform and
public acceptance remain required before the issue can close.
