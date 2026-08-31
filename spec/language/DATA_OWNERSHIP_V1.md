# Data and ownership v1

Status: normative planning contract for M3. No compiler, frontend, backend, runtime, CLI, or host
support is implemented by this document. An implementation may claim this profile only after the
complete three-target conformance gate and authenticated publication gate pass.

## 1. Profile identity and compatibility

The exact universal profile name is `DataOwnershipV1`; its CLI spelling is
`data-ownership-v1`, its manifest profile is `zryna-data-ownership-v1`, and its future public
bundle is `zryna-manifest-v3.json`. The profile requires a separately versioned provider-neutral
syntax protocol and a separately verified Universal IR. Those versions are not selected or
implemented by this specification issue.

`DataOwnershipV1` extends the language behavior of `ControlFlowV1`; it does not reinterpret any
existing contract. In particular:

- omitting `--profile` continues to select M1 `I32V1` and manifest v1;
- exact `--profile control-flow-v1` continues to select M2 `ControlFlowV1` and manifest v2;
- syntax protocols v2 and v3, scalar ABI v1, M1 and M2 verified IR, existing diagnostics,
  artifact bytes, fixtures, and conformance registries remain independent regression authorities;
- no M3 declaration, value, instruction, helper, import, or manifest field may appear in an M1 or
  M2 artifact; and
- no intermediate M3 issue may expose the profile publicly. Public selection is an integration
  gate after language, verifier, three backends, runtime boundary, and conformance are complete.

Only entry-module exports enter scalar ABI v1. Aggregate, owned, borrowed, shared, and weak values
are internal in v1 and may not appear in a public parameter or result. Public M3 observations are
therefore still exact `i32` or `bool` observations. A public aggregate host ABI requires a separate
proposal and cannot infer its representation from compiler-private storage.

## 2. Evaluation and type identity

All expressions, arguments, initializers, fields, elements, and statements evaluate left to right,
exactly once. A controlled trap performs the cleanup required by section 11 and prevents every
later evaluation. Divergence prevents later evaluation but is not a trap.

The admitted scalar types remain `i32` and `bool`. M3 additionally defines these internal types:

- nominal structs;
- nominal enums with zero or one payload per variant;
- fixed arrays `[T; N]` with a compile-time `u32` length;
- owned UTF-8 `String`;
- owned `Vec<T>`;
- shared ownership `Shared<T>`;
- non-owning `Weak<T>`; and
- non-escaping shared and exclusive borrows of places.

A nominal declaration identity is the sealed pair `(ModuleId, declaration-index)` after the exact
M2 module closure is authenticated. The index is zero-based source order among struct and enum data
declarations in that module. Source spelling, path, backend name, and structural equality cannot
substitute for that identity. Type arguments are part of the complete type identity.

By-value type containment must be finite. A struct, enum payload, or fixed array that reaches its
own nominal identity through only by-value edges is rejected. `String`, `Vec`, `Shared`, and `Weak`
are indirections and stop layout recursion. Borrow types are not storable and do not participate in
stored layout.

There are no implicit conversions, structural object types, prototype members, dynamic property
creation, index signatures, unions, nullable values, reflection, layout introspection, or type
assertions. Generics remain unavailable except for the compiler-known containers and references
listed above; user-defined generic declarations are not enabled.

## 3. Structs and the first vertical slice

A struct declares a nonempty, source-ordered sequence of uniquely named fields. Field names use the
portable identifier grammar and may not be computed. A struct value is constructed by naming every
field exactly once. Missing, duplicate, unknown, or type-mismatched fields are errors. Construction
evaluates initializers in declaration order, irrespective of the source order used by a future
syntax. Field access resolves to a sealed field ordinal and never performs a target property lookup.

The smallest mandatory fixed-oracle M3 aggregate case is this internal behavior. The following
block is semantic pseudocode, not TypeScript-parseable Zryna source; Issue #76 freezes the exact
provider syntax and DTO encoding without changing these meanings:

```text
struct Pair {
  left: i32;
  right: i32;
}

export function pairScore(left: i32, right: i32): i32 {
  const pair: Pair = Pair { left, right };
  return pair.left * 31 + pair.right;
}
```

