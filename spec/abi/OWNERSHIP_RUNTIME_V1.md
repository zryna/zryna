# Ownership runtime ABI v1

Status: normative planning contract for the memory-bearing portion of `DataOwnershipV1`. No
runtime crate, allocator, helper import, public profile, or host capability is implemented by this
document.

## 1. Authority and separation

The ABI identifier is `zryna-ownership-runtime-v1`. It is the only contract between verified M3
code generation and target runtime implementations for allocation, growth, release, owned String,
owned Vec, Shared, Weak, and controlled failure. It consumes sealed layouts from
[`AGGREGATE_LAYOUT_V1.md`](../memory-model/AGGREGATE_LAYOUT_V1.md) and implements the state
transitions specified by
[`DATA_OWNERSHIP_V1.md`](../language/DATA_OWNERSHIP_V1.md).

Compiler phases may depend on ABI declarations and opaque verified views. Runtime implementations
must not depend on syntax, semantics, Universal IR, a backend, or the driver. Backends may emit
only operations and target mappings sealed by this ABI; they may not link an implementation into
another backend. The driver alone selects, audits, and composes a target implementation.

This is a compiler-private runtime ABI, not a public aggregate host ABI. It never exposes Rust
`String`, `Vec`, `Rc`, `Arc`, `Weak`, trait objects, allocator objects, unwinding, layout metadata,
or standard-library calling conventions. No source program can name a helper symbol, pointer,
reference count, allocation address, capacity, status word, or runtime control block.

## 2. Universal runtime limits

These limits apply identically before any target-specific call:

| Limit                                       |                           Value |
| ------------------------------------------- | ------------------------------: |
| one dynamic allocation                      |           `2,147,483,647` bytes |
| String byte length/capacity                 |                 `2,147,483,647` |
| Vec element length/capacity                 |                     `1,048,576` |
| allocation alignment                        | one of `1`, `2`, `4`, `8` bytes |
| strong handle count                         |                 `4,294,967,295` |
| weak count including implicit weak owner    |                 `4,294,967,295` |
| live allocations per invocation             |                     `1,048,576` |
| allocation/growth operations per invocation |                     `1,048,576` |
| runtime status transitions per invocation   |                     `4,194,304` |

Every length-to-byte conversion, capacity growth, control-block size, payload offset, and base plus
offset calculation uses checked unsigned arithmetic. A valid static layout does not prove a dynamic
address valid. A request exceeding a language/profile limit returns `CAPACITY`; a request within
the limit that cannot be satisfied by the selected target returns `ALLOCATION`. The distinction is
stable.

The runtime is single-threaded and non-reentrant in v1. It imports no clock, randomness,
filesystem, network, environment, locale, thread, atomic, callback, finalizer, or exception
capability. A later threaded ABI must use a new identifier and specify memory ordering.

## 3. Status and result contract

Every fallible operation returns one exact status before generated code commits a source-visible
state transition:

| Numeric status | Name            | Meaning                                                                       |
| -------------: | --------------- | ----------------------------------------------------------------------------- |
|            `0` | `OK`            | operation succeeded and all result fields are initialized                     |
|            `1` | `ALLOCATION`    | valid allocation request could not be satisfied                               |
|            `2` | `CAPACITY`      | checked size/capacity arithmetic or a profile maximum failed                  |
|            `3` | `REFCOUNT`      | a strong or weak increment would exceed `u32::MAX`                            |
|            `4` | `UTF8`          | input bytes are not well-formed UTF-8                                         |
|            `5` | `EXPIRED`       | Weak upgrade observed strong count zero; this is a branch result, not a trap  |
|          `255` | `ABI_VIOLATION` | forged pointer, invalid state, invalid status/result shape, or runtime defect |

Unknown numeric statuses are `ABI_VIOLATION`. Except for `EXPIRED`, nonzero language statuses map
to the exact controlled traps in `DataOwnershipV1`. `ABI_VIOLATION` is a fail-closed host/runtime
failure and must never be relabeled as a source trap.

On a non-`OK` result, every output pointer and output handle is zero, no input ownership transfers,
no count changes, and every existing allocation remains byte-for-byte and state-for-state valid.
`EXPIRED` likewise performs no increment and returns no handle. Operations that are infallible for
verified inputs may still report `ABI_VIOLATION` when the implementation detects corruption; such
an outcome aborts the trusted invocation boundary.

