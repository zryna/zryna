# Zryna semantics

Permanent Zryna-owned phase boundary from verified provider-neutral syntax to Universal IR. The
legacy M1 entry returns raw IR to its verifier call site; the isolated M2 entry seals IR internally.

`SemanticInput::try_new` accepts only a verified protocol-v2 snapshot bound to the exact immutable
`SourceMap` that issued it. It rejects every snapshot containing a provider error, so parse
recovery or unsupported syntax cannot enter name resolution, type checking, or lowering as a
smaller program. Provider warnings remain non-fatal and are preserved by `zryna-driver`.

## First strict source subset (protocol v2/M1)

`lower` currently requires exactly one source file containing at least one explicitly exported
function. It accepts:

- portable scalar-ABI export names that are unique exactly and under ASCII case folding;
- explicitly annotated `i32` and `bool` parameters and results;
- unique parameter bindings and references to those parameters;
- exactly one value-returning statement per function;
- `i32` literals in the inclusive range `-2147483648` through `2147483647`;
- `bool` literals; and
- left-associative `+` expressions whose operands are both `i32`.

Missing annotations, `any`, unknown types, unresolved names, duplicate parameters or exports,
invalid or colliding export names, out-of-range literals, non-`i32` addition, mismatched return
types, empty entrypoints, multiple files, and bodies without exactly one return fail with bounded,
deterministically ordered `ZRYNA-M1xxx` diagnostics. Semantic input sizes are compile-time proven
not to exceed the corresponding Universal IR limits.

The protocol-v2 bootstrap path rejects parenthesized expressions, calls, local declarations,
control flow, and heap-backed expressions before M1 semantics instead of normalizing a smaller
program. Multi-file and module semantics remain outside this legacy `lower` gate, which rejects
multiple files. Protocol v3 and the separate M2 entry below do not inherit those M1 restrictions.

M1 semantic success returns raw `Program`; only `zryna-ir::verify` can turn it into backend-safe
`VerifiedProgram`. `bool` is valid source semantics and lowers to `BoolLiteral`, but the current
`I32V1` verifier deliberately rejects it with `ZRYNA-I1006`. Therefore the current complete
source-to-verified-IR success path is the documented `i32` subset. Enabling `bool` requires one
future universal profile implemented consistently by every active backend.

This crate owns language meaning and must never depend on a replaceable frontend provider.

## Internal M2 semantics boundary

The separate `control_flow_v1` module consumes only an exact source-map-bound verified
protocol-v3 snapshot and an entry present in that snapshot,
revalidates the complete deterministic module graph, owns module/callable/lexical names and exact
types, and lowers the frozen M2 semantic subset. It implements `i32` arithmetic and signed
comparisons, Boolean equality, initialized locals, assignment, lexical shadowing, and statically
resolved acyclic direct calls while preserving left-to-right once-only evaluation.

M2 semantic success returns only mandatory-verifier-approved
`zryna_ir::control_flow_v1::VerifiedProgram`; raw M2 IR never leaves the boundary. Canonical `if`
and `while` lowering carries definite mutable state through typed merge and loop-header parameters,
with reachability and all-path return checks. No backend or public CLI selects this profile. See
[M2 straight-line semantics](../../docs/M2_STRAIGHT_LINE_SEMANTICS.md) and
[M2 control-flow semantics](../../docs/M2_CONTROL_FLOW_SEMANTICS.md).

## Internal M3 aggregate and owned-data foundation boundary

The separate `data_ownership_v1` module consumes an exact source-map-bound verified protocol-v4
snapshot and entry file. It owns nominal identities, names, exact types, field and variant
ordinals, and recursively Copy classification. It verifies the same semantic type graph through
both `Linear32V1` and `LinuxX8664V1` layout authorities and lowers Copy structs, enums, and fixed
arrays. The sealed semantic `VerifiedProgram` retains both mandatory-verifier-approved
`zryna_ir::data_ownership_v1::VerifiedProgram` and the exact verified ownership-runtime ABI
declaration authority; neither raw IR nor raw runtime declarations can be recovered.

