# JS/WASM adapter v1 examples and conformance plan

State: **specified-only**. This companion to the
[adapter contract](JS_WASM_ADAPTERS_V1.md) owns fixed future examples and delivery gates for
[#359](https://github.com/zryna/zryna/issues/359). The fixtures and declarations are design
oracles, not generated bindings or runtime execution evidence. Library decisions are separately
owned by the merged #358 contract linked in the alignment section below.

## Interface examples

These names describe an artificial test interface only. `add` reuses the scalar ABI fixture;
`roundTripText`, `roundTripI32s`, `createTextCell` and `TextCell` name no planned library API.
For this fixture alone, explicitly map `round-trip-text` to `roundTripText`,
`round-trip-i32s` to `roundTripI32s`, `create-text-cell` to `createTextCell`, and `text-cell` to
`TextCell`; `add` and `read` retain their names. General name mapping/collision checks remain a
later generation gate. Resource/string/list operations require the corresponding public bridge
and the corresponding #358 implementation gates if applied to a library. ESM declarations describe one initialized
instance; the loader format is deferred.

```typescript
declare const textCellBrand: unique symbol;
export interface TextCell {
  readonly [textCellBrand]: true;
  read(): string;
  dispose(): void;
}
export function add(a: number, b: number): number;
export function roundTripText(value: string): string;
export function roundTripI32s(values: ReadonlyArray<number>): number[];
export function createTextCell(value: string): TextCell;
```

The brand is declaration-only and cannot replace runtime provenance checks. `dispose` is
administrative; it does not add source unit. Boundary errors are thrown only after cleanup as
`AdapterError` with a closed `kind` from the outcome table below. A controlled language trap
additionally carries its exact existing trap identity; messages are bounded descriptions without
host paths, secrets or arbitrary exception strings. No generic TypeScript inference grants
source generics, structural resources or exact i32 range checking.

The following WIT fragment follows
[the #167 WIT grammar pin](https://github.com/WebAssembly/component-model/blob/2bed77e4228841c1d2721996d3ecc169ff96b158/design/mvp/WIT.md).
It defines no package/world, does not change #167's flat-world parser or registry, and is not a
claim that a component toolchain has parsed it. A later independent parser must validate the
complete resolved application world and generated component before execution.

```wit
interface sample {
  enum adapter-error {
    invalid-value,
    encoding,
    capacity,
    allocation,
    invalid-resource,
    busy,
  }
  resource text-cell {
    read: func() -> result<string, adapter-error>;
  }
  add: func(a: s32, b: s32) -> s32;
  round-trip-text: func(value: string) -> result<string, adapter-error>;
  round-trip-i32s: func(values: list<s32>) -> result<list<s32>, adapter-error>;
  create-text-cell: func(value: string) -> result<own<text-cell>, adapter-error>;
}
```

The implicit method receiver is borrowed; canonical drop consumes the owned handle separately.
This small sample has no controlled-trap-producing body. Its administrative error enum/result
does not map general source enums or Option/Result. A later language-trap-producing interface
needs an explicit versioned transport preserving the existing trap identities before admission;
it cannot use this sample's enum as a catch-all.

## Fixed outcome vocabulary

Names below are required future outcome categories, not assigned compiler diagnostic codes.
Implementation must allocate stable diagnostics at the owning boundary before conformance;
#167's `ZRYNA-C4000` through `ZRYNA-C4004` keep their existing meanings.

| Outcome | Phase / meaning | Recovery and observables |
| --- | --- | --- |
| returned | completed call, output copy and normal cleanup | exact type/value; no outstanding borrow or temporary allocation |
| invalid-value | ESM shape/arity/carrier validation or explicit checked bridge request | no target call on input rejection; no coercion or partial output |
| encoding | validation before canonical lowering or recoverable bridge byte ingestion | no replacement/normalization; unchanged original input, zero new live allocations |
| capacity | checked size/limit admission | no allocation past the bound; no truncation or partial result |
| allocation | admitted recoverable allocation request fails | reverse-free initialized prefix; preserve pre-existing owners |
| invalid-resource | checked provenance/liveness validation | no target use, transfer or second payload destruction |
| busy | checked binding observes active entry/borrow | no nested target entry or destruction |
| invalid-instance | call attempted after teardown/invalidation | no guest execution or cleanup retry |
| language-trap | verified language failure with existing exact identity | verified language cleanup; explicit transport required for the selected interface |
| fatal-boundary | malformed output, canonical/host trap, failed destructor/post-return or uncertain cleanup | no output commit; invalidate instance and wrappers; embedding reclaims storage/host resources |

At an untyped core boundary, malformed canonical encoding/index/type/range is fatal-boundary,
not a recoverable `adapter-error`. ESM prevalidation and a typed WIT call therefore need not
report the same transport for a malformed carrier that WIT cannot represent. They must agree on
no partial result, no unauthorized effect, and no duplicate release.

## Round-trip and independent malformed cases

Each row is a later fixture with independently fixed input/output, not an observed pass. All
normal cases run through both browser and Node ESM and the reviewed application component
interface. The raw malformed component cases require independently constructed core bytes,
not just outputs from the matching generator. Reference data lives in
[`js-wasm-adapter-v1-vectors.json`](../../tests/js-wasm-adapter-v1-vectors.json).

| Case | Input / action | Fixed expected outcome |
| --- | --- | --- |
| scalar-add | `add(20, 22)` | returned, typed i32 42 on JS and WASI consumer; no allocations |
| scalar-wrap | `add(2147483647, 1)` | returned, typed i32 -2147483648 |
| scalar-extremes | round-trip -2147483648, 0 and 2147483647 | returned, same exact i32 value |
| scalar-invalid | -0, fraction, NaN, infinities, out-of-range, BigInt, string, boxed Number or wrong arity | invalid-value before ESM target entry |
| bool-round-trip | false and true | returned, same bool; use bool-admitting profile |
| bool-invalid | 0, 1, null or truthy object at ESM boundary | invalid-value; never truthiness conversion |
| unicode-round-trip | empty, NUL, initial BOM, accent, supplementary scalar, decomposed accent | returned, exact vector scalar sequence and UTF-8 bytes; no normalization |
| surrogate-input | lone high/low UTF-16 surrogate | encoding before ESM lowering; no call |
| malformed-utf8 | overlong, truncated, encoded surrogate, above-Unicode-limit and isolated continuation vectors | encoding in checked byte ingestion; fatal-boundary in raw canonical lifting |
| list-round-trip | empty and `[20, -1, 22]`, `[true, false]`, `["A", "é"]` | returned, equal values in independent storage; input mutation cannot alter retained language copy |
| list-malformed | hole, inherited/accessor element, null element, wrong element type or typed array carrier | invalid-value before ESM target entry; no getter invocation for ordinary accessor elements |
| nullability | null/undefined/omission for each nonnullable row | invalid-value; no empty value or handle sentinel substituted |
| input-prefix-failure | list-string copy allocates outer storage + element 0; element 1 allocation fails | allocation; drop element 0, then outer storage; original host list unchanged |
| output-copy-failure | complete language result; recoverable host-copy failure after copied element 0 | allocation; discard copied prefix; release language result and eligible transfer buffers; no published list |
| create-publish-failure | payload prepared, then recoverable wrapper/registration preparation fails before own transfer | allocation; payload destroyed once, no owning handle published |
| malformed-result | foreign result contains invalid UTF-8, out-of-range/alignment pointer, bad discriminant or wrong exact type | fatal-boundary; no output; no assumed post-return after failed lift |
| canonical-allocation-trap | canonical realloc traps midway through nested lowering | fatal-boundary; discard instance memory; no rollback/reentry assumption |
| post-return-trap | complete lifted result but selected post-return traps | fatal-boundary; no wrapper output commit; embedding invalidates instance |
| version-mismatch | stale declaration/interface digest, resource identity or WIT/WASI version | reject before load/call; no version fallback |
| unknown-interface | unknown type, owner mode, export mapping or incomplete metadata | reject before generation; no guessed mapping |

The scalar vectors in [scalar ABI v1](../abi/scalar-v1-fixtures.json) remain the shared authority
for existing ESM/raw scalar carriers. Raw core scalar bool accepts only 0/1; test WIT bool using
the pinned Canonical ABI rules instead of claiming those are the same raw boundary.

## Lifecycle and cleanup traces

Use the pure illustrative TextCell payload, not a host descriptor. `create("A")` returns one
owner; `read` copies "A"; `dispose` drops it once. The normal resource count returns to zero.
For host resources in later #358 work, add the corresponding exact permission and host-owned
reclamation oracle without changing these generic ownership rules.

| Case | Trace / adversarial action | Fixed expected outcome |
| --- | --- | --- |
| resource-normal | create -> read -> dispose | returned "A"; one payload destroy; zero live owners/borrows |
| stale-use | create -> dispose -> read through alias | invalid-resource before ESM body; no new destructor |
| stale-after-reuse | dispose old wrapper; create new resource using reused private slot; use old alias | invalid-resource; new owner untouched; wrapper identity cannot repeat |
| double-dispose-js | create -> dispose -> dispose through alias | both disposals return normally; exactly one destructor |
| double-drop-wit | drop owner twice | typed consumer rejects use-after-move; absent raw canonical index traps; no second payload free |
| wrong-instance | pass wrapper from instance A to B, including same resource spelling | invalid-resource; both owner tables unchanged |
| forged-resource | object with copied prototype/properties or serialized numeric index | invalid-resource; no target entry |
| borrow-escape | retain use receiver after dynamic call or attempt release while borrowed | verifier/checked binding rejects; illegal canonical use traps; no use beyond borrow scope |
| reentry | conversion/import attempts call or dispose on active instance | busy before nested body; outer owner unchanged on recoverable rejection |
| destructor-failure | destructor throws/traps after invalidation | fatal-boundary; no retry, including via second dispose |
| callback-input | function passed as scalar/list/resource or proposed callback declaration | invalid-value for carrier; unsupported declaration before generation; zero callback registrations |
| async-output | Promise/thenable used as result | fatal-boundary; no await or then invocation; invalidate instance |
| worker-transfer | structured clone or foreign-thread use of resource | reject carrier before target entry; no new owner |
| teardown | close idle instance with unreleased owner; call old wrapper | reclaim owner once; invalid-instance on later call |

For recoverable list preparation, a private later trace should be `allocate outer`,
`allocate element 0`, `fail element 1`, `free element 0`, `free outer`. The unchanged original
is not in that cleanup list. Inject failure at every preparation site and at output publication;
count allocations, successful frees, payload destructions, live registrations and active borrows
independently of producer cleanup metadata. A successful retry creates a fresh identity.

For fatal failure, the oracle is instance invalidation plus embedding-owned reclamation, not
language destructor execution. Record that guest reentry/destructor attempts stop, known host
registrations close once, and storage is discarded. If reclamation cannot be established, report
fatal-boundary and mark the embedding conformance gate failed; do not report a successful cleanup.
Externally terminated processes have no guaranteed language cleanup and require host isolation
and reclamation proof before an affected resource adapter can be activated.

## Portable resource bounds

These are proposed adapter-v1 maxima, not changes to M3 limits or #167 quotas. The effective
limit is the minimum of this table, verified language/runtime admission, the exact interface,
and the host policy. Dependencies share an instance's bounds under #357; they cannot multiply
quotas. Preflight checked arithmetic before allocations or body entry, including all nested
payloads/transfer buffers; exclude host-engine object header sizes from portable byte accounting.

| Metric | Maximum | Exact-limit / first-extra oracle |
| --- | ---: | --- |
| UTF-8 bytes in one string | 1,048,576 | valid sequence at limit; one ASCII byte extra: capacity |
| elements in one list | 65,536 | valid list at limit; one valid element extra: capacity |
| simultaneous conversion bytes per call | 4,194,304 | sum live input/output payload and transfer storage across domains; one byte extra: capacity |
| live resource payload bytes per instance | 8,388,608 | sum all payloads; one byte extra: capacity without changing old owners |
| live resource owners per instance | 1,024 | small payload owners at limit; owner 1,025: capacity |
| successful resource creations per instance | 65,536 | repeated create/drop at limit; creation 65,537: capacity, no identity wrap |
| active entry per instance | 1 | one call allowed; nested/parallel second entry: busy |

Per-string and list limits do not override aggregate conversion bytes. Test each bound in
isolation with other bounds satisfied, and combinations where nested copies exceed aggregate
bytes while every individual value fits. Reserve output/registration capacity before ownership
commit. Resource counts include retained children of affected host resources and remain subject
to #167's stricter descriptor/timer/network limits. Fatal canonical exhaustion follows the trap
rule even if logical adapter byte budgets admitted the request.

## Implementation ledger

The implementation states below are narrower than the delivery slices. A private compiler
checkpoint does not make the overall specified-only adapter contract or a host row public.

| Boundary | State | Exact implemented evidence | Still separate |
| --- | --- | --- | --- |
| Private scalar ESM interface | implemented-private by #381 | authenticated `ControlFlowV1` graph -> sealed scalar ABI -> byte-compared deterministic ESM; exact revision/profile/browser-or-Node pure policy, ordered exports, source graph, interface and artifact identities; independent forged/stale/limit/recovery rejection | generated declarations/loaders, real browser/Node consumer conformance and public activation |
| Private scalar host consumer | local candidate by #387; conformance unrun | retained-ABI invocation and retained-byte Node ESM import; shared strict carrier corpus and separately scheduled pinned real-browser resource test | reviewed browser archive/inventory pins, executed Node/browser proof, M2 regression and required gates; package-backed composition and public activation |
| Non-scalar ESM conversions | specified-only | design vectors only | every public String/list/resource language, ABI, cleanup and consumer gate |
| WIT/Component adapters | specified-only | #167 worlds and design matrices only | reviewed application world, component emission/parser/host enforcement and WASI consumer proof |

The #381 view is private and JavaScript-bound: unsupported core-WebAssembly, native, WIT and
Component target claims are rejected rather than assigned names or policies. Its retained exact
ESM source and sealed scalar export view are the internal seam for the next real Node/browser
scalar consumer; they do not imply Windows or macOS native ABI decisions.

The #387 candidate uses the pure non-package specialization of #357: one authenticated complete
`ControlFlowV1` source closure and an explicitly verified empty host-interface request. It does
not copy #380 metadata claims into source authority. Host-specific seals may share immutable ESM
bytes while retaining distinct interface/binding identities. The
[scalar host conformance lane](../../tests/scalar-host/README.md) records exact dependency pins,
pending acquisition review, transport costs and required nonzero Linux/Windows resource-test
commands. An ignored browser test or null acquisition pin is not conformance evidence or #387
completion; I1, C1 and public activation remain open.

## Separate consumer proofs and delivery slices

| Slice | Exact prerequisites | Independent completion evidence |
| --- | --- | --- |
| I1 sealed scalar interface and declaration generation | accepted #357 cross-target profiles v1 + #167; scalar ABI/verified profile; reviewed interface identity/binding format | exact exported declarations and scalar outcomes; malformed names/arity/carriers and version replay reject |
| I2 one owned string/list bridge | I1 + public conversion/language/ABI gate; only corresponding accepted #358 API when library-facing | positive deep-copy round trips, every malformed encoding, checked bounds and every preparation/output failure site |
| I3 one pure resource bridge | I1 + resource identity/nonescape/destructor authority; #358 reconciliation only for affected library payload | create/use/drop, wrong instance, stale-after-reuse, double-release, reentry, teardown and allocation ledger |
| I4 one WASI consumer | #167 + accepted #357 composition + reviewed versioned application world; independent component parser/emitter/host enforcement; I2/I3 only for conversions used | exact imports/exports and granted/denied enforcement; malformed component values and fatal reclamation |
| C1 adapter conformance | relevant I slices and pinned browser, Node and component runtime/toolchain matrix | all claimed host rows execute their positive, negative, deterministic, limit and lifecycle corpus; required repository gates |
| A1 public activation | C1 + reviewed selectors/formats, compatibility/migration and authenticated documentation | explicit supported boundaries; no inference from a local prototype or declaration |

The first portable computation is `add(20,22) -> i32:42`, plus wrapping overflow and scalar
negative carriers. A future browser ESM page and Node ESM consumer import the same exact verified
interface and assert the values without console output as an ABI. The WASI proof uses a reviewed
command/application arrangement whose `run` compares 42 internally and returns its typed success
or error; process exit status is harness health only. No stdout/environment grant is needed for
that computation. Its host supplies only exact required #167 interfaces under denied/default
policy; unused eligible imports convey no permission to perform effects.

The empty pinned browser world cannot export `add`; a separate reviewed application world is
required for a browser component proof. The pure core computation can be checked through existing
JS/core-Wasm/native gates independently; such results do not substitute for browser or WASI
consumer execution. Text/list round-trip and TextCell proofs are a second stage after their
specific bridge gates, not prerequisites for the scalar seed.

For browser, Node and WASI separately pin tool versions and record artifact/interface digest,
request/grant policy, value or error, call count, allocations/frees, live owners/borrows and
teardown. Browser tests must exercise a real browser; Node's WebAssembly API is not that
proof. Include command environment/ filesystem denial and server environment/filesystem denial,
plus exact #167 grants for any host API actually tested. No unversioned host or npm compatibility
claim is allowed. Current Linux/Windows M0 and relevant target gates remain required; this design
does not execute or weaken them.

## Accepted library alignment

This alignment consumes [minimal core and host libraries v0](../libraries/MINIMAL_CORE_HOST_V0.md),
accepted by [#358 / PR #375](https://github.com/zryna/zryna/pull/375) at
`823a76d4792460b478ae239140aab6cfe92f1f9a`. C/A/H, O/E and F labels refer to that specification,
not new adapter exports. #167 retains exact WIT interfaces/grants; accepted #357 owns profile
composition. The library/adapter design alignment is complete; the remaining gates below concern
implementation and additional language/interface admission, not acceptance of #358.

| Library APIs | Adapter alignment decision | Remaining implementation gate |
| --- | --- | --- |
| C1-C3 | Exact i32/bool carriers, wrapping addition and eager argument order align; preserve declared export names rather than rename the sample interface | separate source/profile verification and adapter generation; no host requirement |
| A1-A5 | String and exact Vec<i32> copies retain inputs; push remains an internal exclusive mutation, not a new public method | any public bridge used; E1/E2 remain non-catchable language allocation/capacity/bounds traps with controlled cleanup, not recoverable adapter errors |
| H1 | Copy key/result text; distinguish missing from empty text without null/undefined | F1/F2 concrete found/missing and operation-error transport; accepted environment grant and byte bounds; sample adapter-error does not encode missing |
| H2 | Copy complete UTF-8 text, release all acquired descriptors through their owning host, publish no prefix | F1/F2 outcome/resource authority, exact preopen/path/grant enforcement and input/output limits; all effectful JS operations remain unavailable in adapter v1 |
| H3 | Status plus text body needs a concrete result shape; network eligibility does not provide command HTTP | F1/F2 shape/error acceptance; no implicit record conversion; asynchronous realizations including browser fetch also need F4 and an adapter/API revision |
| H4 | Unsigned 64-bit timestamp has no admitted adapter-v1 carrier | F1/F2/F3 exact u64 language/ABI/carrier contract; never truncate to i32, invent lanes or silently admit BigInt |
| H5 | Exact Vec<i32> carrier can describe copied byte-valued elements, but generic list validation is insufficient | F1/F2 outcomes; validate count 0..256 and every element 0..255, exact output length and explicit entropy grant/quota even for count zero |
| other conversion rows | list-bool, list-string and the illustrative resource remain low-level design cases without a matching v0 library API | no inferred library API or expansion of #358; accept only if a corresponding API is separately reviewed |

O4 aligns with call-scoped input authority, one result owner, paired-domain temporary
release and independently confirmed host-resource reclamation. Recoverable host operation errors
are distinct from language traps and fatal interface/host failures. The sample WIT error enum is
not E3's operation-specific transport. F2 must define that transport before generation; failure
to confirm disposal is fatal, never a recoverable absence. This alignment review adds no async,
callback, host-operation, unsigned-integer or source-outcome admission to #359 v1.

H1-H5 preserve E3's operation-specific `invalid-input`, `not-found` where applicable,
`permission-denied`, `quota-exceeded`, `invalid-encoding` and `io-failure` outcomes, separately
from typed language traps, interface violation and host/process failure. H1 missing is a normal
absence, not a failure or empty string. Reject unknown status values; do not translate E3 into
the illustrative WIT `adapter-error` enum. Malformed raw file/response bytes at an admitted
recoverable library decode boundary yield invalid-encoding; malformed data at an interface
declared as text instead triggers the adapter's fatal invariant-failure rule.

| Library oracle | Exact later consumer proof |
| --- | --- |
| H1 / Q9 | key 1..64 UTF-8 bytes; value 0..1024 bytes; found empty text differs from missing; 1025-byte value is quota-exceeded with no truncation |
| H2 / Q10 | authorized relative path 1..1024 UTF-8 bytes; complete text at most 4096 bytes; 4097 is quota-exceeded; raw bytes [195,40] are invalid-encoding at library decode; release descriptors |
| H3 / Q11 | endpoint/path total 1..1024 UTF-8 bytes; status 100..599 and body at most 4096 bytes; 404 is a response, 302 is returned without redirect; 4097-byte body is quota-exceeded and response released |
| H4 / Q12 | preserve unsigned timestamp 4294967297 exactly after F3; current adapter-v1 interface admission rejects u64 before generation; omitted clock grant means zero clock calls |
| H5 / Q13 | counts 0 and 256 accepted only with grant/quota; -1 and 257 yield invalid-input with zero entropy calls; count 3 stub returns [0,127,255]; reject wrong length or any result element outside 0..255 |

All H calls retain input/authority owners until call cleanup, transfer only one complete result
owner and never retain a borrowed callback or memory view. A failure after an external effect
does not undo it and causes no automatic retry. Successful or recoverable paths release temporary
descriptors/response resources; canonical fatal paths invalidate the instance and rely on
embedding-owned reclamation without guest cleanup reentry. ESM dispose remains idempotent;
repeated canonical owner drop remains invalid. #364 independently owns native release and failure
rules: this alignment creates no native wrapper, shared native policy or C ABI dependency.