The notation above freezes source-level meaning, not a syntax-protocol encoding. The provider must
eventually report source-faithful declaration, construction, shorthand, and field-access DTOs; it
never assigns nominal identity, types, ordinals, layout, ownership, or IR operations.

The fixed first-slice observations are:

| Call                         |     Exact result |
| ---------------------------- | ---------------: |
| `pairScore(0, 0)`            |          `i32:0` |
| `pairScore(1, 2)`            |         `i32:33` |
| `pairScore(-1, 2)`           |        `i32:-29` |
| `pairScore(2147483647, 1)`   | `i32:2147483618` |
| `pairScore(-2147483648, -1)` | `i32:2147483647` |

Multiplication and addition retain the exact wrapping `ControlFlowV1` definitions. The same fixed
oracle must pass JavaScript, direct core WebAssembly, and Linux x86-64 native execution. The Pair
layout fixtures are normative in
[`AGGREGATE_LAYOUT_V1.md`](../memory-model/AGGREGATE_LAYOUT_V1.md). This slice has no heap,
allocator, runtime import, move-only value, drop effect, borrow, shared reference, weak reference,
or public aggregate ABI. JavaScript and machine backends may scalarize Pair only when verification
proves that doing so cannot change any observation or layout-dependent runtime operation.

## 4. Enums and fixed arrays

An enum declares a nonempty source-ordered list of uniquely named variants. A variant has either no
payload or one payload type. Its discriminant is its zero-based declaration ordinal represented as
an unsigned 32-bit value in stored layout. Source cannot assign discriminants. Construction names
one exact variant and supplies exactly the required payload. Reading an inactive payload, forging a
discriminant, or reaching a match arm that does not correspond to the active variant is impossible
after verification.

Pattern matching must be exhaustive, evaluate the scrutinee once, and bind only the active payload.
Duplicate or unreachable variants are errors. A future wildcard arm may not hide an enum variant
added within the same closed compilation. Enums are closed within their declaring module graph.

`[T; N]` contains exactly `N` values in ascending index order. `N` is a canonical decimal `u32`
constant. Construction evaluates elements from index zero upward. Array equality, slicing, spread,
and implicit array-to-vector conversion are unavailable. Indexing accepts `i32`; a negative index
or an index whose unsigned value is not less than `N` produces `zryna.trap.bounds-v1`. The check is
mandatory even when a target would otherwise mask, coerce, or trap differently. Length zero is
valid, but every indexing operation on it traps.

## 5. Copy values, owned values, and places

`i32` and `bool` are `Copy`. A struct, enum, or fixed array is `Copy` if and only if every reachable
stored field, active payload possibility, or element is `Copy`. `String`, `Vec<T>`, `Shared<T>`, and
`Weak<T>` are never `Copy`; copying a source spelling does not clone them. Borrow values are
non-owning compiler authorities and are never stored or copied as ordinary values.

An owned value occupies one exact place. Places are locals and statically resolved projections of
struct fields, the active enum payload, or fixed-array constant indices. A vector or dynamically
indexed array element is conservatively part of the complete container place in v1. Overlapping
places cannot be independently moved or borrowed.

Using a `Copy` place reads a copy and leaves the place initialized. Using a non-`Copy` place in a
value-consuming context moves it and leaves the place moved. Assignment evaluates the right-hand
side completely before changing the destination. If evaluation succeeds, the old initialized
destination is dropped, then the new value is installed. If evaluation traps, the old destination
remains initialized until trap cleanup.

Explicit clone operations exist only for types with the structural `Clone` capability. `bool` and
`i32` are `Clone` because they are `Copy`. String is `Clone` by allocating and copying its bytes.
A fixed array is `Clone` exactly when its element type is `Clone`; a struct is `Clone` exactly when
every field is `Clone`; an enum is `Clone` exactly when every payload type is `Clone`; and `Vec<T>`
is `Clone` exactly when `T` is `Clone`. Aggregate and Vec clones proceed in source/ascending element
order and clean the completed destination prefix in reverse order on failure. `Shared<T>` and
`Weak<T>` are `Clone` for every admitted `T` by checked handle-count operations; cloning does not
clone their payload. Shared and exclusive borrows are not owned values and are never `Clone`.
There is no implicit clone at assignment, call, return, branch, or container construction.

## 6. Ownership state and control-flow joins

Every non-`Copy` place has one compile-time state:

