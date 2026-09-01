# M3 Copy aggregate semantics

Status: implemented as an internal compiler boundary for Issue #79. This boundary is not a public
compiler profile and does not activate a runtime, backend, driver route, CLI selector, manifest, or
public aggregate ABI.

## Authority boundary

`zryna-semantics::data_ownership_v1` consumes only a verified protocol-v4 syntax snapshot bound to
the exact final `SourceMap` and an expected entry file present in that snapshot. Provider errors
stop before semantic analysis. Provider syntax kinds, symbols, inferred types, nominal identities,
field or variant ordinals, layouts, and ownership claims never enter this boundary.

Semantics owns the complete meaning of the admitted source graph. It authenticates the module
inventory, resolves aggregate names only to declarations in the same module, assigns nominal
identity as `(ModuleId, source-order declaration index)`, checks exact types and the closed Copy
subset, constructs the source-bound raw layout graph, and independently verifies that same graph
for both `Linear32V1` and `LinuxX8664V1`. It then lowers raw `DataOwnershipV1` and calls the
independent IR verifier with the exact source map, expected entry, and both sealed layout
authorities. Success exposes only the sealed
`zryna_semantics::data_ownership_v1::VerifiedProgram`, which retains the mandatory-verifier-approved
IR together with the exact verified ownership-runtime ABI declaration authority. Raw layout, IR,
and runtime declarations remain private.

The public semantic boundary is deliberately small:

```text
SemanticInput::try_new(&v4::ProjectSyntaxSnapshot, &SourceMap, expected_entry: FileId)
lower(SemanticInput) -> Result<semantics::data_ownership_v1::VerifiedProgram, Vec<Diagnostic>>
```

`SemanticInput` exposes read-only `syntax()`, `sources()`, and `entry()` accessors. Its fields are
private, and construction returns `None` for a foreign source-map identity, a missing entry, or a
snapshot containing a provider error. `MAX_SEMANTIC_DIAGNOSTICS` is 256, including the terminal
budget diagnostic.

The two layout snapshots must describe the same semantic type universe. Semantic lowering resolves
sealed types through their verified nominal identity, category, and children; it never treats a raw
layout node index as a canonical `TypeId`. Source spans are reissued only through the exact
authoritative `SourceMap`.

This is a separate M3 path. It does not change the protocol-v2 M1 `lower` function, the protocol-v3
`control_flow_v1` boundary, their diagnostics, fixtures, IR, artifacts, manifests, or public
commands.

## Admitted Copy subset

The internal Issue #79 gate admits:

- `bool` and wrapping `i32` scalar operations already defined by `ControlFlowV1`;
- nominal structs whose complete stored field graph is Copy;
- nominal enums with zero or one payload per variant when every possible payload is Copy;
- fixed arrays whose element type is Copy;
- exact struct construction and statically resolved field projection;
- exact enum construction, discriminant selection, and exhaustive variant matching;
- fixed-array construction and constant, statically checked element projection; and
- internal functions and locals carrying admitted aggregates while entry-module exports retain
  scalar ABI v1 parameters and results.

Struct fields and enum variants use source declaration order. A struct constructor must initialize
every declared field exactly once. Initializers evaluate left to right in declaration order even
when their source spelling uses a different property order. Field access lowers the sealed field
ordinal; it is never a target-language property lookup.

An enum constructor names one exact variant and supplies exactly its declared payload shape. A
match evaluates its scrutinee once, lists every variant exactly once, and makes a payload binding
available only in the active payload arm. Arm result types must agree exactly. This Issue #79 gate
admits match only as the returned expression of a single-statement internal function, with each arm
producing a scalar literal, parameter, or active payload binding. General nested match expressions,
aggregate-valued arms, and shared continuation blocks remain unavailable.

