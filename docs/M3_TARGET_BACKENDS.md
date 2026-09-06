# Internal M3 target backends

Issues #84–#86 add three internal consumers of the same sealed `DataOwnershipV1` authority. They
do not add a driver route, artifact publication, public aggregate ABI or
`--profile data-ownership-v1` selection.

## Shared trust boundary

Each entrypoint accepts `zryna_ir::data_ownership_v1::VerifiedProgram` plus the separately sealed
`VerifiedOwnershipRuntimeAbi`. It compares the type-universe identity and exact Linear32/Linux
x86-64 layout fingerprints before emitting anything. Raw IR and independently selected runtime
declarations cannot enter. The closed backend instruction/terminator view preserves typed value,
place, borrow, call, edge and cleanup identities without revealing raw verifier claims.

All target output is deterministic for the same authorities. JavaScript and WebAssembly cap a
complete artifact at 32 MiB. Native MIR retains the upstream function/value/place budgets and
rejects raw claims with at most 256 stable diagnostics.

## JavaScript representation

`emit_data_ownership` emits a self-contained ESM string. Primitive `bool` and `i32` remain native
primitive values behind exact scalar wrappers. String, aggregate, enum, Vec, Shared and Weak
values use private null-prototype-independent records with closed numeric tags. Generated code
uses explicit state dispatch for CFG edges, scratch values for parallel block arguments, explicit
place traversal, lexical borrow records, checked indexes, deep structural clone, reverse drop and
strong/weak transitions. A failed recursive clone reverse-drops its completed prefix, and emitted
fallible instructions run their sealed cleanup actions before propagating the trap. It uses no
eval, dynamic Function, global object, package, process,
browser, DOM or dynamic import capability. Engine GC may reclaim unreachable implementation
records, but Zryna release order and refcount transitions are explicit and do not depend on GC.

Stable failures are `ZRYNA-J3001` for authority/profile inconsistency, `ZRYNA-J3002` for formatting
failure and `ZRYNA-J3003` for the artifact bound. Runtime traps use private `ZRYNA-R3*` identities.

## Core WebAssembly representation

`emit_data_ownership` emits a core module with one private fixed-maximum Linear32 memory, one
private checked invocation arena, exact sealed function exports and no imports. Aggregate fields
and fixed-array projections use sealed byte offsets; values and borrow addresses are explicit
`i32` carriers. Type-indexed helpers implement recursive clone/drop, String/Vec storage, checked
indexing and strong/weak transitions. Scalar export wrappers reset the arena before and after each
successful invocation, and before a new invocation after a trap. Completed bytes pass the pinned
WebAssembly validator and a second fail-closed audit.
The audit rejects imports, tables, start, elements, data, tags, custom/unknown sections, non-function
exports, indirect calls and ambient memory growth. The fixed 256-page maximum and checked allocator
make address addition and exhaustion deterministic without granting WASI, Component Model, GC,
threads, SIMD, filesystem or network capability.

Stable failures are `ZRYNA-W3001` for authority/index inconsistency, `ZRYNA-W3002` for invalid
bytes, `ZRYNA-W3003` for the byte bound and `ZRYNA-W3004` for capability-audit rejection.

## Linux x86-64 native MIR

`zryna_native_mir::data_ownership_v1::lower` maps the sealed Universal program one-for-one into
raw target MIR and immediately invokes an independent verifier. MIR carries exact Linux layout
records, address-derived places, dense typed values, blocks and parallel edges, cleanup kinds,
direct callees and only the 17 authenticated runtime symbols. The verifier rechecks the authority,
complete layout records, canonical symbols, dense identities, operand existence, call targets,
edge arity including synthesized Weak-upgrade success ownership, layout offsets and cleanup type
compatibility before returning an opaque `VerifiedMirModule`.

The public `raw` module and `lower_unverified` exist for hostile-claim testing and grant no codegen
authority. Object emission, relocation/symbol audit, runtime objects, linking and execution remain
the responsibility of #87.

## Focused verification

- JavaScript tests execute aggregate arithmetic, String/Vec helpers and lexical borrow write-through
  in Node, inject recursive clone failure, compare repeated source bytes and audit forbidden
  capabilities.
- WebAssembly tests execute the aggregate oracle from validated bytes, inspect its private-memory
  and export boundary, compare repeated bytes and validate a private borrow-bearing module.
- Native MIR tests compare repeated lowering, inspect opaque views and replay forged type/layout,
  address, cleanup-reference and runtime-symbol claims to exact rejection codes.

Full repository preflight, M0/M2 regressions and Linux/Windows hosted CI remain mandatory merge
evidence. #87–#90 remain downstream and public M3 selection stays unavailable.