- `uninitialized` before a declaration or field has completed initialization;
- `initialized` while it owns one live value;
- `moved` after its value transfers elsewhere;
- `shared-borrowed(k)` while `k > 0` shared borrows are active;
- `exclusive-borrowed` while one exclusive borrow is active; or
- `dropped` after final destruction on a path.

The verifier rejects reading an uninitialized, moved, or dropped place; moving or dropping a
borrowed place; creating an exclusive borrow while any borrow exists; creating a shared borrow
while an exclusive borrow exists; mutating through a shared borrow; and using an owner in a way
forbidden by its active borrow.

Every reachable control-flow edge carries the complete definite state of live places. A merge is
valid only when each live place has the same ownership and initialization state on every incoming
edge. Borrow state may not cross a branch, loop header, loop backedge, or function return in v1.
Loop bodies must therefore restore every loop-carried place to the exact header state before the
backedge. The compiler does not insert an implicit clone or conditional drop to repair mismatched
states.

Function arguments evaluate left to right. Passing a non-`Copy` argument by value moves it into the
callee. A returned non-`Copy` value moves into the caller. The resolved call graph remains acyclic.
Each call has one owner for each by-value argument and result at every point.

## 7. Deterministic drop

Normal exits and controlled traps drop every live owned value exactly once. Drops occur in reverse
completion order, not source-name or target-storage order:

1. locals in reverse successful initialization order;
2. struct fields in reverse declaration order;
3. the active enum payload only;
4. fixed-array and vector elements from highest initialized index to zero; and
5. a container's allocation after all contained values are dropped.

Moving a value transfers its pending drop obligation. A moved place is not dropped. A `Copy` value
has no drop obligation. A partially initialized aggregate owns only the fields or elements whose
initializers completed; cleanup drops exactly that initialized prefix in reverse order. Replacing
an initialized value drops the old value only after the replacement has completed successfully.

There are no user-defined destructors, finalizers, exceptions, unwinding callbacks, or observable
drop hooks in v1. Runtime releases are infallible and may not allocate. Internal fault-injection and
drop-trace fixtures must nevertheless prove exact-once release and ordering. An external process
kill, engine termination, hardware fault, or host violation is outside the language trap contract
and does not promise cleanup.

## 8. Owned strings and vectors

`String` owns a finite, well-formed UTF-8 byte sequence. Its length is the number of bytes. String
construction must validate external or decoded bytes before producing a value. Concatenation is
checked for length and capacity overflow and either returns one initialized String or performs
controlled trap cleanup. Direct numeric string indexing, code-unit indexing, implicit coercion,
normalization, locale comparison, and exposing capacity are unavailable. Later character or slice
operations require a separate boundary-safe specification.

`Vec<T>` owns a contiguous sequence of initialized `T` values with logical length. Capacity and
allocation strategy are not source-observable. Push evaluates its value first, grows storage if
needed, and then moves the value into the new final element. On growth failure, the original vector
and argument remain owned until controlled trap cleanup; no partially moved public state escapes.
Pop requires an explicit syntax-independent result contract in a later issue and is not authorized
by this document. Indexing uses the same checked `i32` bounds behavior as fixed arrays. Mutation
requires an exclusive owner or exclusive borrow.

The target-private storage mappings and checked allocator operations are defined by
[`OWNERSHIP_RUNTIME_V1.md`](../abi/OWNERSHIP_RUNTIME_V1.md). No target may expose a Rust `String`,
Rust `Vec`, JavaScript Array prototype, raw WebAssembly address, or native pointer as a Zryna value.

## 9. Borrowing

A shared borrow grants read-only access to one place. Any number of shared borrows may coexist when
their places do not overlap an exclusive borrow. An exclusive borrow grants read/write access and
requires that no overlapping borrow exists. Borrow creation does not move or clone the owner.

M3 v1 borrows are non-escaping and scoped:

- they begin and end within one structured lexical block;
- they are discharged before a branch edge, loop edge, return, move, or owner drop;
- they cannot be stored in a struct, enum, array, String, Vec, Shared, or Weak value;
- they cannot be returned, exported, captured, converted to an integer, or compared by address;
- an internal direct call may receive a borrow only for that call, and cannot retain or return it;
- constant projections of distinct struct fields or fixed-array indices are disjoint; dynamic
  indexes and vector elements borrow the complete container.