## 4. Raw storage operations

The logical operations are:

```text
allocate(byteSize, alignment) -> (status, pointer)
grow(pointer, oldByteSize, newByteSize, alignment) -> (status, newPointer)
release(pointer, byteSize, alignment) -> status
```

Pointers are opaque target-private unsigned addresses. Zero is the only null value. The exact
rules are:

- `allocate(0, alignment)` returns `OK, 0` and consumes no live-allocation budget;
- a successful nonzero allocation returns a nonzero address aligned to `alignment` and disjoint
  from every other live allocation;
- new bytes are uninitialized and may not be read until generated initialization writes them;
- `grow` with unchanged size returns the original pointer and performs no allocation;
- `grow` to zero releases a valid old allocation and returns `OK, 0`;
- successful growth preserves exactly `min(oldByteSize, newByteSize)` bytes and invalidates the old
  pointer only when `OK` is returned;
- failed growth preserves the old allocation and pointer exactly;
- `release(0, 0, alignment)` is a no-op returning `OK`;
- release of a valid nonzero allocation invalidates it exactly once and does not allocate; and
- wrong size, wrong alignment, non-base pointer, double release, overlapping live allocation, or
  use after successful growth is `ABI_VIOLATION`.

Generated code owns initialized-range metadata. The raw allocator never guesses which fields or
elements require drop. Before grow or release, generated code proves the exact allocation base,
size, alignment, and initialized prefix through sealed layout and ownership authority.

## 5. Owned String ABI

The stored handle is the exact `(pointer, byteLength, capacity)` layout for the selected target.
Its invariants are:

- `0 <= byteLength <= capacity <= 2,147,483,647`;
- empty String is canonically `(0, 0, 0)`;
- nonempty String has a nonzero pointer to exactly `capacity` owned bytes with alignment one;
- bytes `0..byteLength` are initialized and form one well-formed UTF-8 sequence;
- bytes `byteLength..capacity` are spare and inaccessible; and
- no two unique String values own the same allocation.

The logical operations are:

```text
stringFromUtf8Copy(bytes, byteLength) -> (status, String)
stringClone(source) -> (status, String)
stringConcat(left, right) -> (status, String)
stringRelease(value) -> status
```

`stringFromUtf8Copy` validates the complete input before returning a String. Compiler-authenticated
literal bytes may use a separately verified static path but must have the same value semantics.
Clone and concatenation allocate disjoint storage and copy exact bytes. Concatenation checks the
sum before allocation. A failure returns no String and leaves every input owned and unchanged.
Release invalidates the handle exactly once and is infallible for a verified value.

Length is bytes. This ABI provides no code-point iterator, code-unit view, numeric indexing,
normalization, locale operation, null terminator, borrowed host string, or external buffer adoption.
Capacity and allocation address are not source-observable.

## 6. Owned Vec ABI

The stored handle is `(pointer, elementLength, capacity)` in the selected target word width. For a
sealed element layout with positive `stride` and `alignment`:

- `0 <= elementLength <= capacity <= 1,048,576`;
- `checkedMultiply(capacity, stride)` is within the dynamic-byte maximum;
- empty Vec is canonically `(0, 0, 0)`;
- nonempty capacity has a nonzero pointer to exactly `capacity * stride` bytes aligned to the
  element alignment;
- elements `0..elementLength` are fully initialized verified `T` values;
- slots `elementLength..capacity` are uninitialized and never read or dropped; and
- no two unique Vec values own the same allocation.

`Vec<T>` with zero-sized `T` is rejected by language semantics before this ABI. Runtime helpers do
not accept element layout from source or unverified backend constants; they consume one sealed
layout record.

The logical raw operations are:

```text
vecAllocate(elementLayout, requiredCapacity) -> (status, VecStorage)
vecReserve(elementLayout, storage, requiredLength) -> (status, VecStorage)
vecReleaseStorage(elementLayout, storage) -> status
```

When growth is required, capacity is deterministic:

```text
candidate = 4                         when oldCapacity = 0
candidate = checkedMultiply(oldCapacity, 2) otherwise
repeat doubling while candidate < requiredLength
newCapacity = max(candidate, requiredLength)
```

