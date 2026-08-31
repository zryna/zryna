# M2 verified native MIR

Status: implemented as a separate internal lowering and mandatory raw-to-verified trust boundary.
The downstream [M2 Linux x86-64 native backend](M2_NATIVE_BACKEND.md) now consumes this authority
for audited object emission and typed link/run. The public driver composes it only under explicit
`control-flow-v1`; this MIR component does not publish or claim three-target conformance. The
existing M1 native MIR, object, and executable paths remain unchanged.

## Authority boundary

M2 native MIR is a separately versioned raw-to-verified boundary. Its lowering entrypoint accepts
only `zryna_ir::control_flow_v1::VerifiedProgram`, creates explicit raw M2 native MIR claims, and
passes the complete result through the M2 native MIR verifier. Raw claims cannot enter native
code generation, construct the opaque verified wrapper, or be recovered from verified views.

The M2 profile remains separate from the root straight-line M1 `raw::Module`, `verify`,
`lower`, and `VerifiedMirModule` APIs. M2 does not reinterpret their types, symbols,
diagnostics, fixtures, or behavior. The separate M2 object emitter accepts only the
verifier-created M2 wrapper.

## Canonical program and lowering

The raw M2 program retains the dense entry-module identity and modules in sealed normalized-path
order. Functions remain in source declaration order. Blocks and values retain their verified dense
identities. Function parameters allocate the first values; blocks follow in dense `BlockId` order,
with each block's parameters before its instruction results.

Lowering preserves every exact `bool` and `i32` type and exhaustively maps `BoolLiteral`,
`I32Literal`, `I32Add`, `I32Sub`, `I32Mul`, `I32Neg`, `Eq`, `Ne`, the four signed comparisons, and
`DirectCall`. A call retains the sealed callee `FunctionIdentity` and arguments in evaluation order;
it never resolves through a source name or target symbol.

Every block retains exactly one `Return`, `Jump`, or `Branch` terminator. Jump and branch argument
vectors remain simultaneous SSA transfers into target block parameters. Lowering must not
sequentialize an edge, introduce mutable target storage, or collapse two branch arms that name the
same target. Native object lowering owns the later target-specific realization of these parallel
edges.

## Symbols and ABI

Every function body has one canonical target-internal symbol:

```text
zryna_m2_i_m<module-id-decimal>_f<declaration-index-decimal>
```

Decimal components are canonical unsigned ASCII with no leading zero except the value `0`.
Source paths, provider identities, source names, and locale never participate. A raw function must
claim the exact derived spelling; the verifier rejects rather than sanitizes any alternative.
`zryna_m2_i_` is reserved for M2 implementation bodies, `zryna_m2_w_` for future public wrappers,
and `zryna_m2_r_` for future runtime helpers.

Only entry-module export claims enter the independently rebuilt scalar ABI v1 module. Their public
Linux symbols remain the ABI-owned `zryna_v1_e_<logical-name>` mapping. Dependency exports and
unexported functions have no public symbol. Internal calls always retain function identities and
therefore address implementation bodies, never public wrappers. Exact and ASCII-case-folded
uniqueness is proved across all admitted internal and public symbols, and the four reserved
namespaces are disjoint.

M2 native MIR represents `bool` and `i32` as distinct exact types. A Boolean can originate only
from a Boolean literal, equality/inequality, or signed comparison and can be consumed as a Boolean
operand or branch condition. It is not an arbitrary nonzero integer. The internal native backend
independently validates the scalar ABI 32-bit Boolean carrier as exactly `0` or `1`, narrows it for
the typed body, and zero-extends a canonical result. Public profile activation remains a later gate.

## Independent verification

The verifier performs bounded collection preflight before proportional graph work, then proves:

- dense module, function, block, and value identities and the exact derived internal symbol;
- exact `bool`/`i32` signatures, definitions, operation operands and results, call signatures,
  return values, branch conditions, and edge arguments;
- one definition for every value, canonical definition order, and dominance of every use;
- exactly one terminator per block, no edge to entry, exact predecessor/successor derivation,
  reachability of every block, and exact edge arity and types;