Borrow checking is a compiler proof and has no runtime reference-count or tracing requirement.
Backends may erase a borrow only after verified IR retains the exact place, access mode, region,
and dominance proof needed to prevent use-after-move or conflicting access.

## 10. Shared and weak ownership

`Shared<T>` is immutable shared ownership of one `T`. Creating it consumes one uniquely owned `T`.
Cloning it is explicit and checked. The payload is dropped exactly once when the final strong handle
is released. V1 provides no interior mutability, identity comparison, exposed count, cycle
detection, or thread-safe atomic reference counting.

`Weak<T>` does not keep the payload alive. Downgrading a `Shared<T>` creates one Weak handle;
cloning Weak is explicit and checked. Weak cannot be dereferenced directly.

Weak upgrade does not depend on an unspecified generic `Option<T>`. Verified IR contains one
`WeakUpgradeBranch` operation with two successors. If a strong owner still exists, the operation
performs one checked strong-count increment and passes a new `Shared<T>` only to the success
successor. If no strong owner exists, it passes no value to the failure successor. The decision and
increment are one indivisible language operation in the single-threaded v1 execution model. Count
overflow traps instead of taking either successor. A future source syntax must expose both branches
without constructing a nullable or forgeable handle.

The immutable, fully initialized Shared construction model provides no operation that can construct
a strong-reference cycle in v1. There is no partial Shared initialization, interior mutation, or
cycle constructor. Weak remains useful for non-owning observers and acyclic parent/back links built
while the uniquely owned payload is assembled. A forged cyclic control graph is an ABI violation,
not a source value. This profile performs no tracing garbage collection. Threads and host
reentrancy are unavailable, so v1 makes no cross-thread weak-upgrade or memory-ordering claim.

## 11. Controlled traps and cleanup

The exact language trap identities are:

| Trap                       | Cause                                                                                          |
| -------------------------- | ---------------------------------------------------------------------------------------------- |
| `zryna.trap.bounds-v1`     | negative or out-of-range array/vector index                                                    |
| `zryna.trap.allocation-v1` | allocator cannot satisfy a valid request                                                       |
| `zryna.trap.capacity-v1`   | checked length, capacity, byte-size, or growth arithmetic overflows or exceeds a profile limit |
| `zryna.trap.refcount-v1`   | a checked strong or weak count increment would overflow                                        |
| `zryna.trap.utf8-v1`       | a runtime boundary attempts to construct String from invalid UTF-8                             |

These traps are target-independent, non-catchable language outcomes. Runtime operations return
typed success/failure status to generated code; they must not throw, abort, signal, or use ambient
host exceptions as the semantic decision. Generated code runs the exact live-value cleanup plan,
then reports the original trap identity through the driver-owned observation channel. Cleanup
failure is an internal runtime-contract violation, not a replacement trap.

JavaScript uses private sealed helpers and a private trap sentinel caught only by the generated
entry wrapper after cleanup. WebAssembly uses explicit status/control flow and an audited host
observation wrapper; a raw engine trap is not a Zryna trap. Native uses explicit status/control
flow and the private typed harness; a signal or process exit code is not a Zryna trap.

## 12. Resource budgets and diagnostics

All existing M2 budgets continue to apply. M3 adds these maximum admitted counts per compilation:

| Budget                                                |     Limit |
| ----------------------------------------------------- | --------: |
| nominal data declarations                             |     4,096 |
| fields plus enum variants                             |    65,536 |
| fields or variants in one declaration                 |     1,024 |
| distinct fully instantiated M3 types                  |    65,536 |
| aggregate-construction operands                       |   262,144 |
| fixed-array length                                    | 1,048,576 |
| ownership places per function                         |    65,536 |
| ownership state transitions per function              |   262,144 |
| active borrows per function                           |    16,384 |
| inserted drop actions per function                    |   262,144 |
| retained M3 diagnostics including terminal diagnostic |       256 |

Runtime String and Vec lengths, capacities, and allocation sizes additionally obey the target
limits in the runtime ABI. Every count and size uses checked arithmetic. Exact-limit and first-extra
fixtures are mandatory. Budget exhaustion emits one terminal diagnostic and prevents later phases
from acting on an incomplete type graph, state graph, layout, or drop plan.

