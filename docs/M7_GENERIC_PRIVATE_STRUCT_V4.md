# Private generic Struct execution

This private #416 continuation implements nominal Struct construction, whole-value
transport and Copy field observation through a distinct owned-IRv4 issuer. The
accepted #415 language and decision contracts remain normative. Full #416,
including its public driver activation, remains open.

The currently qualified source slice supports scalar/String/opaque stored fields,
nested nominal fields and arguments, and both Option/Result roots and nominal
members containing those families. Containers and fixed arrays, affine partial
field moves and field subloans remain separate implementation work. Whole Struct
Clone uses the separate [private V5 issuer](M7_GENERIC_PRIVATE_STRUCT_CLONE_V5.md);
the V4 issuer retains its original operation domain. Layout representability alone
does not grant execution authority for remaining forms.

The existing [owned execution](M7_GENERIC_OWNED_EXECUTION.md) and its frozen v1/v2/v3
wire contracts remain separate. Syntax-v5 already carries StructConstruction and
FieldAccess on both producers. There is no new frontend protocol, runtime ABI,
dependency, limit increase or public scalar export shape in this continuation.

## Distinct wire and issuer

Use a new ASCII header `ZRYNA-GENERIC-OWNED-IR-V4` followed by NUL (26 bytes), then
little-endian u32 version4. The remaining envelope retains the owned-v3 field order:
length-prefixed frozen generic core-v1 message, ordered per-function extension lists,
then ordered per-function cleanup plans. Each extension contains resultId:u32,
tag:u8 and its payload. Each plan step retains block:u32, position:u32, canonical
failure:u8 and length-prefixed end-loan and cleanup ID vectors. Consume all bytes.

Owned tags1..7 retain their existing meanings and layouts. Add exactly:

| Tag | Operation | Payload, all u32 little-endian |
| --- | --- | --- |
| 8 | `StructConstruct { ty, fields }` | exact typeId, fieldCount, fieldValueIds in declaration order |
| 9 | `CopyStructField { value, ordinal }` | whole stored base valueId, original field ordinal |

Each extension replaces one exact core Unit placeholder with the same resultId.
The placeholder carries the claimed result type. Generic core-v1 operation tags1..8
are frozen; do not append an operation to that namespace. Unknown owned tags reject.

`wire::v4::decode` returns a distinct opaque `DecodedOwnedV4`. Its `claims()` accessor
borrows immutable untrusted raw claims. It has no public raw constructor, `From`,
unchecked conversion, generic decoder fallback or conversion to the old decoded
wrapper. `verify_v4` consumes this wrapper and independently proves every authority
before issuing the existing opaque `VerifiedOwnedProgram`. The program retains its
owned claims and borrowed dual-layout/runtime authorities as before.

The typed v4 entrypoint alone selects a private unforgeable admission token for
shared internal routines. Raw claims contain no mode and callers receive no boolean
switch. Old v2/v3 entrypoints continue rejecting Struct keys0x10/0x12, including
pass/return-only graphs with no new opcode. Old encoders reject new owned tags; old
decoders reject them even after both header and version are rewritten to an old pair.
Copying v4 raw claims through old encoders and decoders cannot acquire admission.

## Source, type and ownership proof

Before generated instance expansion, check all originals in their opaque declaration
frames, including unused templates and every Struct field. First validate actual
supplied source arguments in a private preparation state; retain that same builder
and its counters. Original ownership must pass before pending expansion can finish
or issue a context. Reuse original nominal identity, finite substitution and
dual-layout authority. Equal width or shape cannot identify
foreign nominal declarations. Exact field types come from all substituted stored
fields; a phantom type argument alone cannot determine ownership.

Validate the complete initializer-name mapping before evaluating any initializer.
Check each value against its exact substituted stored field type before committing
the aggregate. IR source replay reports malformed maps/types with M7006; complete
provider-level diagnostic ownership remains part of #416 qualification. Evaluate
in field declaration order while preserving authentic source spelling and spans. Empty declarations remain D7001: accepted #415 retains
M3's nonempty field requirement. A physically decoded zero-length raw field vector
cannot validate against a sealed Struct.

Retain each successful initializer as a separate live temporary. The final
allocation-free StructConstruct consumes each affine field once and publishes one
whole owner. Independently reject duplicate affine IDs, moved or loaned operands,
wrong counts/types/order and invented placeholders. Failed construction cleans up
only successful initializers in reverse order, then earlier surviving roots. There
is no partial aggregate carrier, initialized bitmap or constructor failure step.
Destruction of a completed Struct visits fields in reverse declaration order.

