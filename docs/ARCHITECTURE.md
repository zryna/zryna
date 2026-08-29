# Compiler architecture

## Permanent boundary

Verified Universal IR is the permanent contract. Frontend providers and output backends are replaceable around it.

```text
source files
    ↓
FrontendProvider
    ├── TypeScript 6 adapter (bootstrap)
    ├── TypeScript 7 IPC adapter (planned after a stable upstream API)
    └── native UTS frontend (planned)
    ↓
ProjectSyntaxSnapshot v1
    ↓
UTS name resolution and strict semantic checking
    ↓
unverified Universal IR
    ↓
mandatory IR verifier
    ↓
VerifiedProgram
    ├── JavaScript IR and printer
    └── native MIR, codegen, object emission, and linking
```

No provider-specific syntax-kind number, node identity, symbol identity, or type identity may cross `uts-frontend`. Providers normalize supported syntax into UTS-owned enums and records.

## Authority of each phase

1. `uts-architecture` proves that the repository can be inspected completely and matches its declared graph.
2. A frontend provider reads compatible syntax and produces an immutable snapshot.
3. UTS lowering maps the snapshot into provider-independent syntax.
4. UTS semantics rejects unsupported dynamic constructs and assigns exact types.
5. `uts-ir` represents exact operations such as `I32Add`; generic target-dependent arithmetic is forbidden.
6. The IR verifier is the only constructor of a backend-accepted verified program.
7. The JavaScript backend preserves UTS behavior using direct syntax or explicit helpers.
8. Native lowering adds control flow, layout, moves, drops, and calling conventions before code generation.

## Dependency direction

```text
diagnostics ───────────────┐
source ────────────────────┤
frontend contracts ────────┤
                           ↓
                     UTS semantics
                           ↓
                     verified UTS IR
                       ┌────────┐
                       ↓        ↓
                 JavaScript   native MIR
                                  ↓
                               codegen
                                  ↓
                                linker
```

`uts-driver` is the only library allowed to orchestrate all phases. The CLI calls the driver and architecture engine; individual backends do not call one another.

## Initial numeric contract

The first vertical slice defines signed 32-bit wrapping addition:

```text
UTS IR:      I32Add(a, b)
JavaScript:  (a + b) | 0
LLVM IR:     add i32 %a, %b
```

Future integer operations must specify width, signedness, overflow, conversion, comparison, and JavaScript representation before implementation.
