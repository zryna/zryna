# M3 fixed-oracle candidate conformance

Issue #89 verifies the internal #88 candidate route. Public `--profile data-ownership-v1`
remains rejected with `ZRYNA-C3401`; activation belongs to #90. No public capability is activated.

## Authorities and commands

`tests/m3-conformance-v1.json` freezes source hashes, literal scalar observations, invalid
phase/diagnostic records and named layout/resource/fault oracles. Its digest is pinned by
`scripts/check-m3-conformance.mjs`. Expected values are repository constants, never target output.
Mutation self-tests reject removed/substituted evidence, changed or extra fixtures, weakened
commands and CI dependency bypasses.

- `pnpm m3:registry`: registry authentication and hostile mutation self-tests.
- `pnpm m3:quick`: registry, v4 syntax/worker, candidate execution/repeat builds/rejections and
  public-selector rejection; excludes proportional resource suites.
- `pnpm m3:check`: quick evidence plus complete layout, runtime ABI, ownership IR/semantics,
  backend/native MIR and candidate-security suites, including ignored resource tests and named
  fault/resource oracles. A filtered command that executes zero passing tests fails.
- Before merge: `pnpm preflight`, `pnpm m0:check`, `pnpm m1:check`, `pnpm m2:check`, and
  `pnpm m3:check`; required hosted Linux and Windows aggregate checks must also pass.

## Evidence and host obligations

The corpus observes Pair arithmetic and signed wrap, enum matching, fixed arrays, exclusive borrow writes,
Vec clone/push/index, String clone/move/replace/concat cleanup, aggregate clone/move cleanup,
and Shared/Weak clone/downgrade/release plus live/expired upgrades. String and handle cases observe a scalar continuation;
they do not expose owned host values. Injected trace and runtime-ledger oracles separately check
cleanup order, destination retention and released allocation state.

The harness uses the existing test frontend configuration for isolated fixture roots while
retaining source authentication, semantic/IR verification, dispatch, execution and publication.
M0 separately retains mandatory real-workspace architecture checks. Every valid fixture is
rebuilt from fresh analysis twice and compares complete artifact and manifest bytes.
Invalid cases assert exact owning phase, stable diagnostic and empty output before cleanup.

Linux x86-64 executes all three targets together against each literal oracle. Windows executes
JavaScript/core WebAssembly, emits all three deterministic build artifacts, and rejects native
runs without artifacts. Linux-only runtime/publication fault evidence is explicitly unsupported
on Windows. The `m3` aggregate depends on both M3 hosts and complete `m0` and `m2` aggregates;
skipped or failed dependencies reject. Existing M0-M2 registries remain unchanged authorities.
The main branch protection requires `m3` alongside the existing Rust, adapter, M0 and M2
contexts, with strict up-to-date checking and the existing status-provider binding.

## Boundaries

Use bounded synthetic state and deterministic fault injection, never real exhaustion.
Compiler-only resource and fault evidence is not target allocator execution. Required full
suites retain hostile IR, layout, borrowing, shared/weak, bounds, count and resource checks at
their owning phases. Publication injection proves rollback and create-only destination retention.

No Windows/macOS native execution, WASI, Components, ambient filesystem/network imports, FFI,
threads, raw pointers, custom allocators, owned public host ABI, performance guarantees,
fuzzing substitution or production certification is claimed. Issue #90 remains separate.

## Typed trap and cleanup observations

The internal candidate channel carries returned scalars or the five section 11 language traps:
`zryna.trap.bounds-v1`, `zryna.trap.allocation-v1`, `zryna.trap.capacity-v1`,
`zryna.trap.refcount-v1`, and `zryna.trap.utf8-v1`. Host exceptions, machine traps, malformed
frames and process failures cannot substitute for a typed language outcome.

The frozen corpus contains 15 scalar cases, four source/ABI rejections, two bounds failures and
26 injected fault cases. Fault observations compare the entire ordered trace: exact source
module/function/place cleanup identities, recursive value drops, implicit weak releases and
control releases. Nested String aggregates, Vec<String> clone prefixes and Shared<String>
payloads are included. No target supplies the expected value or trace for another target.

Private fault selection accepts defined trap classes and ordinals 1 through 1,048,576. Logical
injection points are shared by all three targets. Additional Linux tests fail actual descriptor,
payload, growth, control and nested clone allocations. Native finalization releases non-owning
SSA descriptors separately and rejects remaining owned allocations or control blocks before
emitting an observation. The standalone ownership runtime retains its sealed 17-symbol ABI;
private candidate observation and descriptor capabilities are audited separately.

Trace collection is enabled only for injected evidence and is bounded to 4,096 encoded words.
Overflow preserves the original language trap internally and rejects the incomplete observation
at execution with `ZRYNA-C3302`. It does not impose a cleanup-event limit on ordinary execution.
Manifest decoding independently enforces typed traps, event vocabulary, identities and size.
A complete trapped observation may publish a complete run bundle; malformed execution or any
publication failure rolls back without a partial bundle.

Native MIR verification authenticates non-cleanup operations against the sealed source and
independently checks each used cleanup plan's exact source-derived roots and action kinds.
Hostile substitutions and omissions fail before code generation; aggregate action counts retain
the exact/first-extra resource boundary. Public profile activation remains Issue #90.
