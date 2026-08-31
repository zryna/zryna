# Scalar control flow and modules v1

Status: specified for M2, not implemented. This document does not enable a compiler profile,
command, artifact, or public support claim.

## 1. Profile identity and compatibility

The exact universal profile name is `ControlFlowV1`; its CLI spelling is `control-flow-v1` and its
future manifest profile is `zryna-control-flow-v1`. It is a separate verified profile, not an
extension that weakens `I32V1` verification.

Protocol v3 will be the first provider-neutral syntax protocol capable of describing this profile.
Protocol v2, scalar ABI v1, the M1 CLI default, and `zryna-manifest-v1.json` remain immutable M1
contracts. A future M2 public command must require `--profile control-flow-v1` and emit a distinct
canonical `zryna-manifest-v2.json`. Omitting `--profile` continues to select the implemented M1
behavior until a separate compatibility proposal changes that default.

Only the entry module's exported functions enter scalar ABI v1 and become public target exports.
Dependency-module exports and unexported functions are internal. Scalar ABI v1 therefore remains
the external `bool`/`i32` carrier and observation contract; M2 adds sealed module and function
identities below that public boundary.

## 2. Types and values

`ControlFlowV1` admits exactly `i32` and `bool` parameters, results, locals, block parameters, and
operation results. `unit` remains reserved. There are no implicit conversions.

- `i32` is a signed two's-complement 32-bit integer. Its mathematical residue is interpreted
  modulo `2^32` for wrapping operations and as the signed range `-2147483648..=2147483647` for
  ordering.
- `bool` has exactly the language values `false` and `true`. JavaScript carries them as primitive
  Boolean values. Core WebAssembly and Linux x86-64 carry them as `i32` lanes `0` and `1` only.
  Any other boundary lane fails; it is never truthy.

All expressions evaluate left to right, exactly once. A trap or divergence stops evaluation and
prevents every later operand, initializer, or argument from being evaluated.

## 3. Exact operators

The following table is exhaustive. Operands must have the listed exact type.

| Source | IR operation | Operands | Result | Meaning |
| --- | --- | --- | --- | --- |
| `a + b` | `I32Add` | `i32`, `i32` | `i32` | signed two's-complement addition modulo `2^32` |
| `a - b` | `I32Sub` | `i32`, `i32` | `i32` | signed two's-complement subtraction modulo `2^32` |
| `a * b` | `I32Mul` | `i32`, `i32` | `i32` | low 32 bits of the mathematical product |
| `-a` | `I32Neg` | `i32` | `i32` | `0 - a` modulo `2^32` |
| `a === b` | `Eq` | equal `i32` or equal `bool` | `bool` | exact value equality |
| `a !== b` | `Ne` | equal `i32` or equal `bool` | `bool` | exact value inequality |
| `a < b` | `I32LtS` | `i32`, `i32` | `bool` | signed less-than |
| `a <= b` | `I32LeS` | `i32`, `i32` | `bool` | signed less-than-or-equal |
| `a > b` | `I32GtS` | `i32`, `i32` | `bool` | signed greater-than |
| `a >= b` | `I32GeS` | `i32`, `i32` | `bool` | signed greater-than-or-equal |

This profile has no language-level arithmetic trap. Division and remainder are deliberately
unavailable until a later profile freezes a versioned outcome ABI that can distinguish their edge
cases identically across JavaScript, core WebAssembly, and native execution. Backend engine
messages, signals, exception objects, and process statuses never become language results by
accident.

Division, remainder, bitwise operations, shifts, `&&`, `||`, `!`, loose equality, floating-point
operators, and mixed-type equality are unavailable.

## 4. Bindings, assignment, and scope

The supported local declarations are:

```ts
const name: i32 = expression;
let other: bool = expression;
```

Both an explicit `i32` or `bool` annotation and an initializer are mandatory. A declaration's name
enters scope only after its initializer succeeds. `const` cannot be assigned. `let` may be assigned
by the statement `name = expression;`; assignment is not an expression and its value must have the
declared exact type.

