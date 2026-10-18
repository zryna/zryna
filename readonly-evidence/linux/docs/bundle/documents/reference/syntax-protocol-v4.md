# Syntax protocol v4

Status: frozen syntax-only contract for the future `DataOwnershipV1` profile. Protocol v4 does not
activate that profile and does not authorize name resolution, type checking, ownership checking,
layout, IR, runtime, ABI, or code generation.

## 1. Compatibility and authority

Protocol v4 is a new provider-neutral wire contract. It extends the syntax inventory of protocol v3
without changing protocol v2 or v3 bytes, handshakes, DTOs, fixtures, or behavior. A v4 provider
must advertise exact protocol version `4`; a v2 or v3 provider is not a v4 provider.

The provider reports only source-faithful syntax and UTF-8 byte spans. Compiler verification against
the exact final `SourceMap` remains authoritative. In particular, the provider never:

- resolves an identifier, import, nominal declaration, field, variant, call, or type;
- assigns `ModuleId`, declaration index, field ordinal, variant discriminant, or canonical `TypeId`;
- decides whether a use copies, moves, clones, borrows, drops, or aliases a value;
- computes a layout, ownership state, control-flow join, runtime operation, ABI, or IR node; or
- activates `data-ownership-v1` or any backend capability.

Any malformed, unknown, duplicate, unsupported, foreign, noncanonical, or over-budget response
fails as a whole. No partial verified v4 snapshot is exposed.

## 2. Frozen TypeScript-friendly source grammar

Data declarations use these exact TypeScript 6 shapes:

```ts
export interface Pair extends ZrynaStruct {
  left: i32;
  right: i32;
}

export interface MaybeI32 extends ZrynaEnum {
  none: ZrynaNone;
  some: i32;
}
```

`export` is optional. A declaration has exactly one `extends` marker. Struct fields are required
property signatures. Enum variants are required property signatures; exact `ZrynaNone` means no
payload and every other admitted type syntax means one payload. Heritage lists with another marker,
multiple markers, type parameters, declaration merging, methods, call/construct signatures,
optional/readonly properties, computed names, index signatures, initializers, accessors, classes,
TypeScript `enum`, aliases, unions, intersections, and structural object types are unsupported.

The admitted type spellings are:

```text
Name
String
Vec<T>
Shared<T>
Weak<T>
Borrow<T>
BorrowMut<T>
FixedArray<T, N>
```

`Name` and every other v4 identifier use ASCII `[A-Za-z_][A-Za-z0-9_]*`, at most 128 UTF-8 bytes.
Every container has exactly the shown argument count. `N` is a
canonical unsigned decimal integer from `0` through `1,048,576`: `0` or a nonzero digit followed by
decimal digits, with no sign, separator, exponent, leading zero, or suffix. The provider preserves
every type occurrence as a distinct node; it does not intern or resolve equal spellings.

Construction and operations use these reserved expression shapes:

```ts
Pair({ left, right: other })
MaybeI32.none()
MaybeI32.some(value)
FixedArray<i32, 2>([left, right])
Vec<i32>([left, right])
clone(value)
shared(value)
downgrade(sharedValue)
borrow(place)
borrowMut(place)
push(vectorPlace, value)
```

String literals use one exact single- or double-quoted spelling with no escape, carriage-return, or
newline character. `base.field` and `base[index]` are also admitted. Object initializers contain
only identifier shorthand or `identifier: expression` entries. A shorthand record preserves both
the identifier and its source-identical reference-expression edge, so arena ownership remains
explicit. Array literals contain no holes or spreads. Reserved forms require their exact spelling,
delimiters, and arity. The syntax provider
does not decide whether `Pair` or `MaybeI32` denotes a declaration, whether a projection is valid,
or whether an expression is a legal place. Those are later compiler checks.

The exact TypeScript-valid enum match form is:

```ts
match(value, {
  "MaybeI32.none": () => zero,
  "MaybeI32.some": (item) => item,
})
```

