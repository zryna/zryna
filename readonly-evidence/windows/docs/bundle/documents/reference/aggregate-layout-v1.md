# Aggregate layout v1

Status: normative planning contract for `DataOwnershipV1`. No compiler or runtime implementation
is authorized by the presence of this document.

## 1. Authority and scope

Aggregate layout v1 is the only M3 authority for stored size, alignment, field order, array stride,
enum discriminants, and compiler/runtime agreement. Semantics, Universal IR, every backend, and
every runtime implementation consume opaque verified layout views from one shared authority. They
must not independently apply a host-language layout, Rust `repr`, JavaScript property order, C ABI
guess, WebAssembly engine convention, or optimization-specific representation.

The contract identifier is `zryna-aggregate-layout-v1`. It defines:

- a target-independent algorithm parameterized only by a sealed storage target;
- exact `Linear32V1` storage for core WebAssembly linear memory;
- exact `LinuxX8664V1` storage for audited Linux x86-64 native output;
- logical field/variant/element ordinals for JavaScript; and
- canonical fixtures and a sealed layout fingerprint for compiler/runtime handshakes.

The first Pair slice may be scalarized and uses no memory or runtime. Scalarization does not erase
the verified logical field ordinals or authorize a different result. Once a value crosses a
runtime, memory, call, or non-scalarized storage boundary, its exact selected layout is mandatory.

This is not a public host ABI. Aggregate values remain internal and scalar ABI v1 remains the only
public function boundary. Packed layout, user-selected alignment, field reordering, niche
optimization, bit fields, C layout, serialization, and raw byte access are unavailable.

## 2. Terms and sealed inputs

A `TypeId` is a dense compiler-owned identity assigned by the canonical algorithm below. Nominal
identities are the sealed `(ModuleId, declaration-index)` pairs defined by `DataOwnershipV1`.
Container identities are structural and are computed before TypeId assignment, so assignment never
depends on discovery order or a provisional child ID. A layout request contains one exact
authenticated type graph and one exact `StorageTarget`:

| Storage target | Pointer bytes | Length/capacity bytes | Maximum alignment | Endian |
| -------------- | ------------: | --------------------: | ----------------: | ------ |
| `Linear32V1`   |             4 |                     4 |                 8 | little |
| `LinuxX8664V1` |             8 |                     8 |                 8 | little |

JavaScript has no byte layout target. It consumes only sealed type identity and source-declaration
ordinals. A JavaScript implementation may scalarize or use private dense storage; it cannot expose
property enumeration, prototype behavior, address, capacity, or padding as language behavior.

All sizes, alignments, offsets, strides, lengths, ordinals, and counts are unsigned integers.
Calculations use a mathematical value first and are admitted only when every intermediate and final
value is representable by unsigned 64-bit checked arithmetic and by the selected target limits.
Wrapping arithmetic is forbidden.

`alignUp(value, alignment)` is defined only when `alignment` is a nonzero power of two:

```text
remainder = value mod alignment
alignUp(value, alignment) =
  value                         when remainder = 0
  checkedAdd(value, alignment - remainder) otherwise
```

No implementation may use a wrapping `(value + alignment - 1) & -alignment` expression as its
validation authority.

### Canonical TypeId assignment

The type universe contains `bool`, `i32`, and String; every nominal data declaration in the final
authenticated module graph; and every structurally distinct fixed-array, Vec, Shared, or Weak
instantiation referenced transitively by declarations or admitted program types. Borrows are
verification authorities and never receive stored TypeIds.

Each universe member has one canonical binary key. One-byte tags and all multibyte lanes below are
unsigned; integers are little-endian. A nested key is length-prefixed so no concatenation is
ambiguous:

```text
00                                      bool
01                                      i32
02                                      String
10 || u32 ModuleId || u32 declIndex     nominal struct
11 || u32 ModuleId || u32 declIndex     nominal enum
20 || u32 length || childKey             fixed array
21 || childKey                           Vec
22 || childKey                           Shared
23 || childKey                           Weak

childKey = u32 childKeyByteLength || child canonical-key bytes
```

A nominal key is a leaf: its fields do not recursively enter that key. This keeps keys finite for a
declaration such as `Node { next: Vec<Node> }`; the Vec key contains the finite nominal Node key.
The authenticated `ModuleId` already comes from the canonical M2 final module map, and
`declIndex` is the zero-based source-order data-declaration index within that module.

The authority rejects duplicate canonical keys that claim different type structure, sorts all
distinct keys by unsigned bytewise lexicographic order, and assigns zero-based TypeIds in that
order. Consequently `bool=0`, `i32=1`, and `String=2`; later IDs depend only on the sealed universe,
never map iteration, traversal, allocation, or backend order. At most 65,536 types are admitted, so
valid TypeIds are `0..65,535`. `0xffffffff` is permanently reserved for the payload-free enum
sentinel and may never identify a type. Every child reference in a sealed record is rewritten to the
final ID obtained from this algorithm before layout or fingerprint computation.