Parameters and locals share one lexical value namespace. A parameter may not be redeclared in the
function body's outer block, and a name may not be declared twice in one block. A nested block may
shadow a parameter or an already declared outer local; resolution selects the nearest enclosing
declaration and the outer binding becomes visible again after the nested block. Imports and
top-level functions share a separate module callable namespace. Local and parameter names never
shadow callable names because call callee resolution uses only that callable namespace, while every
other identifier expression uses the lexical value namespace. A declaration may not refer to
itself before it enters scope. Every local is initialized by construction, and every control-flow
merge must prove one exact typed current value for each live mutable binding.

A statement after a statement that returns on every path in the same block is unreachable and is
rejected. There are no hoisted local declarations, implicit declarations, destructuring, compound
assignments, increment/decrement, or bare expression statements.

## 5. Functions and direct calls

Functions are top-level module declarations with explicitly typed parameters and result. A
function may be unexported or use an explicit named export. Overloads are unavailable, and each
source-level function name is unique within its module.

A call expression has one identifier callee and a source-ordered argument list. The identifier
must resolve statically to a same-module function or one named import. Arity and every argument
type must match exactly. Arguments evaluate left to right exactly once. Forward references are
allowed because module declarations are collected before bodies are checked.

The complete resolved function call graph must be acyclic. Direct recursion, mutual recursion, and
cycles crossing modules are rejected before Universal IR construction. Function values, methods,
closures, callbacks, indirect calls, generics, async functions, exceptions, and runtime dispatch
are unavailable.

## 6. Statements and structured control flow

The supported executable statements are local declaration, assignment, `return`, block,
`if`/`else`, and `while`.

```ts
if (condition) {
  // statements
} else {
  // statements
}

while (condition) {
  // statements
}
```

Each condition evaluates exactly once at the branch point and must have exact type `bool`.
`while` reevaluates its condition before every iteration, including the first. There is no
truthiness or numeric condition conversion.

An omitted `else` is exactly an empty false-path block. It preserves every incoming mutable value
on the false edge and then joins the true edge under the same definite-state rules as an explicit
empty `else {}`.

Every reachable function path must return one value of the declared exact result type. A
`while`, including `while (true)`, is treated as potentially falling through for return analysis;
the checker does not use constant-condition termination proofs. A loop may diverge at runtime.
The existing bounded CLI host deadline contains an invocation but does not redefine divergence as
a language trap.

`break`, `continue`, labeled statements, `switch`, conditional expressions, short-circuit
operators, `try`, `throw`, and `finally` are unavailable.

## 7. Modules and public exports

One requested portable workspace-relative `.zry` file is the entry module. Imports use only this
form, at module top level before functions:

```ts
import { exportedName, otherName as localName } from "./relative/path.zry";
```

The import list is nonempty and named. A specifier must be a UTF-8 string beginning with `./` or
`../`, use `/`, end in the explicit lowercase extension `.zry`, and contain no absolute root,
backslash, query, fragment, URL scheme, NUL, empty component, or host-specific prefix. Resolution
joins the importer directory and specifier, removes `.` components, resolves `..`, and then applies
the existing portable `NormalizedSourcePath` grammar. Escaping the validated workspace root fails.

There is no bare/package import, default import/export, namespace import, re-export, wildcard,
dynamic import, alias configuration, implicit extension, implicit `index.zry`, URL import, or
`node_modules` lookup.

Every imported name must be explicitly exported by the resolved dependency module. Imported local
names participate in the module's top-level namespace and cannot collide with another import or
function. The entry module's exported functions are the only public scalar ABI exports. An export
in a dependency module is visible to named imports but remains target-internal for that build.

Module identities are dense IDs assigned by normalized portable path byte order after graph
closure. Function identities are `(module-id, declaration-index)` in source declaration order.
Target backends derive private names only from sealed verified identities; source paths never
become unsanitized target symbols.

The import graph and resolved call graph must both be acyclic. Duplicate edges are rejected rather
than silently collapsed. Diagnostics are ordered by normalized portable path bytes, primary start
offset, stable code, and complete deterministic tie-break data.

## 8. Compiler-owned fixed-point discovery

The TypeScript provider never reads imported files and continues to advertise
`module_resolution: false`. Discovery is driver-owned:

1. open and retain a capability for the validated workspace root, then safely read the entrypoint
   into a bounded discovery batch;
2. authenticate protocol v3 analysis for exactly that batch and retain only verified import DTOs
   bound to each file's normalized path and source hash;
3. inspect those DTOs, resolve their specifiers with the rules above, and determine unresolved
   normalized paths;
