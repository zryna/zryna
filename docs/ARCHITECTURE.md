# Compiler architecture

## Permanent boundary

Verified Universal IR is the permanent contract. Frontend providers and output backends are replaceable around it.

```text
source files
    ↓
FrontendProvider
    ├── TypeScript 6 adapter (bootstrap)
    ├── TypeScript 7 IPC adapter (planned after a stable upstream API)
    └── native Zryna frontend (planned)
    ↓
RawProjectSyntaxSnapshot v1 (declarations) or v2 (executable syntax)
    ↓ exact file-set, path, budget, graph, and span verification
ProjectSyntaxSnapshot v1 or zryna_syntax::v2::ProjectSyntaxSnapshot
    ↓
Zryna name resolution and strict semantic checking
    ↓
unverified Universal IR
    ↓
mandatory IR verifier
    ↓
VerifiedProgram
    ├── JavaScript IR and printer
    ├── WebAssembly lowering and binary emission
    └── native MIR, codegen, object emission, and linking
```

No provider-specific syntax-kind number, node identity, symbol identity, or type identity may cross
`zryna-frontend`. Protocol-v2 syntax is owned by the lower `zryna-syntax` foundation crate so
semantic lowering never depends on a replaceable provider.

## Authority of each phase

1. `zryna-architecture` proves that the repository can be inspected completely and matches its declared graph.
2. A frontend provider reads compatible syntax and produces an untrusted, provider-neutral raw
   snapshot.
3. `zryna-syntax` verifies protocol-v2 file identity, budgets, source spans, lexical order, and the
   canonical flat expression graph before constructing opaque executable syntax.
4. Zryna semantics rejects unsupported dynamic constructs and assigns exact types.
5. `zryna-ir` represents exact operations such as `I32Add`; generic target-dependent arithmetic is forbidden.
6. The IR verifier is the only constructor of a backend-accepted verified program.
7. The JavaScript backend preserves Zryna behavior using direct syntax or explicit helpers.
8. The WebAssembly backend maps exact Zryna operations directly to validated core WebAssembly. Browser bindings and WASI capabilities remain explicit host profiles.
9. Native lowering adds control flow, layout, moves, drops, and calling conventions before code generation.

## Dependency direction

```text
source ───────────────→ diagnostics
  ├──────────────────────→ syntax
  └──────────────────────→ frontend contracts ──→ syntax
diagnostics ─────────────→ syntax
  └──────────────────────────────────────────────┐
source ──────────────────────────────────────────┤
syntax ──────────────────────────────────────────┤
                                                  ↓
                                            Zryna semantics
                           ↓
                     verified Zryna IR
                  ┌────────────┼────────────┐
                  ↓            ↓            ↓
            JavaScript   WebAssembly    native MIR
                              ↓              ↓
                         validation       codegen
                                             ↓
                                           linker
```

`zryna-driver` is the only library allowed to orchestrate all phases. The CLI calls the driver and architecture engine; individual backends do not call one another.

The permanent direction is `frontend -> syntax -> semantics -> IR`. `zryna-semantics` is a compiler
component and cannot depend on `zryna-frontend`; backends cannot depend on either provider layer.
The architecture engine has a negative graph fixture for both forbidden edges.

## Source and diagnostic authority

`zryna-source` is below diagnostics and every provider. One immutable bounded `SourceMap` owns
the exact UTF-8 text and assigns dense snapshot-local `FileId` values after normalized path
sorting. Source paths are portable workspace-relative ASCII with `/` separators and an
ASCII-case-folded uniqueness identity. Host filesystem path behavior is never used to interpret
provider paths.

All internal spans are zero-based half-open UTF-8 byte ranges. Opaque `FileId` and `Span` values
retain the issuing source-map identity. A span is authoritative only when constructed or resolved
through that exact `SourceMap`, which proves the file exists and both endpoints are ordered, in
bounds, and UTF-8 character boundaries. Source content is never normalized.

Diagnostics use one primary-location variant: source span, workspace path, or global. Source
diagnostics are resolved and sorted through the source map before stable text or versioned JSON
is emitted; a forged or mismatched span fails rendering rather than producing a misleading path.
IR verification also resolves every expression span with the compilation source map before it can
construct `VerifiedProgram`.

## Verified Universal IR trust contract

`Program`, `Function`, and `Expr` are untrusted compiler claims. `zryna_ir::verify` is the only
constructor of `VerifiedProgram`; backends iterate opaque `VerifiedFunction` views and cannot
recover the raw program. The current `I32V1` profile admits only `i32` parameters, results,
literals, and signed wrapping addition. `bool` and `unit` remain profile-gated until every active
universal backend implements their specified representation and behavior.

A verified function proves all of the following:

- its logical export is bounded ASCII matching `[A-Za-z_][A-Za-z0-9_]*`, is safe for the current
  ECMAScript binding and LLVM bare-identifier surfaces, and is unique exactly and under ASCII
  case folding;
- its body is in the same arena and has the declared result type;
- every operand is a distinct earlier entry, every entry has exactly one owner, and the complete
  tree is stored in exact left-to-right postorder without shared or orphan entries;
- its maximum expression depth is 128 and every expression span resolves in the exact compilation
  `SourceMap`; and
- program, parameter, expression, export-byte, and diagnostic budgets remain within the public
  constants in `zryna-ir`.

Verification and current JavaScript emission are iterative and bounded. A logical export remains a
language-level identity; later backend ABI contracts must define any concrete symbol encoding
without weakening collision checks. Current native lowering is the only constructor of an opaque,
read-only `MirModule`, so the LLVM proof path cannot bypass `VerifiedProgram`. A separate MIR
verifier is still required before transformations or other MIR producers are introduced; calling
conventions, object emission, and linking remain later mandatory gates.

## Initial numeric contract

The first vertical slice defines signed 32-bit wrapping addition:

```text
Zryna IR:      I32Add(a, b)
JavaScript:  (a + b) | 0
WebAssembly:   i32.add
LLVM IR:     add i32 %a, %b
```

Future integer operations must specify width, signedness, overflow, conversion, comparison, and JavaScript representation before implementation.

## WebAssembly profiles

The first WebAssembly backend will emit a core module directly from `VerifiedProgram`; it will not translate JavaScript or native output. Its first slice will consume the `I32V1` profile and export pure functions over `i32`, validate every emitted binary, and execute conformance fixtures in a pinned runtime. `bool` will be enabled only by a later universal profile implemented by every active backend.

Browser integration adds a generated JavaScript loader without giving the loader authority over language semantics. WASI and the Component Model are a later capability-bearing profile with separately pinned interface and ABI versions. Filesystem, network, clock, randomness, and environment access are unavailable unless a declared host profile imports them.
