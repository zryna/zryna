# M2 direct core WebAssembly backend

Status: implemented as a sealed backend with typed execution, composed by the public driver only
for explicit `--profile control-flow-v1`. The backend itself cannot select protocol v3, publish a
manifest, or claim cross-target equivalence.

## Authority boundary

`zryna_backend_webassembly::emit_control_flow` accepts only
`zryna_ir::control_flow_v1::VerifiedProgram`. Raw control-flow IR, syntax snapshots, source names,
paths, and provider identities cannot cross this API. The existing M1 `emit` API and public M1
driver remain unchanged.

Modules and functions are flattened once in sealed canonical order. Function indices are assigned
from verified module and declaration identities, while type indices use deterministic first-use
order over exact mapped signatures. Dependency exports and unexported functions remain private.
Only `public_export()` entry functions appear in the export section under their sealed scalar ABI
WebAssembly names.

## Direct lowering

The backend writes a canonical WebAssembly 1.0 core module directly; it does not translate
ECMAScript, WAT, native output, or an external assembler. A nonempty artifact contains exactly
type, function, export, and code sections; an empty verified program emits only the core header.
It has no imports, tables, memory, globals, start function,
element or data segments, custom sections, tags, WASI, WIT, Component Model, GC, threads, SIMD,
reference types, or ambient host capability.

Every current `ControlFlowV1` instruction has one exact lowering:

| Verified instruction | Core WebAssembly behavior |
| --- | --- |
| `BoolLiteral`, `I32Literal` | canonical `i32.const`, with Boolean values exactly `0` or `1` |
| `I32Add`, `I32Sub`, `I32Mul` | wrapping `i32.add`, `i32.sub`, or `i32.mul` |
| `I32Neg` | `i32.const 0`, operand, then `i32.sub` |
| `Eq`, `Ne` | `i32.eq` or `i32.ne` over equal verified scalar types |
| signed comparisons | `i32.lt_s`, `i32.le_s`, `i32.gt_s`, or `i32.ge_s` |
| `DirectCall` | direct `call` to the sealed flattened function index |

Function parameters retain their core parameter locals. Instruction results use dense function
locals. A fixed set of scratch locals copies every edge argument before any destination block
parameter is changed, preserving parallel SSA edge semantics, including loop-carried swaps.

Each function uses one state local and a constant-host-stack-depth dispatcher. A verified block
executes only when its dense block identity equals the state. `Jump` transfers parallel edge
arguments and updates the state; `Branch` selects one exact Boolean arm; `Return` produces the
verified result. An impossible state reaches `unreachable`. This dispatcher does not duplicate
source expressions or recursively encode the CFG.

## Validation and capability audit

The complete bytes first pass the exactly pinned WebAssembly validator. A separate exhaustive
profile audit then proves the exact section inventory, type arities, function type indices, export
names and indices, function and body counts, local counts, and operator allowlist. Any unsupported
section, operator, index, type, local, export, or trailing observation fails with `ZRYNA-W2004`.
Validation failure is `ZRYNA-W2003`; unsupported verified signatures and impossible verified
identity/index claims use `ZRYNA-W2001` and `ZRYNA-W2002`.

Encoding is incremental and bounded. The exact selected byte budget is checked before every
append or reservation. The production cap is 32 MiB; allocation failure and the first byte beyond
the selected budget fail with `ZRYNA-W2005`. No partial artifact is returned.

## Typed execution boundary

Internal driver tests execute the already validated artifact bytes through the pinned direct
Node.js capability. The parent passes those exact bytes over standard input to an inline module
script, so execution never reopens a staged module or script pathname. The runtime clears the
environment, bounds script, module, standard output, and standard error bytes, imposes a deadline,
and confirms process-tree and input-writer cleanup.

An `i32` argument uses its exact signed 32-bit carrier. A `bool` argument must be a typed scalar ABI
Boolean and crosses the core boundary as exactly `0` or `1`. The host requires one exact four-byte
little-endian result frame; shared scalar ABI normalization rejects any Boolean result other than
`0` or `1`. This host validation complements, but does not replace, the module validator and
capability audit.

## Verification evidence

Focused backend tests cover every instruction and terminator, public and private function-index
mapping, sealed export inventory, repeated byte-identical emission, Boolean lanes, parallel edge
transfer, the exact artifact-budget boundary, validation failures, and forbidden capability
sections and operators. Driver integration executes typed `i32` and `bool` exports from the same
sealed M2 program and rejects a noncanonical Boolean result. The portable execution path is
covered on Linux and Windows before merge.

## Remaining M2 gates

The [M2 native MIR profile](M2_NATIVE_MIR.md) and
[M2 Linux x86-64 native backend](M2_NATIVE_BACKEND.md) now independently reseal and execute the
same verified authority. The explicit-profile driver and
[manifest-v2 transaction](M2_MANIFEST_V2.md) and the independent
[fixed-oracle three-target gate](M2_CONFORMANCE.md) are implemented. Authenticated public
documentation, website, and live-deployment closure remain Issue #57.