## 3. Primitive and handle layouts

The exact stored layouts are:

| Type        | `Linear32V1` size/alignment | `LinuxX8664V1` size/alignment | Stored validity                           |
| ----------- | --------------------------- | ----------------------------- | ----------------------------------------- |
| `bool`      | `1 / 1`                     | `1 / 1`                       | byte `0` or `1` only                      |
| `i32`       | `4 / 4`                     | `4 / 4`                       | every 32-bit pattern, little-endian       |
| `String`    | `12 / 4`                    | `24 / 8`                      | `(pointer, byte-length, capacity)`        |
| `Vec<T>`    | `12 / 4`                    | `24 / 8`                      | `(pointer, element-length, capacity)`     |
| `Shared<T>` | `4 / 4`                     | `8 / 8`                       | nonzero pointer to verified control block |
| `Weak<T>`   | `4 / 4`                     | `8 / 8`                       | nonzero pointer to verified control block |

Each String and Vec word occurs in the listed order without internal padding. Pointer, length, and
capacity use the target word width. The runtime ABI defines their state invariants and allocation
limits. These handle layouts do not expose allocator metadata or a Rust standard-library layout.

Borrow authorities have no stored layout. Attempting to place a borrow in a struct, enum, array,
heap object, Shared payload, Weak payload, manifest, or public ABI is rejected before layout.

The public scalar ABI's 32-bit Boolean carrier is distinct from this internal one-byte Boolean
storage. A verified wrapper must canonicalize between them. Neither representation can be inferred
from the other.

## 4. Struct layout

A struct has at least one field. Fields are processed exactly in source declaration order. Let
`cursor = 0` and `aggregateAlignment = 1`. For each field:

1. obtain the already verified size and alignment of its type;
2. set the field offset to `alignUp(cursor, fieldAlignment)`;
3. set `cursor = checkedAdd(fieldOffset, fieldSize)`; and
4. set `aggregateAlignment = max(aggregateAlignment, fieldAlignment)`.

The struct alignment is `aggregateAlignment`. The struct size is
`alignUp(cursor, aggregateAlignment)`. No tail-padding reuse, field reordering, packed layout,
overlap, or target optimization is permitted at a storage boundary.

A zero-sized field is permitted and consumes no bytes; its offset is still the aligned cursor.
Multiple zero-sized fields may have the same numeric offset because source cannot observe or take
their addresses. A struct containing only zero-sized fields has size zero and alignment equal to
the maximum field alignment. `Vec<T>` rejects a zero-sized element type even though its standalone
layout is valid.

Padding bytes are not language values and cannot be read, compared, hashed, serialized, or passed
to a host. Generated code must never use an uninitialized padding byte as an input. Any future
boundary that exports complete aggregate bytes must separately require canonical zero padding;
this internal layout contract does not create that boundary.

## 5. Fixed-array layout

For `[T; N]`, element alignment is the array alignment and
`stride = alignUp(elementSize, elementAlignment)`. Array size is
`checkedMultiply(stride, N)`. Element `i` begins at `checkedMultiply(stride, i)` for
`0 <= i < N`.

A zero-length array has size zero and retains the element alignment. A zero-sized element has zero
stride and produces a zero-sized fixed array for every admitted length; addresses are unavailable,
so coincident offsets are unobservable. Dynamic indexing still performs the language bounds check.
Owned `Vec<T>` does not admit zero-sized `T` in v1 because its allocation and capacity state would
require a separate contract.

## 6. Enum layout

Every enum stores one exact unsigned 32-bit discriminant at offset zero. Variant ordinal `k` has
discriminant `k`; source-selected values and niche encodings are forbidden. With the current
per-declaration budget, every ordinal is representable.

For variants without payload, payload size is zero and payload alignment is one. Across all
payload variants:

```text
payloadAlignment = max(1, every payload alignment)
payloadSize      = max(0, every payload size)
enumAlignment    = max(4, payloadAlignment)
payloadOffset    = alignUp(4, payloadAlignment)
enumSize         = alignUp(checkedAdd(payloadOffset, payloadSize), enumAlignment)
```

Only the active variant payload is initialized and may be read or dropped. Bytes in the inactive
payload area and tail padding are inaccessible. Stored discriminants outside the declared ordinal
range are invalid and must be rejected at any authenticated runtime boundary before constructing a
verified value. Internal compiler-produced values cannot contain an invalid discriminant.

The maximum payload size rule deliberately forgoes niche optimization. Adding, removing, or
reordering variants changes the nominal type's fingerprint; it is not a cross-version ABI promise.

