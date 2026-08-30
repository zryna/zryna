# Linux x86-64 native object v1

Status: implemented for the universal `I32V1` leaf-function slice.

## Fixed target and format

- request string: exactly `x86_64-unknown-linux-gnu`;
- output suffix: `.o`;
- format: ELF64, little-endian, relocatable (`ET_REL`), x86-64 (`EM_X86_64`);
- code model: baseline x86-64, non-PIC, no optimization, unwind output disabled, shared text
  section rather than per-function sections;
- implementation: Cranelift 0.135.1 and target-lexicon 0.13.5;
- independent parser: object 0.39.0.

Other triples, aliases, case variants, musl, Windows, macOS, and other architectures fail with
`ZRYNA-N3001`. The host platform never selects the output target implicitly.

## ABI and object audit

Every function has only `i32` parameters and one `i32` result, uses the System V AMD64 calling
convention, external linkage, and the exact scalar ABI v1 symbol `zryna_v1_e_<logical>`. Native MIR
retains that sealed mapping; the backend does not prefix or sanitize names.

The target selector and ISA capability prove the exact requested triple; ELF itself cannot
distinguish GNU from another environment. Encoded bytes are private until a second parser proves
ELF64/x86-64/little-endian/relocatable identity, bounded total size, the exact closed section
sequence and flags, one nonempty global text function per sealed export in declaration order, no
undefined symbols, and zero relocations. These last two requirements are deliberately strict
because the current operations are parameters, constants, and wrapping addition only. Any future
call, data, runtime, or relocation profile requires a new documented allowlist before
implementation.

Publication accepts only the sealed audited artifact and atomically creates
`.zryna/out/<stem>.o`; it never replaces an existing destination.

## Exclusions

This contract does not provide product linking, startup, `main`, loading, execution, a bundled
runtime, calls, memory, FFI, optimization, debug/unwind information, Windows/macOS objects, or
Boolean source/IR. Linux test code may link the object solely to verify full-width results and the
System V boundary. Product link/run behavior belongs to the next contract.
