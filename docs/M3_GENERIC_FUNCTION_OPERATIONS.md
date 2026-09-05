# Private generic non-handle function operations

Issue #278 extends the existing private straight-line owned-data producer using the shared
[composition contracts](M3_OWNERSHIP_COMPOSITION.md), especially C1–C5. This is source-to-verified-IR
operation support. It supplies no public signature, runtime implementation, target execution,
backend, driver route or public-profile activation. Structured ownership control flow remains the
separate C6/C7 integration obligation; this document does not claim arbitrary branches, loops,
scope exits or owner-carrying joins.

## Exact signatures and routing

The generic route accepts exact by-value parameters and results from the supported non-handle
graph: bool/i32, String, Struct, Enum, FixedArray and positive-stride Vec compositions. Permitted
Vec-indirected recursive graphs use sealed layout identities, not an expanded recursive syntax
tree. Shared/Weak-containing graphs and borrow parameters do not enter this route. A function
with an owned input may return a Copy result; unused owned inputs still require cleanup.

Selection examines authenticated signature/body shape without evaluating source or issuing a
diagnostic. The selected producer subsequently validates every operation. Existing scalar,
String, exact-Vec and bounded aggregate routes retain their established selection where no new
generic shape is required. This is not a blanket reinterpretation of earlier bounded CFG routes.

Each parameter gets its exact declared value identity and addressable parameter place. Non-Copy
parameters enter pending ownership in declaration order; Copy parameters are not pending owners.
Names retain portable case-collision and exact lookup checks. Owned parameters are immutable
bindings: mutation requires an appropriate mutable local rather than silently making the
parameter mutable. Return transfers its exact completed owned result, when any, and reverse-drops
the remaining initialized owners with their current masks and enum refinements.

## Calls and transfer boundaries

A generic call resolves one exact private same-module catalog signature. It rejects borrow
parameters, unsupported graph categories, mismatched result type and wrong arity. Its ordered
argument plan uses each declared parameter type, not the result type as a shorthand for all
parameters. String or `Vec<String>` results therefore still use generic call preparation when the
parameter list is not the legacy zero/one same-type signature.

Arguments evaluate left to right exactly once. Copy arguments remain Copy values; owned arguments
must produce distinct complete available owners through the existing move, constructor, clone or
call operations. Exact contextual type checks and ownership checks occur during preparation.
Neither an implicit clone nor a fabricated projected owner repairs a mismatch or repeated use.
Nested calls finish their own argument preparation and transfer before supplying a result to the
enclosing expression.

While an argument is being prepared, the caller still owns every completed argument temporary.
Its failure cleanup includes those temporaries and other pending roots in reverse completion
order. Only after all arguments complete does the call transfer its non-Copy inputs in signature
order. Copy arguments have no ownership-transfer entry. The call's `CallTrap` cleanup excludes
the transferred inputs and includes the caller's remaining owners. The callee is responsible for
its inputs if it traps; successful return supplies one exact result, owned only when non-Copy.
The mandatory IR verifier remains authority for call identity, argument types, owner exclusion,
cleanup, acyclicity and static call-depth limits.

Consumption rechecks the immutable catalog signature, ordered prepared result identities and
actual emitted argument types. It replays exactly the recorded owner transfers and verifies the
final ownership and preparation facts. A nested returned constructor consumes the call result
as its ordinary exact child; it does not create a second ownership or cleanup convention.

## Shared preparation and static operations

The generic route uses the existing typed decisions and shared preparation summary for nested
non-handle construction, movement, explicit clone, calls and replacement. Semantic validation
and resource replay complete before actual source-state mutation. Held ancestor credits and
future commit transitions remain part of the same plan; allocation of a result identity is not
permission to consume an input early. Rejected preparation leaves the original bindings,
pending owners, projection masks, enum facts, cleanup plans and counters unchanged.

The [static subobject adapters](M3_GENERIC_STATIC_PLACES.md) retain exact Struct-field and constant
FixedArray identity without weakening legacy one-site transfer rules. A Copy
read does not consume that subobject. Moving a complete non-Copy subobject records its moved mask
while leaving the enclosing root responsible for its remaining contents. Static replacement
requires an exact complete mutable target both before and after RHS preparation, commits the
prepared value and drops only the old target subtree. Unavailable or overlapping subobjects are
not repaired by a whole-root move or implicit initialization.

Explicit structural clone reuses the [canonical clone core](M3_GENERIC_CLONE_CORE.md), including
its retained source and recursive destination frontier. Ordinary Vec operations reuse the
[Vec operation contract](M3_GENERIC_VEC_OPERATIONS.md): Copy indexing, explicit owned clone,
checked replacement and the existing Vec push authority. Indexed replacement checks bounds
before RHS preparation and commits only the completed exact element. Internal transient indexed
authority is not a claim that explicit source `Borrow`/`BorrowMut` syntax is implemented here.

The current static-address adapter resolves named owners and supported static field/array paths.
It does not manufacture a place for a fresh returned container or a dynamically selected nested
container. `produceVec()[i]` and `outer[i][j]` require a sound additional operand adapter; cloning
an intermediate container is not equivalent because it adds allocation and effects. This source
composition boundary must remain explicit when assessing the enclosing issue's acceptance.

## Evidence and remaining scope

`generic_call_source` authenticates the source fixture before lowering and checks mixed owned/Copy
argument order, multiple owned transfers, retained caller trap cleanup, callee input cleanup,
String/exact-Vec generic fallback, nested returned constructors and deterministic invalid-input
rejection. `aggregate_contracts` additionally checks recursive owned parameters with a scalar
result and complete reverse cleanup. Ordinary Vec and shared-summary resource tests provide
separate operation and exact/first-extra state-retention evidence.

These are compiler proofs, not allocator fault-execution receipts. The complete #278 assessment
must also account for its operation/extraction and reusable-composition obligations; passing the
call matrix alone does not close that issue. Handle integration, broader CFG composition and
later runtime consumers retain their existing ownership in the composition plan.