CopyStructField retains its complete live base, selects an exact sealed ordinal and
produces precisely that field's type. Both original-frame proof and independently
sealed zero-drop ownership must justify Copy. Specializing opaque T to i32 cannot
create original Copy authority. The operation cannot move an affine field, create a
subloan, erase an exclusive loan or read a moved/uninitialized base. General partial
places, opaque field moves, field loans and structural Struct Clone remain separate
acceptance gaps.

## Contextual nested source facts

Private v4 resolves each occurrence in its exact original function or data frame.
Opaque atoms retain definition kind, module, declaration and ordered slot identity;
a callee substitution keeps its caller's atom. Complete closed keys keep their
existing bytes. Every actual stored field and possible enum payload determines
Copy affinity; a phantom argument is validated independently and does not add
stored affinity or height. Option/Result roots use the same contextual proof.

A per-request normalized term graph separates argument discovery from by-value
member evaluation. Only stored edges determine cycles, ownership and physical
height. Explicit traversal retains key depth64, key bytes4,096 and the layout's
stored-path depth256 as separate bounds. The only persistent cache row contains a
completed root fact. A hit pays the same logical proof work as a cold request;
a failed miss preserves the last completed row and retains only its completed
root-lookup debit. No gray node supplies Copy authority or an acyclicity promise.

Name and type-registry comparisons, key retention and graph expansion are prepaid
on the existing nominal-work ledger. Both declaration-module resolution and
ordered substitutions are independent of sealed layout. The old owned admission
continues through its original facts, resolver and charging paths.

## Original definition rejections

A direct generic stored member whose nominal head and complete ordered parameter
tuple equal its original owner proves a by-value self cycle under every legal
substitution. The rejection-only layout predicate returns no authority. Independent
semantic and IR adapters authenticate actual field or payload roots and report
`ZRYNA-L3002` at the complete original member type. The literal unused `Node<T>`
needs no fabricated closed instance or function.

This sufficient proof handles one or two original parameters. It does not descend
wrapper or phantom arguments, infer equality from possible substitutions, or claim
general symbolic cycle coverage. Nongeneric and old owned issuers retain their
existing behavior. Original expanding definitions retain `ZRYNA-M7003`. Both
rejections join the existing canonical collector after original source-call and
actual supplied-argument validation, before private-v4 ownership and generated
instance completion.

## Original stored demand

The v4 issuer independently replays original stored roots as well as executable
function uses. Every nongeneric nominal declaration is required, including unused
ones. Fully closed types in unused templates also contribute roots, including a
closed child beneath an opaque or borrowed parent. Authenticated external roots
and forest edges assign the exact original binder; no source-span heuristic or
invented parameter specialization supplies that identity.

A child-first descriptor pass selects maximal closed stored subtrees. These
subtrees are disjoint, so resolution does not repeatedly scan every descendant.
The existing layout traversal reaches generic arguments, referents, fields and
variants, including phantom arguments. Root marks borrow the exact program
inventory and cannot be replayed against another raw claim. Missing required roots
and extra undemanded specializations reject. The legacy demand path is unchanged.

## Existing resource ceilings

Keep complete wire bytes32MiB, outer children1,048,576, per-constructor fields1,024,
aggregate operands262,144 and owner/state/key-copy work1,048,576. Every field ID is
charged before reservation and ownership mutation to the appropriate wire and
operand counters. Preserve core key bytes4,096, key depth64 and all inherited
module/function/CFG/value/vector/literal limits. Existing embedded-core and outer
wire counters retain their independent accounting; do not transfer credits.

Struct key copies0x10/0x12 join existing enum0x11/0x13 metering, including Copy-only
signatures and placeholders. Producer and hostile verifier independently account
for StructConstruct operands. Native expansion continues enforcing1,000,000 codegen
units before allocation; target output remains32MiB. Recompute recursive Struct
drop, lane-copy and field-fanout work from actual emitted operations. No backend
wildcard, temporary rejection waiver or guessed enum-only cost can qualify v4.

A sealed v4 program retains a private dense result-to-extension index. Its full
`2F + sum(3B + 2D + E)` debit shares the actual ownership replay ledger and precedes
all index table allocation. Here F counts functions, B blocks, D value definitions
and E extension records. Failed construction preserves caller credit. Old v2/v3
programs retain their previous lookup and charges.

