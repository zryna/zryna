# M3 ownership runtime ABI authority

Status: implemented internal declaration verifier for ownership-runtime ABI v1. This boundary is not
a runtime implementation and is not reachable from the public driver or CLI.

## Authority boundary

The `zryna-ownership-runtime-abi` compiler component accepts untrusted ownership-runtime declarations
only alongside owned, verified `Linear32V1` and `LinuxX8664V1` layout authorities. It verifies the exact
identifier `zryna-ownership-runtime-v1`, version, operation set, target mappings, carrier and result
shapes, layout identities and fingerprints, checked C-header evidence, and pure logical transition
evidence before exposing opaque immutable views. Raw declaration records are not returned on
success.

The declaration set contains exactly 17 logical operations:

- three raw-storage operations: allocate, grow, and release;
- four owned-String operations: UTF-8 copy, clone, concatenate, and release;
- three owned-Vec storage operations: allocate, reserve, and release; and
- seven Shared/Weak transitions: strong clone, weak downgrade, weak clone, weak upgrade,
  strong-release begin, strong-release finish, and weak release.

Operation order, symbols, signatures, status encodings, out-record shapes, target word carriers, and
the header's declaration bytes are verifier-owned evidence. A declaration cannot substitute a
host-sized Rust type, reorder an out record, add a symbol, or bind a layout from another source map,
type universe, target, or fingerprint.

## Layout and Scalar ABI separation

The authority consumes sealed aggregate layouts; it does not recompute Rust, C, or host layouts.
`Linear32V1` and `LinuxX8664V1` records remain distinct and retain the authenticated source-map and
type-universe identities supplied by `zryna-layout`. Scalar ABI v1 remains unchanged and continues
to govern only the existing public scalar boundary. Ownership-runtime handles and control records
do not become public aggregate parameters or results.

The checked header is declaration evidence for later native work. It is not a stable public C
library API, a compiled object, a linked library, or proof that any symbol has an implementation.

## Pure transition evidence

The pure state validator covers Vec allocation and reserve plus all 12 canonical Shared/Weak control
transition cases, including count overflow, expired weak upgrade, both halves of last-strong release,
retention of explicit Weak handles, final deallocation, and illegal finish without a pending
last-strong transition. Successful transitions, controlled failures, and ABI violations must preserve
the exact v1 prepare-before-commit rules.

Raw storage and owned-String behavior are sealed as exact operation, status, result, and target
declaration rules; this issue does not execute allocator or String state models. Verification executes
no allocation, release, reference-count mutation, cleanup, target helper, source callback, or host
operation.

## Determinism, diagnostics, and limits

Malformed declarations fail closed with stable `ZRYNA-R3xxx` diagnostics. `ZRYNA-R3001` covers ABI
identity, version, inventory, operation, and symbol mismatches; `ZRYNA-R3002` covers invalid carriers,
signatures, results, records, and transitions; `ZRYNA-R3003` covers layout target or fingerprint
mismatches; and resource exhaustion uses `ZRYNA-R3201`. Diagnostics are deterministically ordered and
retained under the 256-violation cap.

Declaration verification admits at most 256 operation records, 4,096 target declarations aggregated
across JavaScript, WebAssembly, and native, 65,536 record declarations, 65,536 nested declaration
items, and 16 MiB of checked header bytes. Exact-limit and first-extra cases are tested, and failure
returns no partial verified authority. The normative relocation/call-edge and runtime object/module
byte limits remain reserved for later artifact auditors; this declaration-only verifier neither
accepts nor claims to enforce those artifact inputs.

Use `pnpm m3:runtime-abi:quick` for 17 focused unit tests plus two compile-fail opacity doctests; three
proportional record, nested-item, and checked-header boundary tests remain ignored in that quick lane.
The full repository preflight includes all 20 unit tests plus both compile-fail doctests.

## Deliberately unavailable

This component supplies no allocator, runtime implementation, target helper body, backend lowering,
runtime object or module, linked artifact, driver route, manifest, CLI selector, host execution, or
public aggregate ABI. It does not activate `data-ownership-v1` and does not widen or reinterpret M1,
M2, scalar ABI v1, `I32V1`, or `ControlFlowV1`.

The normative operation and transition rules remain in
[`OWNERSHIP_RUNTIME_V1.md`](../spec/abi/OWNERSHIP_RUNTIME_V1.md). Later issues must independently
implement, audit, compose, and execute target runtimes against these sealed declarations.