Fixed-array access in this internal gate is limited to a compile-time in-range constant. The
in-progress Issue #81 checkpoint recognizes canonical String and `Vec<T>` type graphs and lowers
bounded private functions. String supports UTF-8 literals through
`StringFromUtf8`, explicit clone, checked concatenation, local moves, return with reverse-order
cleanup, and mutable root-local replacement. Vec supports construction, explicit clone for exact
`Vec<bool>`, `Vec<i32>`, and `Vec<String>`, local moves, return, push, checked indexing that returns
a Copy element, and replacement for the supported exact Vec roots.
Both routes admit private zero-argument producers and one-argument owned identity calls. They also
lower one top-level no-phi `if`/`else` from a bool literal or Copy bool parameter into canonical
entry/then/else/join blocks. Branch-local owned roots drop once in reverse order; incoming owners
must be restored exactly, and mutation of an incoming Vec is rejected before lowering its value.
Private String and exact Vec result functions additionally admit one terminal `if`/`else` whose
arms each return one owned-producing expression through a canonical one-parameter join. The join
owns the selected value exactly once and return cleanup excludes that carried value.
Both routes also admit one top-level no-carried-owner `while` after supported declarations and
before the sole final return. Its condition is emitted in the canonical header, iteration-local
owners drop in reverse before the backedge, and both backedge and false exit restore the exact
incoming owner and String-byte state. The stable-place subset replaces one mutable outer String
after complete RHS preparation or pushes one Copy element into a mutable outer exact Vec; both
retain the outer place identity and use no owned loop-header phi. Vec loop replacement and
owned-element Vec loop push remain excluded.
Vec construction, push, and direct calls reserve their parent cleanup and result resources before
fallible child evaluation, using expression-aware net owner growth at exact limits.
The lowerer uses `InitializePlace`, `MoveFromPlace`, and the infallible `ReplacePlace` commit after
right-hand-side preparation. A separate bounded route constructs, moves, explicitly clones,
returns, and drops
acyclic owned Struct and FixedArray graphs with bool, i32, or String leaves, and owned Enums with a
payloadless variant or one supported Copy, String, Struct, or FixedArray payload. Structural clone
retains its source, creates a distinct result owner, derives the exact fallible String-leaf count
and root-enum active variant from sealed authorities, and reverse-drops only the initialized result
prefix on element failure. Mutable whole-root assignment for the same aggregate graphs prepares a
distinct right-hand side before the infallible `ReplacePlace` commit, rejects direct
self-consumption, and exposes the old destination's recursive drop shape. Constructor
operands are evaluated in sealed declaration or index order, whole-value moves transfer one owner,
and return cleanup drops surviving roots in reverse successful-completion order. Canonical static
struct-field and constant fixed-array projections additionally admit Copy reads, exact String-leaf
moves, and at most one exact supported Struct/FixedArray subobject move into a directly initialized
same-type local, at most one explicit clone of an initialized available non-Copy
Struct/FixedArray projection into the immediately following exact same-type local, plus
prepare-before-commit assignment to one mutable available String leaf. Projected aggregate clone
retains the enclosing root and its partial-state mask, uses the same layout-derived recursive
String-leaf failure cleanup as whole-root aggregate clone, and creates a distinct temporary owner.
The
subobject route materializes its complete static descendant topology before the move; the enclosing
root keeps one masked cleanup obligation while the new local owns the moved subtree.
One additional canonical private route extracts the complete non-Copy Struct or FixedArray payload
of a single-variant enum through an exhaustive one-arm `match`, initializes one exact direct local,
drops the now-empty enum root, and returns that local through a zero-argument continuation. The
payload source topology is complete, while the whole destination owner needs no duplicated
descendant places.
Assignment preparation retains the enclosing root and commit drops only the exact old leaf while
leaving sibling masks unchanged. A moved leaf is excluded from the enclosing root's recursive return cleanup;
disjoint leaves stay available, while a repeated or overlapping move and later whole-root
consumption are rejected. The private
String route reports use-after-move as `ZRYNA-M3011`; the bounded owned aggregate and enum route
reports it as `ZRYNA-M3014`; unresolved binding names report `ZRYNA-M3002`. Excluded private String shapes report
`ZRYNA-M3012`, and cumulative String-literal bytes are checked against the exact 8 MiB limit before
lowering.