## 7. Recursive types and canonical computation

The authority builds the by-value graph in dense TypeId order and visits every outgoing edge in
ascending TypeId order with explicit bounded stacks. It computes strongly connected components,
sorts each component and the component inventory by TypeId sequence, and selects the first cyclic
component. A one-node self-edge reports that TypeId. Otherwise the diagnostic starts at the
component's smallest TypeId, selects its smallest in-component outgoing edge, and uses the first
ascending-edge depth-first path back to the start; the repeated closing node is omitted. This is
the exact bounded definition of the reported cycle. No exhaustive simple-cycle enumeration, host
stack trace, recursion, allocation address, or hash-map order participates.

Struct fields, enum payloads, and fixed-array elements are by-value edges. String has no exposed
element edge. Vec, Shared, and Weak are indirections: their handle layout is complete independently
of `T`, but their full type identity and later drop/runtime metadata still retain `T`.

After cycle rejection, layouts are computed bottom-up. An implementation may memoize only values
keyed by the exact source-map/type-graph identity and StorageTarget. A cached layout from another
compilation, target, or graph is untrusted.

## 8. Universal admission limits

The layout authority applies both the language budgets and these exact layout budgets:

| Budget                                                    |                 Limit |
| --------------------------------------------------------- | --------------------: |
| type nodes in one layout graph                            |                65,536 |
| fields plus variants in one graph                         |                65,536 |
| fields or variants in one declaration                     |                 1,024 |
| layout dependency edges                                   |               262,144 |
| traversal depth after cycle rejection                     |                   256 |
| fixed-array length                                        |             1,048,576 |
| stored alignment                                          |               8 bytes |
| one universally stored object                             | `4,294,967,295` bytes |
| retained layout diagnostics including terminal diagnostic |                   256 |

Although Linux x86-64 can address larger objects, a universal type is rejected if either target
cannot represent its size, offset, stride, length, capacity, or runtime request. The universal
single-object ceiling is therefore `u32::MAX`. A selected target's stricter runtime maximum may
reject an otherwise statically laid-out type when allocating a dynamic value; that is a controlled
capacity or allocation trap, not layout reinterpretation.

Every table row requires exact-limit and first-extra fixtures. Checked-add, checked-multiply, and
align-up overflow fixtures must use small synthetic target limits in tests rather than attempting
host-sized allocations. A synthetic counter fixture may exercise an independent defensive ceiling
when stricter graph-shape ceilings make that ceiling unreachable in one concrete graph.

## 9. Sealed layout record and fingerprint

For each complete TypeId the authority seals:

- contract identifier and StorageTarget;
- complete nominal/container type identity;
- size and alignment;
- source-ordered struct field `(ordinal, TypeId, offset)` records;
- fixed-array `(element TypeId, length, stride)`;
- enum `(variant ordinal, optional payload TypeId, payload offset, total payload area)`; and
- required drop/runtime metadata identity without function pointers or host addresses.

The canonical fingerprint input is:

```text
ASCII "ZRYNA-AGGREGATE-LAYOUT-V1\0"
u32 little-endian storage-target tag (1 = Linear32V1, 2 = LinuxX8664V1)
u32 little-endian record count
records in ascending TypeId order
```

Every record is a tagged, length-prefixed binary record using unsigned little-endian `u32` counts
and IDs and unsigned little-endian `u64` sizes, alignments, offsets, strides, and array lengths.
The exact record prefix is:

```text
u32 payloadByteLength
u32 recordTag
u32 TypeId
u32 dropKind
u32 runtimeKind
u64 size
u64 alignment
tag-specific payload[payloadByteLength - 32]
```

`payloadByteLength` counts every byte after its own four-byte lane, so its minimum is 32. Records
have no implicit padding. The exact tags are `1=bool`, `2=i32`, `3=struct`, `4=enum`,
`5=fixed-array`, `6=String`, `7=Vec`, `8=Shared`, and `9=Weak`. The exact `dropKind` values are
`0=none`, `1=aggregate`, `2=string`, `3=vector`, `4=shared`, and `5=weak`. The exact `runtimeKind`
values use the same numeric table; scalar and purely Copy aggregates use zero. A nonzero kind that
does not match the recursively verified type is invalid.

Tag-specific payloads are exact:

```text
bool, i32, String: empty
struct: u32 moduleId, u32 declarationIndex, u32 fieldCount,
        then fieldCount * (u32 ordinal, u32 fieldTypeId, u64 offset)
enum:   u32 moduleId, u32 declarationIndex, u32 variantCount, u64 payloadOffset,
        u64 payloadAreaSize,
        then variantCount * (u32 ordinal, u32 payloadTypeIdOrFFFFFFFF)
array:  u32 elementTypeId, u64 length, u64 stride
Vec, Shared, Weak: u32 elementOrPayloadTypeId
```

