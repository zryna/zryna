# M2 Linux x86-64 native backend

Status: implemented as an object, link, and typed-execution boundary for verified
`ControlFlowV1`, composed by the public driver only for explicit `--profile control-flow-v1`.
The backend itself cannot activate protocol v3, publish a manifest, or claim three-target
conformance. The M1 object API, relocation-free audit, artifact type, CLI, and executable contract
remain unchanged.

## Authority chain

The M2 emitter accepts only `zryna_native_mir::control_flow_v1::VerifiedProgram` and the capability
returned by exact selection of `x86_64-unknown-linux-gnu`. It never accepts raw MIR, M1 verified
MIR, an arbitrary ABI module, caller-selected symbols, or caller-provided object bytes.

The resulting `ValidatedControlFlowNativeObjectArtifact` retains both the independently audited
bytes and a clone of the scalar ABI authority used during emission. Typed invocation is prepared
through that artifact, so an object cannot be paired with a different program or ABI.

## Deterministic lowering

Pinned Cranelift `0.135.1` emits baseline non-PIC ELF64 relocatable code with optimization disabled,
unwind information disabled, and one shared `.text` section. All bodies are declared before any
definition, so forward and cross-module direct calls use only sealed function identities.

Each body is local and uses its verified `zryna_m2_i_m<module-id>_f<declaration-index>` symbol.
Only entry-module exports receive global scalar-ABI wrappers under `zryna_v1_e_<logical-name>`.
Internal calls always target local bodies, never wrappers. Bodies use typed `i32` and canonical
`i8` Boolean values. Public wrappers accept System V `i32` carriers, trap unless every Boolean
argument is exactly `0` or `1`, narrow admitted Boolean arguments, and zero-extend Boolean results
to exact `0` or `1`.

Every MIR block becomes one Cranelift block. Deterministic reverse postorder ensures dominators are
emitted before instruction uses even when dense block order differs. Block parameters and direct
branch argument vectors preserve simultaneous SSA edge transfer. Branches explicitly compare the
verified Boolean lane with `1`; arbitrary nonzero integers are not treated as Zryna `true`.

The lowering exhaustively implements Boolean and `i32` literals, wrapping add, subtract, multiply,
negate, equality, inequality, signed comparisons, direct calls, returns, jumps, branches, and
reducible loops. No operation may introduce a runtime helper or external call.

## Closed ELF object audit

The post-encode audit reparses the complete bytes with pinned `object` `0.39.0` and requires:

- ELF64, little-endian, x86-64, relocatable output no larger than 8 MiB;
- exact ordered sections: `.text`, optional `.rela.text`, `.note.GNU-stack`, `.symtab`, `.strtab`,
  and `.shstrtab`, with fixed kinds and ELF flags;
- no dynamic symbols or dynamic relocations;
- one fixed file symbol, the complete local body inventory, and complete global wrapper inventory;
- nonzero, ordered, nonoverlapping text symbol ranges wholly contained in `.text`;
- no undefined, weak, common, absolute, data, TLS, startup, loader, custom, writable-code, or
  externally supplied symbol claim; and
- exact one-for-one relocation correspondence with every verified direct call plus every public
  wrapper-to-body call.

The only admitted relocation has source `.text`, raw ELF type `R_X86_64_PLT32`, normalized kind
`PltRelative`, encoding `X86Branch`, width 32, explicit addend `-4`, no subtractor, an ordered
four-byte displacement wholly inside the exact expected caller, an x86-64 direct-call opcode at
the exact relocation site, and the exact expected local body target. Missing, extra,
duplicated, redirected, undefined, wrapper-targeted, GOT, absolute, section, differently encoded,
or differently sized relocations fail with `ZRYNA-N3103`. The allowlist is pinned to the observed
Cranelift/object versions; upgrading either dependency requires a contract and fixture update.

## Internal link and execution

The driver prepares a typed invocation only through the artifact-bound scalar ABI, generates the
existing bounded C11 scalar harness, and reuses the validated canonical `/usr/bin/gcc` plus GNU ld
capability. Linking uses direct arguments without a shell, a cleared environment, fixed hardening
flags, tool identity revalidation, bounded output, and a 30-second deadline. The harness is limited
to 128 KiB and the audited executable to 32 MiB.

Each link, publication, and run uses a new mode-`0700` staging directory. The driver retains an
open handle and exact device/inode identity for that directory. Inputs, tool arguments, executable
audit, and process launch resolve through the retained Linux `/proc/<pid>/fd/<dirfd>` capability;
input writes and cleanup are handle-relative. The live binding is revalidated before and after
writes and at authority transitions. Persistent replacement or permission drift fails closed, and
cleanup never removes a replacement directory.

Like the workspace contract, this is deterministic compiler containment rather than an
operating-system sandbox against a concurrently hostile process running with the same user
authority. Such a process can inspect or mutate that user's open descriptors and must not mutate a
workspace or compiler-owned output root during a build. Protecting against that actor requires a
separate OS sandbox boundary.

Execution uses retained audited executable bytes in a fresh private stage, a five-second deadline,
bounded output, an isolated process group, and confirmed descendant cleanup. Success must exit
normally, write exactly four result bytes, and write no stderr. Nonzero exit, signal, timeout,
framing error, overflow, or cleanup failure is a host/process failure, not a language result.
Scalar ABI normalization maps a noncanonical Boolean result to `InvalidTargetResult`.

## Stable diagnostics and unsupported surface

| Code | Boundary |
| --- | --- |
| `ZRYNA-N3001` | exact native target selection shared with M1 |
| `ZRYNA-N3102` | M2 code-generation or sealed-invariant failure |
| `ZRYNA-N3103` | M2 closed object-audit failure |
| `ZRYNA-N4001`–`ZRYNA-N4022` | driver toolchain, staging, link, execution, and frame boundary |

Object emission invokes no compiler, assembler, linker, loader, runtime, or generated code. Linux
link/run is available only on x86-64 Linux. Other hosts retain `ZRYNA-N4002` before staging or
process work. FFI, libc calls from generated bodies, memory, assembly, Windows/macOS objects, user
linker flags and general executables remain unsupported.

## Evidence and remaining gates

Focused backend tests cover every operation and terminator, Boolean wrappers, branches, loops,
block arguments, direct calls, deterministic bytes, exact sections and symbols, and the closed
section, symbol, relocation, opcode-site, corruption, and oversize attack matrix. Driver tests link
the same invocation twice and run fixed wrapping arithmetic, Boolean, cross-module direct-call,
branch, and loop observations while retaining the
earlier timeout, overflow, signal, descendant-cleanup, tool-replacement, failed-link, and executable
audit tests. A stage-replacement fixture proves the retained directory identity.

Issue #55 composes this boundary through public profile selection and
[manifest v2](M2_MANIFEST_V2.md). Issue #56 implements the fixed three-target oracle and required
[aggregate gate](M2_CONFORMANCE.md). Issue #57 records authenticated website import, deployment,
and live provenance separately from this native backend contract.