Each intermediate must remain within both element-count and byte-size limits. The implementation
may allocate more physical allocator bookkeeping but may not report another logical capacity. This
freezes fault-injection order and prevents targets from taking different controlled-capacity paths.

Element initialization, move, and drop are compiler-generated type-specific operations, not runtime
callbacks or function pointers. Clone glue exists only when the verified element type has the
structural `Clone` capability defined by `DataOwnershipV1`; it recursively clones in ascending
element order and is absent otherwise. Push first evaluates and owns the argument, reserves
without changing the logical Vec on failure, writes the complete value into the old-length slot,
then increments length. Generated drop visits initialized elements in descending index order before
calling `vecReleaseStorage`. A deep clone allocates one destination and clones/copies elements in
ascending order; on failure it drops the completed destination prefix in reverse order.

## 7. Shared and Weak control blocks

Shared and Weak use one runtime-owned control allocation per payload. Its stored prefix is exact:

```text
offset 0: u32 strongCount
offset 4: u32 weakCount
payloadOffset = alignUp(8, payloadAlignment)
controlAlignment = max(4, payloadAlignment)
controlSize = alignUp(payloadOffset + payloadSize, controlAlignment)
```

Counts are little-endian. `weakCount` equals explicit live Weak handles plus one implicit weak owner
while `strongCount > 0`. The payload is initialized exactly when `strongCount > 0`. Empty or
zero-sized payloads still use a nonzero control-block allocation so live handles are never null.
Control size and offset are computed by the shared layout authority.

Creating `Shared<T>` consumes one uniquely owned initialized `T`, allocates the control block,
stores counts `(1, 1)`, then moves the payload into the payload area. Allocation failure leaves the
source value owned until generated trap cleanup.

The exact transitions are:

```text
strongClone(control) -> status
weakDowngrade(control) -> status
weakClone(control) -> status
weakUpgrade(control) -> status
strongReleaseBegin(control) -> (status, isLastStrong)
strongReleaseFinish(control) -> status
weakRelease(control) -> (status, deallocated)
```

- `strongClone` and successful `weakUpgrade` increment strong count after checking it is neither
  zero nor `u32::MAX`.
- `weakDowngrade` and `weakClone` increment weak count after checking `u32::MAX`.
- `weakUpgrade` returns `EXPIRED` without mutation when strong count is zero. `OK` means exactly one
  new Shared handle was created. In the single-threaded ABI, observation and increment are one
  indivisible operation.
- `strongReleaseBegin` decrements a positive strong count. It reports last-strong only for the
  transition `1 -> 0`. On that transition the control block remains reserved and no other runtime
  transition is allowed until finish.
- generated code drops the payload exactly once after last-strong begin, then
  `strongReleaseFinish` removes the one implicit weak owner. If weak count becomes zero, finish
  releases the control block.
- non-last strong release returns without touching weak count or payload.
- `weakRelease` decrements one explicit weak handle. It releases the control block only when the
  result is zero, which implies strong count is already zero and no implicit weak owner remains.

Release operations do not allocate and are infallible for verified transitions. Zero counts,
finish without a pending last-strong transition, payload access at strong zero, wrong type/layout
fingerprint, or deallocation with a live handle is `ABI_VIOLATION`.

Shared payloads are immutable in v1. Counts and addresses are not observable. The v1 language has
no operation that constructs a strong-reference cycle; the runtime rejects a forged cyclic control
graph as `ABI_VIOLATION`. It performs no tracing or cycle discovery. Weak references provide
non-owning observation, not interior mutation or a cycle-construction escape hatch.

## 8. Drop, partial initialization, and failure atomicity

Every fallible runtime call is prepare-before-commit. Generated code changes its ownership state
only after validating an `OK` result and complete result shape. On `ALLOCATION`, `CAPACITY`,
`REFCOUNT`, or `UTF8`, it retains the original trap identity, runs the verified cleanup plan for all
currently initialized live values, then reports the controlled trap. `EXPIRED` takes the failure
successor of `WeakUpgradeBranch` without cleanup or trap.

Runtime release operations are the leaves of cleanup and must not allocate, invoke source code,
throw, unwind, or change the original trap identity. A detected ABI violation stops the trusted
boundary; it is never converted into a successful release or language trap.