4. safely read unresolved files in normalized path order and analyze only that new immutable batch;
5. repeat steps 3 and 4 until no unresolved import remains, then build the final immutable
   `SourceMap` once and request exactly one final full-map protocol snapshot;
6. verify graph closure and run Zryna semantics only from that final snapshot.

An intermediate batch response or `FileId` never enters semantics. A provider must return every
requested batch file exactly once and no extra file. Verified import DTOs for unchanged normalized
path and source-hash pairs must be byte-identical across any repeated request. Any provider error
ends discovery immediately, before another path is resolved or file is read. The driver, not the
provider, owns path resolution, file access, graph completion, cycle rejection, and source hashes.

Every dependency open is a component-by-component, handle-relative traversal from the retained
workspace-root capability. Each component is opened without following a symbolic link or Windows
reparse point; opened directory handles remain the authority for the next component. The final
regular-file handle is bounded, read through that handle, and revalidated for identity and state
after the read. An implementation may use an equivalent primitive only when it proves containment
from retained root through every retained ancestor and the final file under concurrent replacement.
Lexical normalization or reopening the same escaped identity is not a containment proof. Parent
directory swaps, junction/reparse replacement, final-file replacement, and exact-limit plus-one
reads are mandatory adversarial cases for Issue #47.

The graph identity is SHA-256 over this canonical byte document: ASCII
`ZRYNA-M2-GRAPH\0`, little-endian `u32` version `1`, then a length-prefixed entrypoint, file table,
and edge table. Every count and UTF-8 byte-string length is an unsigned little-endian `u32`. Files
are sorted by normalized portable path bytes and encode path followed by the raw 32-byte source
SHA-256. Edges are sorted by importer path, specifier bytes, imported name, and local alias; each
encodes those four length-prefixed UTF-8 fields. The document contains no host absolute path,
filesystem enumeration order, JSON, locale-dependent text, or hexadecimal digest spelling.

## 9. `ControlFlowV1` Universal IR

The M2 program is separate from the raw M1 expression-tree program. It contains sealed modules and
functions. Each function contains dense blocks and function-local dense values.

- Block `0` is the entry block. Its first values are the function parameters in signature order.
- Every other block declares zero or more typed block parameters.
- Instructions define one typed value and are ordered within their block. Constants, exact
  operators, and direct calls are the only instruction kinds.
- A terminator is exactly one of `Return(value)`, `Jump(target, arguments)`, or
  `Branch(condition, true-target, true-arguments, false-target, false-arguments)`.
- Mutable source locals do not survive as target-dependent storage claims. Semantic lowering
  tracks the current SSA-like value and passes live values explicitly as block arguments.

Function parameters, block parameters, and instruction results allocate one shared dense
function-local `ValueId` sequence in that order: function parameters first, then blocks in dense
`BlockId` order, with each block's parameters before its instruction results. Every ID in
`0..value-count` has exactly one definition; duplicate definitions and holes are invalid.

The mandatory verifier proves all of the following before constructing a verified M2 program:

- every module, function, block, value, target, argument, and operand ID is in range and belongs to
  the exact containing authority;
- signatures, operation operands/results, returns, calls, branch conditions, and block arguments
  have exact types;
- every block has one terminator, entry has no predecessor, every nonentry block has a predecessor,
  and every block is reachable from entry;
- every dense value ID has exactly one definition and the definition kind and owner match the
  canonical allocation order; there is no missing or duplicate definition;
- definitions dominate every use; instruction operands are earlier definitions in the same block
  or dominating values; block parameters are defined on entry to their block;
- predecessor/successor edges and block-argument arity/types agree exactly;
- a backedge may target only a nonentry dominating loop header, and every cycle has one such header;
  irreducible control flow and every edge to entry are rejected; each `Jump` counts as one CFG edge
  and each `Branch` arm counts as one edge even when both arms name the same target;
- every reachable exit returns the declared value type and the complete call graph is acyclic;
- all spans resolve through the exact final `SourceMap`; and
- all deterministic resource budgets below hold.

Backends see only opaque verified program, module, function, block, instruction, and terminator
views plus sealed public ABI and internal identity mappings. They cannot recover raw IR.

## 10. Resource budgets

