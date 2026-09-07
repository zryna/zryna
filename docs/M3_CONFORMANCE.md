# M3 fixed-oracle candidate conformance

Issue #89 verifies the internal #88 candidate route. Public `--profile data-ownership-v1`
remains rejected with `ZRYNA-C3401`; activation belongs to #90. No capability is activated. This is incomplete conformance work; #89 remains blocked.

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
they do not expose owned host values or directly measure allocations. Separate runtime fault
and cleanup oracles check destination retention and released allocation state.

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

## Boundaries

Use bounded synthetic state and deterministic fault injection, never real exhaustion.
Compiler-only resource and fault evidence is not target allocator execution. Required full
suites retain hostile IR, layout, borrowing, shared/weak, bounds, count and resource checks at
their owning phases. Publication injection proves rollback and create-only destination retention.

No Windows/macOS native execution, WASI, Components, ambient filesystem/network imports, FFI,
threads, raw pointers, custom allocators, owned public host ABI, performance guarantees,
fuzzing substitution or production certification is claimed. Issue #90 remains separate.

## Closure blocker

The new fixed bounds-trap test deliberately enforces the normative language identity rather
than accepting host/process failure as conformance. Current candidate execution reports
`ZRYNA-R3006` for JavaScript/WebAssembly and `ZRYNA-N4021` for native; it does not retain
`zryna.trap.bounds-v1`. The scalar observation enum has no M3 trap identities. Existing
native status paths use terminal machine traps and the JavaScript/Wasm harnesses have no
M3 typed trap observation channel. Section 11 of the language contract forbids treating these
host failures as language traps.

The same prerequisite gap prevents complete three-target fault-injected logical drop/release
trace evidence (section 13). Existing compiler oracles and native C fault tests cannot substitute
for those target observations. #89 cannot close, the M3 gates must remain red, and #90 must not
activate the profile until the missing prerequisite behavior and complete corpus are implemented.
The registry's host-error records document rollback diagnostics only, not accepted language traps.