An internal test-only fault/drop-trace oracle covers every ABI-admitted failure of the implemented
String construction/clone/concat and Vec allocation/reserve operations, plus the separate verified
Vec bounds trap. It consumes authenticated status disposition/trap declarations, retains all
pre-commit operand owners, excludes the uncommitted result, and checks exact reverse cleanup,
deterministic replay, and bounded event accounting. It is not allocator execution, target runtime
fault injection, or a public interpreter. Exact `Vec<String>` clone additionally seals a distinct
element-failure cleanup that reverse-drops only the runtime-recorded initialized destination prefix
before pre-existing owners; this remains compiler evidence rather than allocator execution.
Supported aggregate clone uses its own `AggregateCloneElementFailure` role and typed initialized-
prefix action under the same rule; neither its fallible-leaf count nor root-enum active variant is
accepted from the fault injector.

This checkpoint does not complete general owned lowering. General structural Vec clone beyond
String elements, nested aggregate clone graphs containing Enum, Vec, Shared, or Weak values,
aggregate-subobject moves outside one exact direct local or the one single-variant match-local enum
payload extraction, dynamic or Vec-element projections, projected aggregate clone outside its one
exact direct-local form, projected aggregate assignment outside one whole-root-to-static-projection
site, whole-partial-owner transfer outside the exact-type direct-local,
final-return, or whole-root assignment Struct/FixedArray exceptions, general owned phi joins,
owned loop-carried phi joins,
repeated/nested branches or loops, and general scope-drop insertion remain unavailable. `break`,
`continue`, loop-body return, and post-loop effects are also excluded. Owned String/Vec signatures remain limited to zero or one
exact owned/bool argument. The owned aggregate route is parameter-free, private, and straight-line;
its projection subset is limited to static Struct/FixedArray Copy reads, String-leaf moves, one
direct-local supported Struct/FixedArray subobject move, String-leaf clone, one direct-local
supported Struct/FixedArray clone, String-leaf assignment, and at most one private straight-line
assignment that moves a distinct fully initialized exact same-type supported non-Copy Struct or
FixedArray whole root into a mutable available `StructField`/`FixedArrayConstant` projection. That
commit recursively drops the exact old target, consumes the source, and retains the destination root
and sibling masks. A partially moved Struct or FixedArray root may move
through one exact direct-reference initializer into a same-type local, through one final exact
direct-reference return, or into one distinct mutable fully initialized same-type whole-root
assignment destination. Assignment prepares the source without touching the destination, then an
infallible `ReplacePlace` drops the old destination and installs the exact partial mask. Lowering
preserves the complete recursive static topology, migrates the moved mask through every
temporary/local owner, excludes a returned owner from survivor cleanup, and invalidates old
owners. Enum-payload moves outside that exact one-arm direct-local form, dynamic or Vec-element
moves, broader/projected-source/multi-site aggregate assignment, calls, direct projected-clone
returns, CFG transfer, or public functions remain outside the narrow subobject route. Admitted projected clones retain the
enclosing root and its partial-state masks while creating one distinct temporary owner.
Dynamic bounds execution, borrows, shared or weak references,
and public owned parameters or results also remain unavailable. The Pair scalar oracle
is observed only by a test evaluator over opaque verified views. Enum matching is limited to an
internal single-return function with scalar literal, parameter, or active-payload arms; this is not
a general expression-level match implementation. No runtime, backend, driver, CLI, or public
`data-ownership-v1` profile selects this module. See
[M3 Copy aggregate semantics](../../docs/M3_COPY_AGGREGATE_SEMANTICS.md).