Stable diagnostic families are reserved as follows:

- `ZRYNA-D4xxx`: M3 declaration, type-graph, nominal-identity, and layout-input diagnostics;
- `ZRYNA-M3xxx`: ownership, initialization, move, borrow, container, and semantic diagnostics;
- `ZRYNA-I3xxx`: DataOwnershipV1 verified-IR and drop-plan diagnostics;
- `ZRYNA-L3xxx`: aggregate-layout verification diagnostics; and
- existing backend families use separately documented M3 subranges.

Within every new family, `x2xx1` is reserved for deterministic resource exhaustion. Exact codes
must be frozen by the implementing issue before executable use. Provider diagnostics never
substitute for Zryna type, layout, ownership, borrow, drop, or IR verification.

## 13. Target-equivalence requirements

Equivalent behavior means the same accepted program, ordered scalar observation or language trap,
left-to-right evaluation, move legality, borrow legality, active variant, initialized element set,
and exact logical drop/release trace under fault injection. It does not require identical target
addresses, allocation capacity, padding bytes, object identity, host exception text, or physical
representation.

- JavaScript may use sealed compiler-private records, dense arrays, typed arrays, or scalarization.
  Engine garbage collection is an implementation substrate, never permission to use a moved value
  or omit a specified release transition.
- core WebAssembly uses explicitly declared linear memory only when a memory-bearing subprofile is
  selected. It may not add imports, WASI, host allocation, tables, GC types, threads, or ambient
  capabilities. The Pair case remains import-free and memory-free.
- Linux x86-64 native uses only audited layouts and exact allowlisted runtime symbols. Rust library
  layouts, C++ exceptions, libc allocation behavior, and unverified undefined symbols are not ABI
  authorities.

Every backend consumes opaque verified M3 IR plus sealed layout and ABI views. No backend may infer
field order, recompute ownership, invent a helper symbol, repair invalid counts, or make another
backend authoritative.

## 14. Delivery ledger

Implementation proceeds only in the dependency order frozen by Issues #75 through #90:

1. #75 freezes this profile, aggregate-layout, ownership, and runtime ABI contract plus the
   digest-pinned planning inventory;
2. #76 adds the separately versioned syntax DTO/schema and syntax-only TypeScript adapter while #77
   adds the shared layout authority below semantics and backends;
3. #78 adds isolated DataOwnershipV1 raw and verified IR;
4. #79 adds struct, enum, and fixed-array semantics, including Pair as the smallest mandatory
   aggregate case, while #80 adds the runtime ABI authority;
5. #81 adds owned String and Vec, move checking, and deterministic drop insertion;
6. #82 adds bounded borrowing, then #83 adds Shared and Weak including `WeakUpgradeBranch`;
7. #84, #85, and #86 add the complete JavaScript, direct core WebAssembly, and verified native MIR
   mappings only after the ownership chain they consume is complete;
8. #87 adds the native object, runtime, link, and execution boundary;
9. #88 integrates the three complete targets into a candidate driver route and manifest v3, but
   keeps `--profile data-ownership-v1` unavailable in the public CLI;
10. #89 runs fixed-oracle conformance, with Pair as its first and smallest executable observation,
    plus the full negative, boundary, fault, determinism, and ownership corpus; and
11. #90 activates exact public selection only after #89 passes, then publishes authenticated
    compiler documentation, website deployment, and live provenance evidence.

Pair is therefore a mandatory conformance seed, not an earlier public or independently executable
checkpoint. No issue may bypass the ownership and runtime dependencies in the canonical graph to
publish a partial Pair-only profile.

No row may make the public profile selectable before all preceding trust authorities it consumes
are merged and independently verified.

## 15. Explicit non-goals

This contract does not authorize compiler implementation, protocol v4 bytes, allocator code,
public aggregate host ABI, raw pointers, unsafe blocks, address arithmetic, FFI, custom allocators,
user destructors, exceptions, async, closures, threads, atomics, interior mutability, tracing GC,
WebAssembly GC, WASI, Component Model, browser DOM integration, packed or user-aligned structures,
layout reflection, serialization ABI, freestanding targets, embedded firmware, additional native
platforms, packages, optimization guarantees, or production-readiness claims.
