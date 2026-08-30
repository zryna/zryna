# Zryna native backend boundary

The object emitter accepts only `VerifiedMirModule` and the capability returned by exact target
selection. It uses `cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, and
`cranelift-object` 0.135.1 with `target-lexicon` 0.13.5. Independent inspection uses `object`
0.39.0. All versions are exact workspace pins; no installed LLVM, compiler, assembler, or linker
is a dependency of this backend or object emission.

The only target is `x86_64-unknown-linux-gnu`: ELF64, little-endian, relocatable, baseline x86-64,
non-PIC, no optimization, no unwind output, and no per-function sections. Exported `i32`
functions use the System V AMD64 convention, external linkage, and the exact sealed
`zryna_v1_e_<logical>` symbol. The backend never constructs this mapping.

After encoding, the closed audit checks size, format, architecture, endianness, object kind,
nonzero global text symbols, exact declaration order, undefined symbols, and relocations. The
current pure leaf profile permits neither undefined symbols nor relocations. Only then is
`ValidatedNativeObjectArtifact` constructed. Stable codes distinguish unsupported target
(`ZRYNA-N3001`), code generation (`ZRYNA-N3002`), and object audit (`ZRYNA-N3003`).

Textual LLVM output remains a compatibility proof, not the object implementation. Driver-owned
linking and execution consume only the sealed object under a separate contract and do not expand
this backend boundary. Startup, calls, runtime helpers, FFI, Windows/macOS output, and Boolean
source/IR are outside this slice.
