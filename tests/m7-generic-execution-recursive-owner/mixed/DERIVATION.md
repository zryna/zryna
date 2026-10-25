# Mixed recursive catalogue source and pre-execution derivation

Status: proposed genuine source and independently authored source oracle only. No parser/provider, semantic/new-seal verification, emission, compiler build, guest, runtime, Cargo or test has run for this candidate by this designer. Existing source freezes/oracles are unchanged. Root chooses integration and owns actual evidence collection.

## Candidate and viability

mixed-catalogue.zry retains the complete11 lexical blocks from the already-valid referent-catalogue.zry, including the existing Borrow referents and Clone syntax. A separately declared generic nominal Node uses the same recursive Enum/Vec form as the original recursive-owner fixtures. Before the catalogue, a String leaf "n" is moved into one Vec<Node<String>>, then into an actual Node.branch owner. That owner remains live across all catalogue blocks; it is not merely an unused/erased layout type. A borrowed Clone of this Node is held until function exit. There are no loops, no arguments, no new source intrinsics, and the only return expression remains the literal i32 97.

Consequently the intended successful result is97 with actual owned Node<String> -> Vec<Node<String>> -> Node<String> type-graph recursion reachable from stored root/value/place ownership. Runtime depth is exactly1, yet arena activation must occur. The final successful exit first copies97 to its scalar result cell, then drops the cloned Node and original Node, directly checking the V2 cleanup/result scratch separation. The original three deep fixtures continue to own heap-depth qualification.

The unchanged catalogue executes Copy of bool and i32, String Clone, Pair<String> Struct Clone, Signal nominal Enum Clone, Option.some and Result.ok Enum Clone, FixedArray<String,2>, Vec<String>, SharedCount and WeakCount. The final Node adds recursive Enum/VecEach traversal through an active branch and leaf. All8 certified Clone kinds execute. Inactive Signal.quiet, Option.none, Result.err and recursive empty branches are deliberately not claimed covered here.

## Physical resource event derivation

An identity labels each successful positive physical allocation in source/effect order. Scalars and inline Struct/Enum/FixedArray wrappers add no allocation. Each nonempty StringFrom/StringClone adds one. Each nonempty Vec backing adds one. Each shared() adds one control allocation after constructing its String payload. Shared/Weak Clone and downgrade add canonical counts on the SAME control, without allocating a second control or cloning the String. Compiler frames/carriers add no runtime allocation.

| Scope | Acquisition IDs | Scope release order |
| --- | --- | --- |
| Persistent Node root | String1("n"), Vec2(Node,cap1) | deferred:1,2 |
| text | String3("text"), clone4(from3) | 4,3 |
| Struct | String5("field"), clone6(from5) | 6,5 |
| Signal.yes | String7("payload"), clone8(from7) | 8,7 |
| Option.some | String9("option"), clone10(from9) | 10,9 |
| Result.ok | String11("result"), clone12(from11) | 12,11 |
| FixedArray | a13,b14, clone-a15,clone-b16 | 16,15,14,13 |
| Vec<String> | element17, source-Vec18,clone-Vec19,clone-element20 | 20,19,17,18 |
| Shared<String> | payload21("shared"),control22 | 21,22 |
| Weak<String> | payload23("weak"),control24 | 23,24 |
| Final Node Clone | clone-Vec25,clone-leaf26(from1) | exit:26,25,then original1,2 |

Scope cleanup follows reverse declaration ownership, reverse inline field/index order and payload-before-container release. For the Shared scope, StrongClone22 raises strong1->2; dropping copied9 invokes StrongReleaseBegin22 and sees2->1, so no payload/finish; dropping root9 invokes begin1->0, releases String21 and finishes control22. For the Weak scope, control24 starts(strong1,weak1implicit), downgrade makesweak2, WeakClone makesweak3, copied/root Weak cleanup decrements3->2->1, then original owner begin1->0 releases String23 and strong finish removes implicitweak1->0/free24. These are count/status events but introduce no extra physical identity.

Pristine full acquisition IDs are1..26; full release IDs are:

`[4,3,6,5,8,7,10,9,12,11,16,15,14,13,20,19,17,18,21,22,23,24,26,25,1,2]`