The second argument is an object literal. Every key is one unescaped, double-quoted
`"Type.variant"` spelling. Every value is an unmodified arrow with either zero parameters or one
parenthesized identifier and an expression body. Arms remain source ordered. The provider records
the type and variant substrings inside the key, optional payload binding, arrow token, and result
expression. Exhaustiveness and reachability are semantic.

The exact TypeScript-valid weak-upgrade statement form is:

```ts
upgradeWeak(weakValue, (strong) => {
  use(strong);
}, () => {
  expired();
});
```

It has exactly three arguments: the weak expression, a one-identifier success arrow with a block
body, and a zero-parameter failure arrow with a block body. The DTO's `as_span` and `else_span`
authenticate the success and failure `=>` tokens respectively. The call must be a complete
semicolon-terminated expression statement. Weak upgrade is one structured statement and never
produces a nullable syntax value.

There is no explicit move DTO. A reference, argument, initializer, assignment, return, field, or
element use is source syntax; Copy-versus-move classification belongs exclusively to semantics.

The names `__proto__`, `prototype`, and `constructor` are forbidden for every declaration,
identifier, field, variant, initializer, binding, and projection. This restriction is lexical and
does not grant JavaScript property semantics.

## 3. Wire envelope and source units

The UTF-8 JSON response has exactly:

```text
{ schema_version: 4, files: SourceUnit[], diagnostics: ProviderDiagnostic[] }
```

Files are in dense canonical `FileId` order. A source unit contains exactly:

```text
{ id, path, imports, type_syntax, data_declarations, functions }
```

Imports retain the exact protocol-v3 encoding. Imports precede every data/function declaration.
`data_declarations` is the source-ordered struct/enum inventory. `functions` is source ordered.
Their spans authenticate relative top-level order; separating the arrays does not authorize source
reordering.

## 4. Flat type-syntax arena

`type_syntax` is a module-wide canonical postorder arena. Each node has `{ span, kind }`. The exact
kind tags are `missing`, `named`, `string`, `vec`, `shared`, `weak`, `borrow`, `borrow-mut`, and
`fixed-array`. A fixed-array node carries both exact `length_spelling` and parsed numeric `length`;
they must agree and `length` is at most `1,048,576`. Child IDs are unsigned `u32` indices strictly
less than their parent ID.

Every field, variant payload, parameter, result, and local annotation owns one arena root. Every
non-missing node is reachable from exactly one root; sharing, cycles, forward edges, or orphan nodes
are invalid. Repeated source spellings therefore have distinct nodes and spans. Nesting is at most
128 nodes. A `missing` node uses an empty insertion span.

The arena is syntax identity only. A node does not carry a resolved declaration or stored `TypeId`.

## 5. Data declarations

A data declaration contains its complete span, optional `export` token span, and a tagged `kind`.
Both struct and enum kinds retain the `interface`, `extends`, exact marker, brace, name, member, and
semicolon spans. Members are nonempty and strictly source ordered.

A struct field contains `{ span, name, colon_span, type_syntax, semicolon_span }`.

An enum variant contains `{ span, name, colon_span, payload_type, none_span, semicolon_span }`.
Exactly one of these representations is valid:

- payload-free: `payload_type = null` and `none_span` is the exact `ZrynaNone` span;
- payload-bearing: `payload_type` is a type-arena root and `none_span = null`.

The provider does not assign field ordinals or discriminants. Source order is the only syntax fact.

## 6. Functions, statements, and expressions

Protocol-v3 block, statement, and expression arenas remain canonical preorder/postorder arenas in
v4. V4 parameters, results, and local declarations reference the module type arena by `u32` ID.
Assignment targets use an expression ID so projections can remain source-faithful.

Existing scalar/control-flow tags retain their v3 fields. New expression tags are:

```text
string-literal, struct-construction, enum-construction,
fixed-array-construction, vec-construction, field-access, index,
clone, shared, downgrade, borrow, borrow-mut, vec-push, match
```