- reducible loops with one dominating nonentry header and bounded loop nesting;
- an existing acyclic direct-call graph with bounded static depth;
- entry-only public exports, the independently rebuilt scalar ABI, symbol namespace separation,
  and exact plus portable case-folded symbol uniqueness; and
- every deterministic resource budget below with checked arithmetic and bounded diagnostics.

Predecessor and successor views are derived from the sealed terminators; raw MIR cannot provide a
second edge inventory. Verification is iterative at graph and call-depth limits. It neither trusts
the upstream IR verifier's result as its own proof nor accepts a partially verified program.

## Resource budgets

| M2 native MIR resource | Limit |
| --- | ---: |
| Modules | 4,096 |
| Functions per module / program | 4,096 / 16,384 |
| Parameters per function / program | 256 / 262,144 |
| Blocks per function / program | 4,096 / 65,536 |
| Block parameters per block | 256 |
| Values per function / program | 16,384 / 262,144 |
| CFG edges per function / program | 8,192 / 131,072 |
| Raw terminator claims per function / program | 4,096 / 65,536 |
| Direct-call sites per program | 65,536 |
| Direct-call arguments per site | 256 |
| Aggregate direct-call arguments per program | 16,777,216 |
| Edge arguments per edge | 256 |
| Aggregate edge arguments per program | 33,554,432 |
| Static direct-call depth | 128 |
| Verified natural-loop nesting | 128 |
| One internal symbol | 128 bytes |
| Aggregate internal symbol bytes | 2,097,152 bytes |
| One provisional entry export name | 128 bytes |
| Aggregate provisional entry export-name bytes | 2,097,152 bytes |
| Retained diagnostics including the terminal diagnostic | 256 |

The first item beyond a limit fails before later phases act. Exact-limit and first-extra tests are
required for every MIR-owned row. Variable argument collections are bounded during preflight even
when a malformed signature, target, or callee would otherwise prevent ordinary arity checking.

## Stable diagnostics

| Code | Rejected claim |
| --- | --- |
| `ZRYNA-N2101` | missing or unknown entry module |
| `ZRYNA-N2102` | noncanonical module, function, block, or value identity |
| `ZRYNA-N2103` | internal symbol spelling, namespace, or collision |
| `ZRYNA-N2104` | calling convention, signature, scalar type, or Boolean carrier |
| `ZRYNA-N2105` | dependency export or entry-block parameter claim |
| `ZRYNA-N2106` | missing, multiple, or ill-typed terminator, CFG target, or edge contract |
| `ZRYNA-N2107` | instruction operand/result contract or empty function body |
| `ZRYNA-N2108` | unknown value identity |
| `ZRYNA-N2109` | same-block use-before-definition or definition dominance |
| `ZRYNA-N2110` | unreachable block or irreducible CFG |
| `ZRYNA-N2111` | direct callee, signature, or argument contract |
| `ZRYNA-N2112` | direct-call cycle or static call depth |
| `ZRYNA-N2113` | public export or scalar ABI reconstruction |
| `ZRYNA-N2201` | deterministic resource exhaustion |
| `ZRYNA-N2202` | diagnostic exhaustion or bounded internal construction failure |

Diagnostics are retained in canonical module, function, block, value, stable-code, and complete
tie-break order. They identify bounded ordinals and never expose an absolute host path.

## Required evidence and remaining gates

Focused tests provide positive lowering fixtures for every operation and terminator, same- and
cross-module calls, public and private functions, Boolean lanes, diamonds, loops, and parallel edge
swaps. Adversarial raw MIR covers every invariant and stable code. Exact/first-extra budget,
maximum-depth stack-safety, symbol collision, deterministic repeated lowering, and compile-fail
raw/verified/backend boundaries keep every M1 native MIR fixture unchanged. The focused crate gate
currently contains 31 unit tests and 5 compile-fail doctests.

The separate [M2 native backend](M2_NATIVE_BACKEND.md) now provides audited Linux x86-64 object
emission, Boolean wrappers, internal calls, and typed link/run evidence. The explicit profile and
[manifest v2](M2_MANIFEST_V2.md) are implemented; three-target conformance and authenticated
website/live closure remain Issues #56 and #57.
