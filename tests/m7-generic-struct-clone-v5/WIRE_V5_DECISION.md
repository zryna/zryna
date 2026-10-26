# Distinct private owned Struct Clone V5 proposal

Status: design and fixed test obligations only; no encoder/decoder/provider/compiler execution.

Choose exact header bytes `ZRYNA-GENERIC-OWNED-IR-V5\0` followed by little-endian u32 version5. Preserve the v4 envelope's field ordering, frozen embedded generic core-v1 bytes, effect rows and cleanup Step representation. Retain32MiB complete messages,1,048,576 envelope children and every inherited count/UTF8/vector limit, with checked preallocation accounting. Add only owned tag10: `CloneStruct { value:u32 little-endian }`. No type flag, shape, recipe, prefix count, helper selector or result ID appears in its payload; the enclosing extension retains resultId and the exact typed Unit placeholder.

Owned tags1..9 keep exactly their existing meanings and payloads in v5. Tags1..6 stay permitted in v2, tag7 remains v3/v4 shared Borrow<String> only, and tags8/9 remain v4 Struct construct/Copy field operations. All old codecs must still reject tag10; do not widen v4. Generic core-v1 tags remain frozen. There is no EnumClone opcode in this IR: finite enum clone is source-generated borrowed-match CFG with tag7 String leaves.

Return a distinct opaque DecodedOwnedV5, with immutable untrusted claims access only and no public raw constructor, From/conversion to old carriers or generic decoder fallback. The typed v5 issuer alone selects the internal admission token and performs complete independent source/type/plan checks. A raw envelope cannot supply a Clone capability or mutable side table. Existing scalar ABI/runtime layouts remain unchanged.

Required hostile controls, frozen before implementation:

- Unmodified v5 messages reject in v1/v2/v3/v4 decoders; unmodified old messages reject in v5.
- Rewrite both header and version to each old owned pair: tag10 still rejects. Rewriting only header or only version rejects too.
- Old v2/v3/v4 encoders reject a raw CloneStruct operation; no fallback encodes it as CloneString/CloneBorrowedString/CopyStructField.
- V5 accepts exact old tag payload meanings; v5 tag7 still rejects stored String, exclusive/ended/foreign loans and non-String referents at independent verification.
- Unknown tags0/11/255, a truncated tag10 operand, all truncation boundaries, trailing bytes, hostile vectors/blob lengths/child amplification and checked overflow reject without a partial carrier.
- A physically decoded CloneStruct operand may contain0xffffffff; source/type/plan verification then rejects unknown IDs. Decoder well-formedness alone grants no authority.
- Typed hostile clones reject scalar/String/borrow root substitution, changed Struct nominal identity despite equal width, mismatched result, missing/wrong Unit placeholder, moved source, exclusive loan, mutated closed field/enum records, and fabricated source operations. Shared complete-source loans remain legal when source Clone proof exists.
- Source clone(T), Box<T> and Hidden<T>{value:Option<T>} in unused templates remain M7002 at the exact clone keyword. A closed scalar specialization or inactive payload cannot launder that capability.
- Mutate any new retained recipe/prefix claim in private tests: it must be rejected or impossible to install through public APIs. A backend runtime progress cursor must never be accepted as a static initialized-prefix certificate.

There are no complete v5 wire fixture bytes yet. Those require a genuine source-produced claim after the test-first RED, and must be frozen before target qualification. Existing wire-v4.zir and all earlier wire fixture bytes remain untouched.