New statement tags are `expression-statement` and `weak-upgrade`. Initializer fields and match arms
are closed, bounded, source-ordered records. Expression edges remain canonical postorder and each
expression node has one owner. The adapter records syntax but never checks types, field completeness,
variant payload requirements, bounds, mutability, ownership, borrow overlap, match exhaustiveness,
or weak lifetime.

## 7. Exact limits

Protocol-v3 limits remain unchanged. Protocol v4 additionally applies:

| Inventory                                      | Limit |
| ---------------------------------------------- | ----: |
| data declarations per module                   | 4,096 |
| data declarations per project                 | 16,384 |
| fields or variants per declaration             | 1,024 |
| fields plus variants per project              | 65,536 |
| type-syntax nodes per module                  | 65,536 |
| type-syntax nodes per project                | 262,144 |
| type-syntax nesting                               | 128 |
| object initializers per construction            | 1,024 |
| array/vector elements per construction          | 4,096 |
| construction initializers/elements per project | 65,536 |
| match arms per expression                       | 1,024 |
| match arms per project                         | 65,536 |
| fixed-array length spelling bytes                  | 10 |

The response remains at most 64 MiB, authoritative aggregate source remains at most 8 MiB,
provider diagnostics remain at most 256, and verifier diagnostics including a terminal exhaustion
diagnostic remain at most 256. A string-literal spelling is bounded by the 8 MiB aggregate source
and response ceilings. Counts and byte totals use checked arithmetic. Boundary proof is split to
keep the portable fast gate small:

- unchanged transport, file, source, import, binding, function, parameter, block, statement,
  expression, local, call-argument, parser-nesting, response, and diagnostic limits retain their
  exact/first-extra authority in `adapters/typescript-6/test/worker-v3.test.mjs`; preflight runs
  that suite beside v4 so widening either protocol fails the same gate;
- v4-specific module/project declaration, member, type-arena, construction-operand, match-arm,
  type-depth, and fixed-array boundaries use lowered exact/first-extra cases in
  `adapters/typescript-6/test/worker-v4.test.mjs`, plus production-constant schema fixtures in
  `tests/syntax-protocol-v4.test.mjs`; and
- compiler-side response, diagnostic, arena owner/edge/depth, checked project-total, and
  `SourceMap` boundaries are independently exercised by `zryna-syntax::v4` tests and the
  `zryna-frontend` worker-process contract.

Lowered adapter limits exist only under `NODE_ENV=test`; production constants remain frozen in
`limits-v4.mjs`, the JSON Schema, and `zryna-syntax::v4`.

## 8. Verification and diagnostics

The verifier authenticates schema version, complete file inventory, normalized paths, every span,
exact token text, containment, ordering, arena ownership, postorder/preorder, depth, reserved form,
identifier restriction, canonical decimal spelling, and all local/project budgets. JSON objects are
closed and duplicate object keys are rejected before ordinary JSON decoding.

Stable compiler-side v4 diagnostics are:

- `ZRYNA-Y4001`: malformed, unknown, duplicate, foreign, or noncanonical protocol-v4 snapshot;
- `ZRYNA-Y4002`: non-source-faithful v4 node, span, token, ordering, or arena claim; and
- `ZRYNA-F1401`: deterministic protocol-v4 resource exhaustion.

Provider diagnostics remain advisory and cannot substitute for declaration, type, layout,
ownership, borrow, drop, IR, runtime, or backend verification.

## 9. Explicit non-goals

Protocol v4 does not define semantics, nominal resolution, layout, public aggregate ABI, allocation,
runtime helpers, code generation, manifest v3, CLI selection, exceptions, closures, async, classes,
user generics, user destructors, raw pointers, unsafe access, reflection, tracing GC, Wasm GC, WASI,
DOM access, FFI, threads, atomics, freestanding targets, or production-support claims.