Partial aggregate and container initialization is tracked outside the raw allocator. A trap while
building a struct drops completed fields in reverse declaration order. A trap while cloning or
building a Vec drops the completed element prefix in descending order, then releases storage. A
String operation publishes no partially valid UTF-8 handle. Last-strong payload drop completes
before the implicit weak owner is released.

External termination, process kill, engine failure, hardware fault, or hostile memory corruption
is outside controlled failure and carries no cleanup guarantee.

## 9. JavaScript mapping

JavaScript output uses a generated module-private, frozen helper table whose identity is bound into
the artifact manifest. Helpers use private branded records and dense storage; no helper, brand,
backing buffer, count, capacity, pointer token, or trap sentinel is exported. Source property names
never index runtime records, and no object inherits an attacker-controlled prototype.

The JavaScript mapping returns frozen null-prototype result records containing exact numeric status
and private result fields. It performs explicit integer/range checks before engine allocation. A
private monotonic allocation identity may model pointers for audit and fault injection, but it is
not a language address. Engine garbage collection may reclaim unreachable implementation objects;
it does not replace compile-time move checks, required explicit clone transitions, reference-count
state, exact drop traces, or failure atomicity.

No ambient global, `eval`, `Proxy`, user callback, finalization registry, weak JavaScript reference,
prototype method, truthiness, host exception text, or implicit string coercion defines ABI behavior.
Unexpected engine exceptions are host failures. Controlled failures use returned statuses until
verified cleanup completes; only the private entry wrapper may translate the final trap identity
into the driver observation channel.

## 10. Core WebAssembly mapping

The memory-bearing M3 WebAssembly artifact defines its own core WebAssembly 1.0 linear memory. It
has minimum one page, maximum 32,768 pages, reserves address zero as null, and places the heap base
at the first address at or after static data aligned to eight. It imports no memory, allocator,
table, function, global, WASI capability, clock, randomness, environment, thread, or exception.
Memory is not exported by the public M3 scalar boundary.

Runtime pointers, lengths, capacities, and statuses use `i32` lanes interpreted as unsigned where
specified. A fallible operation that returns status plus pointer uses one `i64` internal result:
the low 32 bits are the pointer and the high 32 bits are the numeric status. Generated code must
extract both lanes and reject a nonzero pointer for non-`OK` status. Operations returning only a
status use `i32`. These internal signatures are not source functions or public exports.

String and Vec results use one caller-owned 12-byte, four-aligned linear-memory record with exact
little-endian lanes `(pointer:u32, length:u32, capacity:u32)`. Boolean outcome records are one
four-byte lane containing canonical zero or one. Generated code zeroes every result record before
the call; a non-`OK` operation must leave it zero. The exact internal core-Wasm signatures, in ABI
operation order, are:

```text
allocate(i32 byteSize, i32 alignment) -> i64 packedStatusPointer
grow(i32 pointer, i32 oldByteSize, i32 newByteSize, i32 alignment) -> i64 packedStatusPointer
release(i32 pointer, i32 byteSize, i32 alignment) -> i32 status
stringFromUtf8Copy(i32 bytes, i32 byteLength, i32 outString) -> i32 status
stringClone(i32 pointer, i32 byteLength, i32 capacity, i32 outString) -> i32 status
stringConcat(i32 leftPointer, i32 leftLength, i32 leftCapacity,
             i32 rightPointer, i32 rightLength, i32 rightCapacity,
             i32 outString) -> i32 status
stringRelease(i32 pointer, i32 byteLength, i32 capacity) -> i32 status
vecAllocate(i32 elementLayoutId, i32 requiredCapacity, i32 outStorage) -> i32 status
vecReserve(i32 elementLayoutId, i32 pointer, i32 elementLength, i32 capacity,
           i32 requiredLength, i32 outStorage) -> i32 status
vecReleaseStorage(i32 elementLayoutId, i32 pointer, i32 elementLength,
                  i32 capacity) -> i32 status
strongClone(i32 control) -> i32 status
weakDowngrade(i32 control) -> i32 status
weakClone(i32 control) -> i32 status
weakUpgrade(i32 control) -> i32 status
strongReleaseBegin(i32 control, i32 outIsLastStrong) -> i32 status
strongReleaseFinish(i32 control) -> i32 status
weakRelease(i32 control, i32 outDeallocated) -> i32 status
```