For Struct-containing programs, JavaScript prepays full definition scans for every
explicit drop and cleanup occurrence on its existing output-work ledger. Wasm
prepays every Struct field and enum width edge, including Copy enums with shared
children. No-Struct paths and all target ceilings retain their previous behavior.

## Fixed test authorities and qualification

The [source oracle](../tests/m7-generic-struct-v4/oracle.json) retains the exact
accepted `struct-construction` source: Box<i32>({ value:7 }), then box.value, gives
i32:7 and no release trace. Its independently packed440-byte wire vector uses tags8/9
at offsets389/406 and a343-byte frozen core. The repeated11/22/33 fingerprints are
deliberate wire-only sentinels, so this literal grants no executable authority.
The independent277-byte Copy-v1 vector stays byte-for-byte exact. Separate191-byte
v2/v3 vectors fix the inherited clone tags6/7 and their old envelopes. These wire
literals are stored as unchanged text hex and decoded at compile time with exact
length and digit checks; their decoded byte counts and hashes are unchanged.

The [fault oracle](../tests/m7-generic-struct-v4/fault-oracle.json) retains the accepted
constructor-fault outcome `zryna.trap.allocation-v1` and exactly
`release:left`, `release:earlier-root`. Frozen declaration-order and reordered source
spellings use the same declaration-order evaluation and prefix cleanup expectations.
The [source catalogue references](../tests/m7-generic-struct-v4/source-authorities.json)
retain nominal identity, finite supplied nesting and diamond imports without
repairing the accepted cases.

Tests cover literal bytes, every truncated prefix, domain/version and opcode
isolation, old raw-claim laundering, typed wrapper compile failures, exact and
first-extra resource bounds, original Copy/move/loan rules, complete stored demand
and hostile source/type claims. Native and TypeScript packets independently cross
source authentication; neither packet grants semantic or executable authority.

The Linux x86-64 [same-seal integration test](../crates/zryna-driver/tests/generic_struct_v4/README.md)
gives one retained v4 program per case to JavaScript, core Wasm and native emitters.
Its explicit exported-entry derivative preserves the exact original source prefix.
Fixed allocation identities distinguish equal-content clones. Success releases
right, left, then the earlier root; allocation faults release only the completed
prefix. Each fault is followed by a pristine retry. Native faults require SIGILL
and exact cleanup stdout before the trap. All three targets retain 15 observations
each across construction and both initializer spelling orders.

Native resource tests first obtain genuine source, semantic, dual-layout, runtime
and v4 IR authorities. A flat Struct with 32 String fields and 1,024 cleanup sites
has a measured exact old fixed-cost census below the unchanged 1M limit; complete
Struct destructor expansion exceeds it and must reject before backend emission.
A one-site control must pass. A separate preserved nested wrapped-enum witness
now crosses original ownership, discovery, both layouts, runtime and the v4 seal
before native lowering. Its one-site control passes; 24 sites exceed the unchanged
1M native work ceiling, including the fixed enum-only lower bound1,007,616, and
report N7001 before code generation.

The additive [nested same-seal test](../crates/zryna-driver/tests/generic_struct_v4_nested/README.md)
authenticates all 13 complete literals with fresh native parsing and both frozen
provider packets. Eight accepted cases cover nested Copy observations, whole owned
call transport, both Option/nominal nesting directions, plain nominal fields,
reversed type arguments, phantom versus fixed-member affinity and declaration-order
failure prefixes. Each retained program feeds all three emitters. Fixed allocation
identities require26 observations per target,78 total, including nine separate
native SIGILL cases and a pristine retry after every controlled fault. The five
negative originals retain owning M7007 rejection before closed discovery.

An additive imported case uses three authenticated modules, aliases and affine
same-spelled caller decoys. It retains the declaring module's exact nested member
keys and independently feeds its one seal to all three targets, with three separate
zero-allocation observations. This does not change the earlier total of 78 observations.

Authenticated plain-source kernel fixtures separately exercise actual member paths
of256 versus257 stored nodes while each nominal key remains9 bytes. A shared child
completed at height255 is rechecked against each incoming prefix. These are
Originals-bound graph-kernel tests; they do not claim that the complete declaration
set passes the separate whole-original aggregate proof budget.

Qualification on one local host does not establish a hosted matrix or the full
accepted #416 scope. Final supported-platform gates, remaining language forms,
provider diagnostics, public activation, examples and support documentation are
still required before closing the ticket.