`moduleId` is the authenticated dense module identity from the final source map, and
`declarationIndex` is the source-order nominal declaration index in that module. The all-ones
payload TypeId denotes a payload-free enum variant and is forbidden everywhere else. Nominal
records never encode source paths or spellings. Struct fields and enum variants occur in ordinal
order. The fingerprint is SHA-256 of the complete byte document. JSON, hexadecimal strings, locale
text, host paths, pointer values, compiler version text, and filesystem order never enter the hash.
Issue #77 implements and verifies this already-frozen encoding; it may not choose another tag,
field, width, sentinel, or order.

The verified IR, target backend, runtime implementation, manifest, and conformance evidence must
name the same fingerprint. A target mismatch or record drift fails closed before emission.

## 10. Normative fixtures

The following fixtures are exact. `size/alignment` uses bytes; offsets are declaration order.

| Type                                               | `Linear32V1`               | `LinuxX8664V1`             |
| -------------------------------------------------- | -------------------------- | -------------------------- |
| `Pair { left: i32, right: i32 }`                   | `8/4`, offsets `[0,4]`     | `8/4`, offsets `[0,4]`     |
| `Mixed { flag: bool, value: i32, tail: bool }`     | `12/4`, offsets `[0,4,8]`  | `12/4`, offsets `[0,4,8]`  |
| `Nested { pair: Pair, flag: bool }`                | `12/4`, offsets `[0,8]`    | `12/4`, offsets `[0,8]`    |
| `[bool; 3]`                                        | `3/1`, stride `1`          | `3/1`, stride `1`          |
| `[Pair; 2]`                                        | `16/4`, stride `8`         | `16/4`, stride `8`         |
| `MaybeI32 { none, some(i32) }`                     | `8/4`, payload offset `4`  | `8/4`, payload offset `4`  |
| `Choice { flag(bool), pair(Pair) }`                | `12/4`, payload offset `4` | `12/4`, payload offset `4` |
| `TextFlag { text: String, flag: bool }`            | `16/4`, offsets `[0,12]`   | `32/8`, offsets `[0,24]`   |
| `Links { strong: Shared<Pair>, weak: Weak<Pair> }` | `8/4`, offsets `[0,4]`     | `16/8`, offsets `[0,8]`    |

The Pair fixture is the only aggregate authorized for the first vertical slice. The remaining rows
are contract fixtures for later dependency-ordered issues and do not claim source or backend
implementation.

Required negative fixtures include direct and indirect by-value recursion, duplicate nominal
identity, duplicate/missing TypeId, unknown field/payload type, borrow storage, invalid target tag,
non-power-of-two or excessive alignment, checked-add overflow, checked-multiply overflow, align-up
overflow, object-size first-extra, graph/depth/edge first-extra, field/variant first-extra, and a
layout fingerprint or ordinal mutation.

## 11. Stable diagnostics

The layout authority reserves these exact diagnostics:

| Code          | Meaning                                                          |
| ------------- | ---------------------------------------------------------------- |
| `ZRYNA-L3001` | invalid, duplicate, missing, or foreign type identity            |
| `ZRYNA-L3002` | direct or indirect by-value recursive layout                     |
| `ZRYNA-L3003` | invalid field, variant, array, ordinal, or type-graph record     |
| `ZRYNA-L3004` | unstorable type, including a borrow authority                    |
| `ZRYNA-L3005` | checked size, alignment, offset, stride, or target-limit failure |
| `ZRYNA-L3006` | storage-target, sealed-record, or fingerprint mismatch           |
| `ZRYNA-L3201` | deterministic layout resource budget exhausted                   |

Diagnostics are source-located when an authoritative declaration span exists and otherwise use a
stable global or workspace location. Selection is deterministic by authoritative source order,
code, canonical TypeId path, and complete numeric tie-break data. Exhaustion emits the terminal
diagnostic and returns no partial sealed layout.

## 12. Security and explicit non-goals

Verification must prevent integer wrap, undersized allocation, misaligned access, field confusion,
invalid discriminants, inactive-payload reads, recursive infinite size, target cache confusion,
padding reads, Rust-layout leakage, and backend-specific field order. Backends must use checked
address formation even after static layout succeeds because dynamic base plus offset can overflow or
leave an allocation.

This contract does not define allocator algorithms, public aggregate calling conventions, stable
cross-release binary compatibility, raw pointers, pointer provenance visible to source, C layout,
packed/aligned attributes, unions, bit fields, SIMD, atomics, threads, endianness beyond the two
listed targets, Windows or macOS native layout, WebAssembly64, Wasm GC, Component Model Canonical
ABI, serialization, reflection, unsafe access, or freestanding systems layout.