`elementLayoutId` is a dense ID in the artifact-authenticated sealed layout table, never a source
integer or address. All result-record addresses must be nonzero, aligned, in bounds, disjoint from
live owned storage for the call, and excluded from the returned owned allocation.

The implementation uses checked unsigned address arithmetic and explicit bounds checks before
every load, store, copy, fill, grow, or control-block access. `memory.grow` failure maps to
`ALLOCATION`; maximum/page/byte arithmetic failure maps to `CAPACITY`. Raw WebAssembly traps,
out-of-bounds accesses, invalid UTF-8 reads, or unreachable instructions are runtime/backend
failures unless generated verified code deliberately reports a language trap after cleanup.

The completed module audit must prove exact memory limits, no imports, no exported memory, the
allowlisted internal runtime operation signatures, no tables/elements/tags, and only the separately
frozen instruction set needed by verified M3 code and runtime. Wasm GC, reference types, threads,
SIMD, memory64, multiple memories, WASI, WIT, and Component Model are unavailable.

## 11. Linux x86-64 native mapping

Native runtime entrypoints use the Linux x86-64 System V C calling convention and the reserved
symbol prefix `zryna_rt_o1_`. The prefix is disjoint from scalar public exports
`zryna_v1_e_` and compiler-private type glue. Exact fixed-width carriers are `uint32_t` status,
`uint32_t` alignments and counts, `uint64_t` sizes/lengths/capacities, and `uintptr_t` opaque
pointers. Every fallible pointer result uses a caller-owned `uintptr_t* out` initialized to zero;
the function returns status. It may write nonzero output only when returning `OK`.

The native ABI freezes these record bytes and declarations (standard integer typedef spellings are
illustrative; widths, field order, offsets, size, alignment, and parameter order are normative):

```c
typedef struct {
  uintptr_t pointer;       /* offset 0 */
  uint64_t length;         /* offset 8 */
  uint64_t capacity;       /* offset 16 */
} zryna_rt_o1_handle;      /* size 24, alignment 8 */

uint32_t zryna_rt_o1_allocate(uint64_t byte_size, uint32_t alignment,
                              uintptr_t *out_pointer);
uint32_t zryna_rt_o1_grow(uintptr_t pointer, uint64_t old_byte_size,
                          uint64_t new_byte_size, uint32_t alignment,
                          uintptr_t *out_pointer);
uint32_t zryna_rt_o1_release(uintptr_t pointer, uint64_t byte_size,
                             uint32_t alignment);
uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t byte_length,
                                           zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,
                                  zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,
                                   const zryna_rt_o1_handle *right,
                                   zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value);
uint32_t zryna_rt_o1_vec_allocate(uint32_t element_layout_id, uint64_t required_capacity,
                                  zryna_rt_o1_handle *out_storage);
uint32_t zryna_rt_o1_vec_reserve(uint32_t element_layout_id,
                                 const zryna_rt_o1_handle *storage,
                                 uint64_t required_length,
                                 zryna_rt_o1_handle *out_storage);
uint32_t zryna_rt_o1_vec_release_storage(uint32_t element_layout_id,
                                         const zryna_rt_o1_handle *storage);
uint32_t zryna_rt_o1_strong_clone(uintptr_t control);
uint32_t zryna_rt_o1_weak_downgrade(uintptr_t control);
uint32_t zryna_rt_o1_weak_clone(uintptr_t control);
uint32_t zryna_rt_o1_weak_upgrade(uintptr_t control);
uint32_t zryna_rt_o1_strong_release_begin(uintptr_t control,
                                          uint32_t *out_is_last_strong);
uint32_t zryna_rt_o1_strong_release_finish(uintptr_t control);
uint32_t zryna_rt_o1_weak_release(uintptr_t control, uint32_t *out_deallocated);
```

Every handle out-record is zeroed before the call and remains all-zero on non-`OK`. Boolean out
lanes are zeroed first and may become canonical one only on `OK`. Input and output records may not
alias. The native verifier rejects any field offset, record size/alignment, carrier width, parameter
order, const/input role, or symbol signature that differs from this table.

The required symbol families are:

```text
zryna_rt_o1_allocate
zryna_rt_o1_grow
zryna_rt_o1_release
zryna_rt_o1_string_from_utf8_copy
zryna_rt_o1_string_clone
zryna_rt_o1_string_concat
zryna_rt_o1_string_release
zryna_rt_o1_vec_allocate
zryna_rt_o1_vec_reserve
zryna_rt_o1_vec_release_storage
zryna_rt_o1_strong_clone
zryna_rt_o1_weak_downgrade
zryna_rt_o1_weak_clone
zryna_rt_o1_weak_upgrade
zryna_rt_o1_strong_release_begin
zryna_rt_o1_strong_release_finish
zryna_rt_o1_weak_release
```

Issue #80 materializes this exact declaration set as a checked C header before any object is linked;
it may not choose a different parameter order or out-record representation. This contract does not
invent an implementation-specific allocator context or expose layout records as C structs. Type
layouts and constant metadata are compiled into verified call sites.

The runtime object and final executable are parsed and audited. Undefined symbols must equal an
explicit implementation allowlist; arbitrary libc, dynamic loading, environment access, threads,
unwinding, C++ runtime, Rust runtime, and constructor sections fail. Runtime symbols must have exact
binding, visibility, section, convention, and one definition. Compiler-private drop/clone glue uses
local hidden symbols derived from sealed type IDs and never enters the runtime namespace.

Allocator choice is an implementation detail only if it satisfies this ABI and the object audit.
Native allocation addresses and capacity bookkeeping are not reproducibility or language outputs.

## 12. Runtime handshake and artifact audit

Every memory-bearing artifact binds these authorities before execution:

- exact ABI identifier `zryna-ownership-runtime-v1`;
- exact storage target and aggregate-layout fingerprint;
- exact runtime implementation identifier and content SHA-256;
- exact set of required logical operations;
- exact String/Vec/control-block layout records; and
- exact fault-injection mode, which is absent in production artifacts.

The driver rejects a missing, extra, duplicate, unknown, mismatched, or reordered authority before
publishing an artifact. A backend cannot silently inline a behavior that differs from the ABI;
inlining is valid only when the audited artifact proves semantic equivalence and retains the same
manifest identity.

Fault-injection builds are test-only and deterministically fail the selected allocation, growth,
clone, or count increment ordinal before mutation. They are never published. The conformance suite
uses them to prove identical trap identity, preserved input state, initialized-prefix cleanup,
drop order, reference-count overflow, expired weak upgrade, and absence of partial artifacts across
JavaScript, WebAssembly, and native execution.

## 13. Stable diagnostics and budgets

Runtime ABI verification reserves:

| Code          | Meaning                                                               |
| ------------- | --------------------------------------------------------------------- |
| `ZRYNA-R3001` | missing, duplicate, unknown, or mismatched ABI operation or symbol    |
| `ZRYNA-R3002` | invalid carrier, status, result, pointer, handle, or state transition |
| `ZRYNA-R3003` | layout target or fingerprint mismatch                                 |
| `ZRYNA-R3004` | forbidden runtime dependency, import, export, or ambient capability   |
| `ZRYNA-R3005` | runtime object/module structural audit failure                        |
| `ZRYNA-R3006` | invalid fault-injection or cleanup evidence                           |
| `ZRYNA-R3201` | deterministic runtime-verification budget exhausted                   |

Verification admits at most 256 runtime operations, 4,096 target symbols/functions, 65,536 sealed
type-layout references, 65,536 relocation/call edges, 16 MiB of runtime object/module bytes, and
256 retained diagnostics including the terminal diagnostic. Exact-limit and first-extra tests are
mandatory. Budget exhaustion returns no partial capability.

## 14. Explicit non-goals

This ABI does not authorize implementation, a public aggregate host ABI, stable C library API,
Rust ABI, custom/source-selected allocators, allocator plugins, raw pointers, unsafe source,
address comparison, memory mapping, files, sockets, clocks, randomness, environment variables,
threads, atomics, locks, interior mutability, user finalizers, exceptions, unwinding, async,
callbacks, tracing GC, cycle collection, JavaScript WeakRef behavior, WebAssembly GC, WASI,
Component Model, memory64, shared memory, Windows/macOS native runtimes, static-distribution claims,
freestanding targets, or production readiness.