Fixed-array construction evaluates elements in ascending index order. Issue #79 deliberately
admits only a compile-time constant index satisfying `0 <= index < N`; negative, nonconstant,
and out-of-bounds source indices are semantic errors. This gate therefore emits no dynamic bounds
check or `BoundsV1` trap. The broader checked dynamic-index behavior in the normative M3 profile
remains unavailable until a later gate adds the corresponding executable failure path.

Copy is derived transitively from the sealed semantic and layout type graph, never from source
spelling, structural similarity, host layout, or target size. Reading a Copy aggregate does not
move, clone, drop, allocate, or create a cleanup obligation. Same-shaped nominal types remain
distinct. Copy parameters, locals, and temporaries may still have addressable places for exact
storage and projection identity; the IR verifier excludes those values from its non-Copy owner map
and pending-drop stack.

The gate rejects imported aggregate-name lookup and direct calls. The in-progress Issue #81
checkpoint extends the same private boundary to canonical String and `Vec<T>` type graphs in
parameter-free straight-line functions. String supports UTF-8 literals, explicit clone, checked
concatenation, moves, return cleanup, and mutable root-local replacement. Vec supports
construction, moves, return, push, checked Copy-element indexing, and replacement of supported
exact root locals. Preparation precedes the infallible `ReplacePlace` commit; return transfers the
selected owner first and drops remaining locals in reverse successful-completion order. Cumulative
String-literal bytes are preflighted against the exact 8 MiB IR limit before proportional lowering.

A second bounded Issue #81 route now admits parameter-free private straight-line owned Struct,
FixedArray, and Enum results. Struct and FixedArray graphs may contain bool, i32, String, and
supported nested Struct or FixedArray values. Enum variants are payloadless or carry one supported
Copy, String, Struct, or FixedArray payload; nested enum and Vec payload graphs remain excluded.
The route evaluates constructors in sealed declaration or index order, transfers whole values on
move, returns one exact owner, and drops surviving roots in reverse successful-completion order.
It also admits explicit clone for exact Copy/String Vec and supported String-bearing aggregates,
prepare-before-commit whole-root assignment, and canonical static StructField or
FixedArrayConstant source projections. Mutable available String projections additionally admit
prepare-before-commit assignment whose commit drops only the exact old leaf. Copy projections retain their source; exact String leaves
move once and refine the enclosing root's recursive cleanup mask while disjoint siblings remain
available.
Unresolved binding names report `ZRYNA-M3002`, while unavailable or already moved owned aggregate
and enum bindings report `ZRYNA-M3014`.

This checkpoint does not enable general owned values: aggregate/enum subobject moves,
whole-partial-owner transfer, dynamic or Vec-element projections, projected clone or general
non-String projected assignment,
general owned parameters/calls/CFG, and general lexical scope-drop insertion remain unavailable.
Public owned results remain rejected by scalar ABI v1.
`Shared`, `Weak`, shared or exclusive borrows, and their
operations also remain unavailable. The boundary still rejects recursive by-value layouts,
unresolved names or types, invalid projections, duplicate declarations or members, and
missing, duplicate, extra, or mistyped constructor and match data. Layout recursion diagnostics
remain owned by `zryna-layout`; verified-IR diagnostics remain owned by `zryna-ir` rather than
being reinterpreted as semantic success.

## Internal Pair oracle

`Pair { left: i32, right: i32 }` is the smallest fixed semantic oracle. The internal
`pairScore(left, right)` behavior constructs one Pair, reads both fields, and returns wrapping
`pair.left * 31 + pair.right`. The repository fixes these scalar observations:

| Call | Result |
| --- | ---: |
| `pairScore(0, 0)` | `i32:0` |
| `pairScore(1, 2)` | `i32:33` |
| `pairScore(-1, 2)` | `i32:-29` |
| `pairScore(2147483647, 1)` | `i32:2147483618` |
| `pairScore(-2147483648, -1)` | `i32:2147483647` |

Focused semantic tests obtain these observations with a test-only scalar evaluator over the opaque
verified IR views. That evaluator is not a production interpreter, runtime, backend, driver, or
CLI path. The oracle proves semantic construction, field roles, wrapping scalar behavior, and
verified-IR consumability without emitting or executing a target artifact.

