# Typed JavaScript and WebAssembly adapter contracts v1

Contract identity: `zryna.js-wasm-adapters.v1`. State: **specified-only**.
This is the bounded M4 design for [#359](https://github.com/zryna/zryna/issues/359), under
[#356](https://github.com/zryna/zryna/issues/356). It specifies future interfaces, not an adapter
generator, runtime, compiler admission, public selector, package, or support claim.

## Authorities and acceptance dependencies

The compiler verifies source types, effects, interface identities and ownership before a backend
or adapter receives sealed views. The driver binds those views to the artifact and deployment
policy; adapters validate foreign values and enforce grants. A declaration file is a consumer
description, never a substitute for runtime checking or the mandatory IR verifier.

| Decision | Authority / prerequisite | Acceptance boundary |
| --- | --- | --- |
| Scalar carriers | [scalar ABI v1](../abi/SCALAR_V1.md); selected I32V1 or ControlFlowV1/DataOwnershipV1 gate | Reuse exact `i32`/`bool` rules; bool requires a bool-admitting profile |
| WIT worlds, capabilities and WASI | [#167 contract](../wit/CAPABILITY_PROFILES_V1.md) | Exact identities, imports, ceilings and request validation remain authoritative |
| Target/profile/dependency composition | Accepted [cross-target profiles v1](../language/CROSS_TARGET_PROFILES_V1.md), #357 | Exact `zryna.cross-target-profiles.v1` composition authority; backend/host selection never grants authority |
| Each library-facing conversion | Accepted [minimal core and host libraries v0](../libraries/MINIMAL_CORE_HOST_V0.md), #358 | Per-API alignment below preserves its ownership/errors and remaining F1-F4 implementation gates; no API IDs are assigned here |
| Public non-scalar interface | This contract plus separately verified language/interface lowering | Internal M3 operations do not admit public String, Vec, aggregate or borrow parameters/results |
| Package compatibility | [#168 contract](../package/PACKAGE_RELEASE_V1.md), then #360 for instance compatibility | Source-package acceptance does not establish binary/interface compatibility |
| Native C / frontend replacement | #364 / M7 | Neither is a prerequisite for this design or its portable scalar proof |

The current [public M3 surface](../../docs/M3_PUBLIC_PROFILE.md) still exposes only `i32`/`bool`.
The [M3 language](../language/DATA_OWNERSHIP_V1.md) and
[ownership runtime](../abi/OWNERSHIP_RUNTIME_V1.md) retain compiler-private String/Vec/Shared/Weak
layouts. No pointer, capacity, refcount, control block, allocator object, layout fingerprint or
internal helper becomes a public adapter ABI. M3 gates and historical authenticated inventories
are unchanged. This design does not require every #358 API or all of M7 to finish together.

## Separate deployment contracts

All rows below describe future adapter availability. The existing `.mjs` and core `.wasm`
artifacts retain their current contracts; neither becomes a component by adding declarations.

| Boundary | Interface and compatibility identity | Consumer assumption | Availability |
| --- | --- | --- | --- |
| JS-BROWSER | ESM exports + exact adapter/interface revision | Browser module realm; no DOM, fetch or storage supplied implicitly | specified-only |
| JS-NODE | ESM exports + exact adapter/interface revision | Pinned Node module realm; no CommonJS, npm resolution or process globals supplied implicitly | specified-only |
| WIT-BROWSER | `zryna:capability-profiles/browser@0.1.0` | Empty imports and exports; current core wasm-web is not this world | specified-only |
| WIT-COMMAND | `zryna:capability-profiles/command@0.1.0` | Exact `wasi:cli/run@0.2.12` export and #167 imports | specified-only |
| WIT-SERVER | `zryna:capability-profiles/server@0.1.0` | Exact `wasi:http/incoming-handler@0.2.12` export and #167 imports | specified-only |

The pinned package is `zryna:capability-profiles@0.1.0`, with WASI `0.2.12`.
The following capability matrix refines the accepted #357 contract's eligibility ceilings. `D` means denied; `E`
means eligible only through the exact accepted interface, explicit transitive request, host
grant and mediated operation. An omitted grant is denied. `E` does not activate an operation.

| Boundary | clock | environment | filesystem | network | randomness | Exact operation status |
| --- | --- | --- | --- | --- | --- | --- |
| JS-BROWSER | E | D | D | E | E | No effectful JS operation admitted in v1; nonempty requests reject |
| JS-NODE | E | E | E | E | E | No effectful JS operation admitted in v1; nonempty requests reject |
| WIT-BROWSER | D | D | D | D | D | Exact empty #167 world |
| WIT-COMMAND | E | E | E | E | E | Only #167's exact command interfaces; #358 host wrappers retain their implementation gates |
| WIT-SERVER | E | D | D | E | E | Only #167's exact server interfaces; #358 host wrappers retain their implementation gates |

Equal capability names do not make command socket interfaces compatible with server HTTP
interfaces. A browser JS grant cannot widen the empty WIT browser world. Node used to compile or
test a program supplies no deployment authority. Additional effects, including standard I/O,
dynamic imports, process launch and DOM access, are outside this adapter contract. Arbitrary npm
code is neither a verified dependency nor an isolated capability provider.

Apply #357's transitive closure, per-dependency restrictions and explicit root approval before
IR construction; a pure declaration cannot conceal an effectful child. The host separately
validates grants before execution and mediates each operation, including revocation and quotas.
Bind adapter authority to that verified graph, profile, target and host policy; changed inputs
invalidate it. Shared dependencies do not multiply per-instance resource limits. These accepted
composition rules do not activate an effectful JS operation or widen a WIT world here.

#167's world exports cannot be extended in place. The application WIT fragment below is a type
design only: it is not inserted into the pinned worlds and has no assigned package/world identity.
A later component proof needs a separately reviewed, versioned application world and corresponding
#167 authority revision where required; it must preserve the existing registry and denials.

## Closed interface shape and conversions

A future compiler-owned interface view binds: contract revision, selected language profile,
target/host row, exact application interface identity, ordered named functions, resource type
identities, parameter/result types and modes, error vocabulary, limits, and exact required host
interfaces. The driver binds it to source/graph and artifact identities. Unknown fields/types,
duplicate or colliding names, missing ownership modes, unbounded declarations and stale identities
reject before generation. This is a required sealed view, not a new serialized package format.

Functions have fixed arity, at most 256 parameters, and one logical result. Scalar-only functions
retain scalar ABI v1 names and validation. New resource operations reserve their explicitly mapped
names; collisions reject rather than receive suffixes. Admission is the closed table below.
Library use follows the [accepted #358 alignment](JS_WASM_ADAPTER_CONFORMANCE_V1.md#accepted-library-alignment),
independently of low-level representation; a specified library API does not activate a public bridge.

| Value | ESM input / output | WIT input / output | Copy or lifetime rule | Required language/interface gate |
| --- | --- | --- | --- | --- |
| i32 | primitive `number`, integral finite signed 32-bit, excluding negative zero | `s32` | value copy; no allocator | scalar ABI v1 + selected profile + verified adapter export |
| bool | primitive `boolean` only | `bool` | value copy; no allocator | ControlFlowV1 or DataOwnershipV1 + scalar ABI v1 + verified adapter export |
| string | primitive `string` containing only Unicode scalar values | `string`, UTF-8 canonical option | complete independent copy in each direction | accepted public String conversion/lowering; M3 private String alone is insufficient |
| list-i32 | dense `ReadonlyArray<number>` in, fresh `number[]` out | `list<s32>` | snapshot then copy; never a shared memory view | accepted public exact Vec<i32> conversion and bounded allocation/cleanup |
| list-bool | dense `ReadonlyArray<boolean>` in, fresh `boolean[]` out | `list<bool>` | validate every element; independent copy | accepted public exact Vec<bool> conversion and bounded allocation/cleanup |
| list-string | dense `ReadonlyArray<string>` in, fresh `string[]` out | `list<string>` | deep copy; reverse initialized-prefix cleanup | accepted public exact Vec<String> conversion and nested fallible cleanup |
| resource | opaque instance-bound object; creator output, method receiver, explicit dispose | creator returns `own<R>`; use takes `borrow<R>`; canonical drop consumes owner | one owner; call-scoped borrow; no implicit clone | accepted resource identity, nonescape, verified bridge and audited destructor |

No nullability is implicit. `null`, `undefined`, omitted arguments and holes reject for every
value row; empty string/list is valid and distinct from absence. No pointer or handle sentinel
encodes absence. General `option`, nullable values, records, tuples, source enums, nested lists,
lists of resources, arbitrary objects, `Shared<T>`/`Weak<T>`, public language borrows, user generics,
`u8`/`u32`/`i64`/floats/BigInt, variadics, futures, streams and function-valued parameters/results
are excluded. WIT types used inside #167's imported WASI signatures are not thereby source types.
Administrative no-result drop and fixed adapter error results do not activate source unit or
general Option/Result syntax; those need their own relevant language gates.

### Encoding and validation order

ESM validation occurs on both inputs and outputs, with exact arity and no coercion (`ToInt32`,
truthiness, boxed primitives, stringification and iterable conversion are not validation).
Strings preserve scalar sequence exactly: no normalization, replacement character insertion or
NUL termination. Embedded U+0000 and an initial U+FEFF are data. Reject lone UTF-16 surrogates
before encoding; reject overlong, truncated, surrogate and above-U+10FFFF UTF-8 before copying
into a language String. Length and budgets count UTF-8 bytes, not JS code units. An encoder that
silently replaces a surrogate or a decoder that strips a BOM does not meet this contract.

Lists capture length once and inspect own data elements in ascending index order under the
instance entry guard. No holes, inherited elements, accessor elements, coercion, typed arrays,
iterators or shared buffers are accepted as list carriers. Snapshot all values into private
storage, validate, then lower; subsequent host mutation cannot affect a callee. Proxy inspection
can itself execute JS: the adapter must contain thrown inspection failures and block reentry,
but promises neither universal Proxy detection nor rollback of hostile host-code side effects.
The host realm and intrinsic functions must be trusted. This is not a sandbox for arbitrary JS.

Validate identity/instance state, arity, then inputs left-to-right (list indices ascending),
checked size arithmetic and budgets, preparation, call, output validation/copy, then cleanup.
Release the entry guard only after cleanup. Invalid input never enters the target body. Invalid
output publishes no value. Allocation fault injection is indexed by the resulting preparation
order; no success result may depend on a partially initialized output.

### Component lowering boundary

Use the synchronous, memory32, unshared-memory subset of the
[Canonical ABI at #167's grammar commit](https://github.com/WebAssembly/component-model/blob/2bed77e4228841c1d2721996d3ecc169ff96b158/design/mvp/CanonicalABI.md).
The selected memory, realloc and post-return options belong to the exact component instance;
string encoding is UTF-8. Async, memory64 and shared-memory options reject for this slice.
Canonical carriers/validation follow that pin; application checks may be stricter but must not
reinterpret its lowering. In particular, WIT `bool` is not the raw scalar-ABI-v1 wrapper.

Canonical transfer buffers are separate from private M3 storage. The generated bridge copies
between them through verified conversion operations. Core pointers/lengths/indices are private
implementation details, never JS API arguments. Checked ranges, alignment, size multiplication
and aggregate budgets precede access; a memory growth invalidates cached host views. No borrowed
linear-memory view survives a call, allocation, growth or result cleanup.

## Allocation and operation ownership

`J` is host-managed JS storage; `T` is adapter-owned temporary storage; `L` is language-owned
storage behind the verified private runtime; `C` is Canonical ABI transfer storage allocated in
the selected receiving linear memory. These are ownership domains, not published layouts.

| Operation | Input -> output representation | Allocator / owner after success | Borrow interval | Failure and cleanup responsibility |
| --- | --- | --- | --- | --- |
| scalar call | ESM primitive or WIT scalar -> same exact scalar type | none; caller receives value | call only; no retained input | input error: no call; invalid output: fatal boundary failure |
| copy-in | J/WIT string or list -> T snapshot -> L copy (through C for a component) | each domain uses its own allocator; callee owns committed L input | host input inspected only until snapshot completes | adapter releases T; bridge frees uncommitted L prefix and its owned C; caller's original stays unchanged |
| copy-out | L result -> C/WIT value or J deep copy | bridge owns L until copied; host owns final J/WIT value | L/C bytes visible only to trusted conversion until cleanup | no partial output; release completed temporary copies and original L/C through their owners |
| create-resource | copied input -> new opaque J object or `own<R>` | resource implementation allocates payload; adapter/runtime owns registration until output commit, then caller owns handle | copied input cannot escape as a borrow | before commit destroy new payload and registration; no handle published; trap follows fatal teardown rule |
| use-resource | J receiver or `borrow<R>` -> copied/scalar output | original owner retains payload; result has copy-out ownership | entry validation through synchronous call and cleanup; no retention | recoverable error ends borrow, retains owner; fatal failure invalidates instance |
| release-resource | J dispose or canonical owner drop -> no result | implementation's paired destructor releases payload; registration consumed once | requires no live borrow and idle instance | invalidate before destructor; destructor failure is fatal, never retry it |
| teardown-instance | all outstanding registrations -> no callable instance | embedding reclaims instance storage and host-owned registrations | reject teardown while a normal call is active | fatal teardown does not reenter guest cleanup; externally owned resources require independent host reclamation |

Pair allocation/free within its domain and instance. JS values are collected by the JS engine;
GC timing is never a resource-release guarantee. T and L allocations have exactly one matching
private release, including completed elements in reverse order before their outer list storage.
C input buffers use the receiving memory's canonical realloc for allocation; the generated
receiving bridge assumes their release obligation on normal entry and releases them with that
same allocator's private deallocator after copying. For a component-produced result, its selected
post-return hook releases result transfer buffers after successful canonical lifting. For a host
import result lowered into guest memory, the receiving guest bridge releases that C storage after
copying. No caller frees another instance's memory or guesses a public `free` symbol. There is no
user-selected allocator or extra public free API.

Recoverable preparation failures occur before canonical lowering, or inside a verified bridge
with explicit recoverable operations and known ownership. They free only the initialized prefix
and leave pre-existing owners intact. Once an `own<R>` transfer or canonical lowering has started,
do not promise transactional rollback of arbitrary canonical operations. A canonical realloc,
lifting, destructor or post-return trap is **fatal-boundary**: discard the instance, invalidate
all wrappers, reclaim its memory and independently tracked host resources through the embedding.
Do not assume post-return runs after failed lifting or call guest destructors to recover a trapped
instance. No cross-boundary unwinding, longjmp or JS exception propagation defines language cleanup.

Output commit is after complete validation and all required normal cleanup. For ESM, translate a
recoverable boundary error into a fixed `AdapterError` only after cleanup; do not expose arbitrary
host exception text. On a typed WIT operation use its explicit fixed error result; malformed raw
canonical data traps rather than becoming that recoverable result. Controlled language traps
retain their existing typed identity and cleanup; an accepted future bridge must transport them
explicitly without confusing them with a host exception. A failure after a host effect cannot
undo that effect or authorize a retry. The accepted #358 H1-H5 policy performs no automatic retry
and returns no partially converted data; normal cleanup and fatal teardown remain distinct.

## Resource lifecycle, callbacks and threading

Resource names denote nominal interface types bound to one instance and interface revision.
The ESM object has private provenance and a checked, non-reused identity (or a checked generation
that retires before wrap). Object aliases share one owner state; aliasing does not clone ownership.
Creating an object with the same properties/prototype is not authority. Resources cannot be
serialized, structured-cloned, transferred to another realm/worker/instance or cast to integers.
Generic ownership adoption, caller-supplied raw handles and resource clone are excluded.

| State / operation | ESM outcome | Typed WIT / canonical outcome | Owner / cleanup |
| --- | --- | --- | --- |
| absent / create succeeds | fresh live object | fresh `own<R>` | one payload and one owning registration |
| absent / prepare fails | recoverable error, no object | explicit error, no owner | all new prefix allocations freed |
| live / use | copied value; state returns to live | call-scoped `borrow<R>`; owner retained | end borrow after call; no payload drop |
| live / release | success; state released | consumes `own<R>` | payload destructor once; owner registration removed |
| released / use | invalid-resource before target entry | typed consumer use-after-move rejects; invalid absent raw index traps | no new destructor invocation |
| released / release | success, idempotent no-op | second owner drop is invalid; absent raw index traps | no second payload free |
| borrowed or active / release or reentry | busy before nested target entry | checked binding rejects; illegal canonical drop traps | no destruction while lent; trap invalidates instance |
| wrong type, instance, realm or forged object / use | invalid-resource before target entry | typed mismatch rejects; invalid canonical type/index traps | no ownership transfer |
| invalidated instance / any call | invalid-instance, including dispose | embedding rejects calls | no guest cleanup retry; teardown owns reclamation |

The raw Canonical ABI uses instance-local table indices, not public generation-tagged handles.
An absent index traps; a recycled integer alone cannot prove freshness. Typed consumers must
invalidate moved/dropped owners, and checked JS wrappers retain independent identity across slot
reuse. This contract does not claim that an arbitrary hand-written core module using a recycled
valid index is diagnosed as stale. Host safety must rely on the typed boundary, table isolation
and capability enforcement, never secrecy of numeric indices.

All operations are synchronous on one owning realm/thread, with at most one active entry per
instance through final cleanup. No concurrent use, worker transfer, atomics, shared memory or
reentry during imports, allocation, conversion, drop or post-return is admitted. Binding checks
must acquire the guard before inspecting foreign objects. Cross-instance calls that would cycle
back into an active instance fail before nested entry. Trusted administrative cleanup executes
within the active guard; it is not a user-call exception to the rule.

Callbacks are excluded in v1, including synchronous function arguments, retained callbacks,
event listeners and async callbacks. The adapter creates no callback registration and may not
invoke or retain a supplied function. Imports are fixed verified interface operations, not
arbitrary function-valued application data. Promise/thenable results reject without awaiting or
reading `then`; no borrow extends into a microtask. A future callback contract must separately
specify registration/release, captures, nonescape, cancellation, reentry and its language gates.
Host loader initialization may use an external asynchronous API, but no instance or borrowed
value is exposed until initialization succeeds; async exported functions remain excluded.

## Limited declarations and interface versions

The [conformance design](JS_WASM_ADAPTER_CONFORMANCE_V1.md) contains illustrative ESM declarations
and a WIT type fragment. They are not produced artifacts, executable examples or #358 API names.
Generation is limited to verified named scalar functions, the exact list/string carriers above,
nominal resource factories/methods/dispose, and a closed adapter error type. No wildcard export,
ambient global, arbitrary overload, namespace merge, class inheritance or npm type inference is
supported. Emit declarations alongside their exact adapter implementation; check export/name and
signature equality, including resource modes and errors. A TypeScript `number` declaration cannot
express exact i32 admission; runtime range and negative-zero checks remain mandatory.

Compatibility binds the exact tuple of adapter contract, application interface revision and
digest, resource type identities, target/host policy, canonical options and WIT/WASI identities
where applicable. Artifact hashes bind the implementation to that tuple. A missing or mismatched
tuple rejects before loading/calling; no semver-range or structural resource compatibility is
inferred. The future artifact binding belongs in its reviewed format, not an invented field of
current manifest v1/v2/v3 or #168's package schema.

Changing encoding, carrier, nullability, copy/borrow/owner mode, error, limit, name or signature
requires a new incompatible interface revision and regenerated declarations/consumers. An added
export also changes the exact tuple; callers may opt into a reviewed new version, never silently
negotiate it. Documentation-only clarification may retain the revision. WIT/WASI changes obey
#167's separate package/registry migration rule. Source packages that still compile are not
necessarily compatible binaries; binary interface equality does not prove source-language or
dependency compatibility. No package/registry publication or public support promise follows.

## Bounded implementation handoff

The [proof plan and accepted library alignment](JS_WASM_ADAPTER_CONFORMANCE_V1.md) specify fixed
outcomes, limits, cleanup observations and later slices. #167 supplies WIT authority, merged #357
supplies composition authority, and merged #358 supplies the corresponding C/A/H library decisions.
Their per-API alignment is complete at the design boundary. F1 bridge implementation, F2 concrete
host-outcome/resource admission, H4's F3 u64 carrier and asynchronous realizations' F4 revision
remain separate implementation prerequisites; this specification does not activate them.

Keep **specified**, **prototype**, **conformance-passed**, and **publicly-supported** separate.
This change supplies design and documentation/fixture consistency checks only. Later work needs
independent malformed-input, lifetime and host proofs, the repository-required verification, and
explicit activation with authenticated support documentation. Neither this issue nor the small
proof adds an M3 acceptance item, native C contract or a universal ecosystem prerequisite.