| Budget | Limit |
| --- | ---: |
| Source files / modules | 4,096 |
| Aggregate source bytes | 8 MiB |
| Import-discovery rounds | 4,096 |
| Provider analysis calls including final full-map call | 4,097 |
| Cumulative provider input source bytes | 16 MiB |
| Serialized protocol request bytes | 72 MiB |
| Serialized protocol response bytes | 64 MiB |
| Import edges | 65,536 |
| Import declarations per module | 4,096 |
| Import declarations per program | 65,536 |
| Imported names per declaration | 256 |
| Imported names per program | 65,536 |
| Functions per module | 4,096 |
| Functions per program | 16,384 |
| Parameters per function | 256 |
| Parameters per program | 262,144 |
| Lexical blocks per function | 4,096 |
| Lexical blocks per program | 65,536 |
| Statements per function | 4,096 |
| Statements per program | 65,536 |
| Expressions per function | 16,384 |
| Expressions per program | 262,144 |
| Local declarations per function | 4,096 |
| Local declarations per program | 65,536 |
| Live mutable bindings at one merge or loop header | 256 |
| IR blocks per function | 4,096 |
| IR blocks per program | 65,536 |
| Block parameters per block | 256 |
| IR values per function | 16,384 |
| IR values per program | 262,144 |
| CFG edges per function | 8,192 |
| CFG edges per program | 131,072 |
| Call edges | 65,536 |
| Static call depth | 128 |
| Syntax nesting / verified loop nesting | 128 |
| Module specifier bytes | 1,024 |
| Retained diagnostics including terminal budget diagnostic | 256 |

All counts use checked arithmetic. A batch plus final-full-map discovery processes at most twice
the aggregate source-byte limit, so a one-import-per-file chain cannot cause quadratic provider
work. Protocol v3 must freeze request framing whose worst-case encoding of every admitted 8 MiB
source map plus bounded paths and metadata remains below the 72 MiB request limit; it may not assume
unescaped source bytes when proving that bound. The first item beyond any limit emits the phase's
stable terminal budget diagnostic and prevents later phases from acting on an incomplete graph.
Exact-limit and first-extra fixtures are
required for every row; Issue #46 owns protocol/syntax rows, #47 owns discovery/module rows, #48
owns IR rows, and #49/#50 own semantic/control-flow rows.

## 11. Stable diagnostic families

Protocol v3 retains `ZRYNA-F1xxx` for transport/provider failures and `ZRYNA-F2xxx` for reported
unsupported syntax. Driver-owned module discovery uses `ZRYNA-D3xxx`; M2 semantic checking uses
`ZRYNA-M2xxx`; `ControlFlowV1` verification uses `ZRYNA-I2xxx`; native MIR and backend phases keep
their existing component prefixes with separately documented M2 subranges. `x2xx1` is reserved in
each new M2 family for deterministic resource exhaustion.

Exact individual codes are frozen by the issue that implements each boundary before its first
executable use. A provider diagnostic never substitutes for Zryna module, name, type, scope,
return, call-graph, or IR verification.

## 12. Excluded runtime surface

`ControlFlowV1` has no heap, GC, allocator, memory object, string, vector, struct, enum, array,
reference, pointer, I/O, environment, clock, randomness, filesystem, network, thread, atomic,
WASI, Component Model, FFI, or ambient runtime capability. Those surfaces require later versioned
profiles. The absence of GC is a consequence of the profile containing only scalar values; it is
not yet the M3 ownership and deterministic-drop contract.

## 13. Delivery gates

The digest-pinned registry in `tests/m2-contract-v1.json` is a reviewed planning inventory, not yet
an executable conformance corpus or an independent authentication root. Its bytes remain immutable
historical governance evidence after Issue #45 closes. Its cases identify required coverage and
owner issues; Issue #56 must add a separately versioned executable registry that covers every
planned item with source fixtures, exact public entrypoints, target order, graph identities,
observations, diagnostics, and boundary matrices.

Issues #46 through #57 implement this specification in dependency order. No intermediate issue may
claim public M2 support. The first public `control-flow-v1` command is gated on JavaScript, direct
core WebAssembly, verified native MIR, audited Linux native execution, and one atomic multi-file
driver path. Executable fixed-oracle cross-target conformance closes executable M2; authenticated
compiler documentation, website CI, deployment, and live inspection close the milestone.