## Diagnostics and resource closure

All source rejections are deterministic, bounded, and located at the authoritative declaration,
use, initializer, arm, or index span. Exact duplicates diagnose the later conflicting occurrence;
unresolved uses diagnose the use; and recursive layouts retain the closing layout edge. Diagnostic
selection is stable across input discovery order, and exhausting the diagnostic budget ends in one
terminal resource diagnostic without exposing a partial verified program.

The semantic diagnostic families are frozen as follows:

| Code | Semantic rejection |
| --- | --- |
| `ZRYNA-M3002` | module-local names, declarations, and exact type resolution |
| `ZRYNA-M3003` | excluded non-Copy, heap, handle, borrow, call, or dynamic operation |
| `ZRYNA-M3004` | independently derived layout-universe inconsistency |
| `ZRYNA-M3005` | struct, enum, or fixed-array constructor shape |
| `ZRYNA-M3006` | field or constant fixed-array projection |
| `ZRYNA-M3007` | initialization, assignment, mutability, or exact-type mismatch |
| `ZRYNA-M3008` | scalar expression, literal, or operator rejection |
| `ZRYNA-M3009` | enum-match scrutinee, arm, payload, exhaustiveness, or result mismatch |
| `ZRYNA-M3010` | return shape or scalar-only public ABI rejection |
| `ZRYNA-M3011` | unavailable, unknown, or already moved private String binding |
| `ZRYNA-M3012` | private String statement, expression, type, or shape outside the straight-line slice |
| `ZRYNA-M3014` | unavailable, duplicate, or already moved owned aggregate or enum owner |
| `ZRYNA-M3016` | owned aggregate graph, constructor, statement, or operation outside the bounded slice |
| `ZRYNA-M3201` | nominal-declaration or derived-value amplification exceeds a verified-IR limit |
| `ZRYNA-M3202` | terminal semantic diagnostic-budget exhaustion |

Protocol-v4, aggregate-layout, and verified-IR failures retain their owning `ZRYNA-Y4xxx`,
`ZRYNA-L3xxx`, and `ZRYNA-I3xxx` codes and locations. Semantic lowering does not relabel them.

The semantic gate preflights nominal-declaration, derived-value amplification, and cumulative
String literal bytes before graph construction, including the exact `DataOwnershipV1` limits and
their first rejected declaration, value, or byte.
Protocol-v4 independently
bounds source collections before semantics; aggregate layout and `DataOwnershipV1` independently
bound and verify their derived graphs. Diagnostic retention is separately capped at 256. No
semantic success can bypass those independent layout or IR resource gates.

## Deliberately unavailable

The `OwnershipRuntimeV1` value in verified IR remains a non-executable contract identity. The
semantic result now retains the separately verified Issue #80 declaration authority beside that
IR, binding exact declarations, both authenticated layouts, header evidence, and pure transitions.
It still implements no allocator or runtime. No runtime import, heap helper body, target artifact,
host observation, or profile manifest is constructed here.

The JavaScript, WebAssembly, native, driver, and CLI components do not depend on or select this
boundary. M1 and explicit M2 remain the only public compiler profiles. Public aggregate parameters
or results, `--profile data-ownership-v1`, manifest v3, and executable Pair support remain gated on
the later runtime, backend, integration, conformance, and publication issues.

Normative profile behavior remains defined by
[`DATA_OWNERSHIP_V1.md`](../spec/language/DATA_OWNERSHIP_V1.md). The provider-neutral input,
layout authority, and verified output contracts are documented in
[`SYNTAX_PROTOCOL_V4.md`](SYNTAX_PROTOCOL_V4.md),
[`AGGREGATE_LAYOUT_V1.md`](../spec/memory-model/AGGREGATE_LAYOUT_V1.md), and
[`M3_DATA_OWNERSHIP_IR.md`](M3_DATA_OWNERSHIP_IR.md).
