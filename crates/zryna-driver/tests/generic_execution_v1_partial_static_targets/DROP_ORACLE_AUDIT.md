# Pre-output partial drop oracle correction

The original source7046 and runtime068b/e5b25 freezes stay byte-for-byte immutable. Two pristine expectations were wrong: `struct-refill-whole` and `opaque-child` must release logical `[a,b,c]`, acquisition IDs `[1,2,3]`, rather than `[a,c,b]`/`[1,3,2]`. Root reports both stopped during target emission before any runtime output. This audit read source and accepted authority only; no emitted artifact/runtime observation was consulted or executed.

`spec/language/DATA_OWNERSHIP_V1.md:191-203` requires reverse completion order between roots and reverse declaration order within a Struct; moves transfer the pending obligation. `docs/M3_OWNED_DATA_SEMANTICS.md:242-249` distinguishes the pending-root stack from declaration/place/address order and says transfer renames the existing stack slot. Its `:493-504` recursively specifies reverse declaration field traversal.

For struct-refill, the owner root keeps its old pending slot while `taken=a` becomes a later pending root. Installing c into first restores Duo(first=c,second=b), and moving it to whole preserves the root slot. Return cleanup therefore releases taken=a first, then whole.second=b, then whole.first=c. For opaque-child<String>, the callee releases its separately taken=a before returning the restored Duo(first=c,second=b); caller cleanup releases b then c. The i32 instantiation has no destructor acquisition. This is the same rule, not a special generic exception.

All twelve cases/fourteen scalar rows were checked against source without reading target outputs:

| Case | Pending-root/recursive reason | Logical release order |
| --- | --- | --- |
| struct-survivor | later taken first field; remaining second frontier | a,b |
| struct-refill-whole | later taken; restored whole second then first | **a,b,c** |
| partial-root-replacement | commit drops old remaining second; later replacement second/first; taken | b,e,d,a |
| nested-disjoint | later sibling; earlier taken inner.first; remaining inner.second | c,a,b |
| array-survivor | later taken index0; remaining index1 | a,b |
| array-refill | later taken index1; whole highest restored index1 then index0 | b,c,a |
| opaque-child | callee taken; caller restored whole second then first | **a,b,c** |
| sibling-loan | block copied clone then taken; outer remaining second | b,a,b |
| failure-after-hole | joined result; copied second; taken first; remaining second | ab,b,a,b |
| branch-same-hole true/false | each arm drops taken first; outer remaining second | a,b |
| loop-refill | taken original a; taken first c; outer last c then second b | a,c,c,b |
| whole-vs-fields true/false | whole reverse second/first, or later field result before earlier | b,a |

Every failure prefix remains unchanged. Both corrected cases' third allocation is the replacement c: failure happens before restored-whole completion. Struct-refill then owns taken a and remaining b, producing a,b. Opaque-child fails while preparing the call argument c, so the already prepared Duo drops second b then first a. No clone/concat failure occurs in either corrected case. The correction changes only the two pristine recursive field orders and exact event suffixes.

Clarification for a canonical DropTree: order pending ROOT obligations first; for each chosen Struct node, visit declaration children in reverse regardless of their byte contents, place IDs, replacement history or runtime storage address. A projection replacement restores storage but does not reverse Struct declaration ordering. Preserve separate-root ordering and exact partial frontier rules.
