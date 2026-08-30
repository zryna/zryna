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
VerifiedProgram + sealed scalar ABI module
    ├── JavaScript IR and printer
    ├── WebAssembly lowering and binary emission
    └── raw native MIR → mandatory MIR verifier → VerifiedMirModule
                                         └── codegen, object emission, and linking
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
4. Zryna semantics resolves parameter names, rejects unsupported or dynamic constructs, assigns
   exact types, and lowers the accepted source subset to raw Universal IR.
5. `zryna-abi` verifies scalar signatures, logical exports, target mappings, and typed host values.
6. `zryna-ir` represents exact operations such as `I32Add`; generic target-dependent arithmetic is forbidden.
7. The IR verifier is the only constructor of a backend-accepted verified program and embeds the
   matching sealed scalar ABI module by declaration index.
8. The JavaScript backend consumes sealed ABI export names and emits deterministic ESM with
   explicit scalar-boundary checks. The driver publishes complete `.mjs` files create-only through
   a validated capability for the workspace's declared `.zryna/out` directory.
9. The WebAssembly backend maps exact Zryna operations directly to validated core WebAssembly. Browser bindings and WASI capabilities remain explicit host profiles.
10. Native lowering creates explicit typed native claims; the native MIR verifier is the only
   constructor of the codegen-accepted `VerifiedMirModule`.
11. Later native profiles add control flow, layout, moves, drops, and public calling conventions
    before object generation and linking.

## Dependency direction

```text
source ───────────────→ diagnostics
  ├──────────────────────→ syntax
  └──────────────────────→ frontend contracts ──→ syntax
diagnostics ─────────────→ syntax
  └──────────────────────────────────────────────┐
source ──────────────────────────────────────────┤
syntax ──────────────────────────────────────────┤
scalar ABI ─────────────→ export-name preflight ─┤
                                                  ↓
                                            Zryna semantics
                                                  ↓
                                          unverified Zryna IR
scalar ABI ──────────────────────────────────────┤
                                                  ↓
                                   verified IR + sealed ABI
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

Verified protocol-v2 snapshots also retain the opaque identity of the issuing `SourceMap`, even
when the map contains no files. The semantic boundary requires that exact identity and rejects a
snapshot verified against any independently constructed map.

## Current strict semantic subset

The first source-to-IR gate accepts exactly one designated source file, conventionally ending in
`.zry`, with at least one exported function. Parameters and results require explicit `i32` or
`bool` annotations. Parameter names are
function-local and unique; bodies contain exactly one value return; expressions are parameter
references, in-range decimal `i32` literals, Boolean literals, and `i32 + i32`. Export names are
preflighted by the scalar ABI authority before IR verification.

The semantic phase owns rejection of missing annotations, `any`, unknown types, duplicate or
unresolved names, invalid export identities, out-of-range integers, invalid addition, mismatched
returns, and unsupported entrypoint shape. Diagnostics are source-located where a source token
exists, deterministic, and capped at 256 entries. Protocol-v2 resource limits are compile-time
constrained to the corresponding IR limits.

Semantic validity is not backend availability. Raw IR can represent `BoolLiteral`, but the current
`I32V1` verifier rejects every `bool` signature or expression before constructing
`VerifiedProgram`. A future universal Boolean profile must be implemented by every active backend
before that gate can be enabled.

## Verified Universal IR trust contract

`Program`, `Function`, and `Expr` are untrusted compiler claims. `zryna_ir::verify` is the only
constructor of `VerifiedProgram`; backends iterate opaque `VerifiedFunction` views and cannot
recover the raw program. The verifier delegates logical-name, collision, scalar-signature, and
target-mapping authority to `zryna-abi`, then embeds the sealed module beside the private program.
The current `I32V1` profile admits only `i32` parameters, results, literals, and signed wrapping
addition. Scalar ABI v1 specifies `bool`, but `bool` and `unit` remain profile-gated until every
active universal backend implements their specified representation and behavior.

A verified function proves all of the following:

- its scalar ABI export has a bounded logical name matching `[A-Za-z_][A-Za-z0-9_]*`, is unique
  exactly and under ASCII case folding, and carries deterministic JavaScript, WebAssembly, and
  Linux x86-64 target names;
- its body is in the same arena and has the declared result type;
- every operand is a distinct earlier entry, every entry has exactly one owner, and the complete
  tree is stored in exact left-to-right postorder without shared or orphan entries;
- its maximum expression depth is 128 and every expression span resolves in the exact compilation
  `SourceMap`; and
- program, parameter, expression, export-byte, and diagnostic budgets remain within the public
  constants in `zryna-ir`.

Verification and current JavaScript emission are iterative and bounded. The normative
[scalar ABI v1](../spec/abi/SCALAR_V1.md) defines target names, carriers, invocation, and typed
observation. The JavaScript emitter implements the sealed export mapping and boundary checks for
the executable `I32V1` profile. Its Boolean carrier helper is exercised against the shared ABI
fixture, but does not admit Boolean source or Boolean IR. WebAssembly and native public wrappers
remain boundary proofs until their later issues adopt the same mappings.

## Current JavaScript artifact path

`zryna-driver::compile_javascript` is the source-connected JavaScript build boundary. It compiles
an authenticated source map through semantics and verified IR, emits one deterministic ECMAScript
module, and publishes `<stem>.mjs` only when the destination does not already exist. The caller
must first derive a `JavaScriptOutputRoot` capability from an absolute workspace path. That
capability resolves only the declared `.zryna/out` location and rejects any persistent path
component that is missing, non-directory, a symbolic link, or a Windows reparse point. The full
chain is revalidated immediately before publication. The artifact stem is one portable ASCII
filename component.

Publication writes, flushes, and synchronizes a create-new sibling temporary file before using a
create-only hard link for the final name. It never replaces an existing file, directory, or link.
A failed source, backend, or publication phase does not report a new artifact; an existing
destination is preserved byte-for-byte. Concurrent hostile replacement of filesystem ancestors
after validation is outside this process-local publication proof, and directory-entry crash
durability is not claimed. Temporary-name cleanup failure after successful publication is returned
as a warning without hiding the successfully published artifact.

The integration suite imports and executes generated modules with Node.js 22.22.1. This is a
compiler conformance harness, not yet a public runtime command. The user-facing build/run CLI is a
later M1 gate.

Native MIR has its own consumed raw-to-verified boundary. Raw functions explicitly claim a symbol,
provisional internal convention, typed signature, dense typed value definitions, operations, and a
result. The iterative bounded verifier proves `i32` types, unique safe symbols, the admitted
convention, exact definitions, strict-predecessor operands, acyclicity, a typed result, and resource
limits before constructing `VerifiedMirModule`. Native codegen accepts only that wrapper. Repeated
SSA uses and bounded dead values are valid; Universal IR's tree-ownership rule is not copied into
MIR. The provisional convention and raw symbol spelling are proof inputs, not scalar ABI v1.
Control-flow dominance, public calling conventions, object emission, and linking remain later
mandatory gates.

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