There are11 nonempty StringFrom calls and9 nonempty StringClone calls,20 String releases,4 VecAllocate and4 VecReleaseStorage calls,2 control-birth Allocate calls,1 StrongClone,1 WeakDowngrade,1 WeakClone,3 StrongReleaseBegin,2 StrongReleaseFinish and2 WeakRelease calls. Thus60 logical canonical status calls if internal factoring is excluded consistently with the original recursive-owner oracle. These are source expectations, not observed counters. The allocation/release ledger never contains more than6 live physical allocations on the pristine route. Numeric raw pointers, target layout IDs, strides and control offsets must come independently from the admitted same-seal metadata; this oracle does not invent them.

## Fault meaning and full frozen rows

PRE_EXECUTION_ORACLE.json contains all9 StringClone fault rows and all26 positive physical Allocate fault rows, including exact acquisitions, complete release IDs, String projection and empty remaining ownership. Every fault returns canonical failure status1, becomes source ALLOCATION (Wasm getter0 status2), produces no scalar result, stops subsequent source scopes and must allow the existing clean reconfiguration gate followed by a pristine retry.

StringClone ordinals1..9 correspond to target physical identities `[4,6,8,10,12,15,16,20,26]`. They fail BEFORE entering that String's allocation attempt. The StringClone ordinal is not a Shared/Weak count ordinal and those operations cannot accidentally be failed by this seam.

Allocate ordinal1..26 refers specifically to the current private Wasm positive physical attempt counter shared between the String allocator and canonical Vec/control allocation. Configure it with0x22000000|ordinal. Thus Allocate22 means the Shared control birth, not the22nd source-level Allocate operation. StringClone faults use0x26000000|ordinal. The normal/clear control is0x20000001. Other targets must state their actual equivalent seam instead of assigning the same name to an unrelated role-specific counter.

A fault before pristine acquisition k has exactly IDs1..k-1 acquired. Previous catalogue scopes have already fully released in their frozen order. Clean the current scope's completed ownership in its source release order, excluding uninitialized/failed owners; then clean persistent Node IDs1,2 if they exist. The complete table is generated from the authored source resource groups, never from generated target code or observed traces. Sorting each row's releases equals its acquisitions exactly once; this is an oracle consistency check, not product execution.

Examples: Allocate1 has no releases; Allocate2 freespending String1. StringClone7 (second fixed-array element) frees clone-a15, then source b14,a13, then persistent1,2 after all prior scopes' releases. StringClone8 first frees empty cloned backing19, then source element17/backing18, then persistent1,2. StringClone9 frees destination backing25, then persistent leaf1/backing2. Control Allocate22 failure frees pending String21, then persistent1,2, after prior scope cleanup; control24 failure analogously frees pending String23. No failed destination field may be read and no failed physical identity may be released.

## Observable claims and proposed root integration

Retain this source/oracle/freeze before invoking either genuine provider. Collect actual TypeScript/native frontend packets from this exact source, independently verify both and issue fresh verified execution authority as existing driver controls do. Reject any source authentication/semantic error rather than replacing an oracle with generated output. For admitted executions, assert actual retained recursive arena and all8 actual certified Clone kinds/categories from the new seal, then ordinary and separate observed final-byte-audited Wasm. Run two pristine calls, every chosen fault, clear/retry, exact97 and status2/trap/no-result expectations. Check original source String handles/UTF8 bytes stay unchanged through every Clone/failure.

The current Wasm observer has a String-only bounded event stream: kind1 From, kind2 Clone, kind3 release, kind4 Allocate failure only when the failed attempt is in the String allocator, and kind5 StringClone failure. The JSON freezes those exact projected acquisition/release IDs, source identity and UTF8 bytes. Vec/control failures have no invented String failure record. Physical IDs in String records expose gaps, but gaps do NOT independently prove Vec/control release ordering. The clean gate confirms no residual counted allocation/busy state; it does not make an unrecorded full Vec/control trace visible. A full observer/native wrapper can compare the full ledger; current Wasm execution must label its evidence as the String projection plus gate/status/result. No public/full profile or all17-role fault coverage follows from this catalogue.

The candidate is a bounded proposal awaiting root integration choice. Do not change an expectation after actual observations to fit output; preserve failures and use a separately documented source-authority correction only if a concrete static derivation mistake is independently established.
