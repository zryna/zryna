# M2 three-target conformance

This document defines the executable closure gate for the explicit `control-flow-v1` profile. It
does not change the default M1 profile, add syntax, or make a backend authoritative for expected
behavior. The authenticated registry at `tests/m2-conformance-v1.json` is the fixed external
oracle.

## Fixed oracle

The registry is canonical JSON with a compiler-owned SHA-256 constant. It separately authenticates
the immutable historical M2 planning registry, every `.zry` fixture, the canonical source graph,
and the expected JavaScript, WebAssembly, and native object bytes. Validation fails when a case,
target, source, edge, artifact, diagnostic, resource row, order, or unknown field drifts.

The valid corpus contains the historical arithmetic, comparison, Boolean, local, assignment,
direct-call, branch, loop, and module cases plus a noncommutative positional argument-order case.
Expected results are typed fixed values. They are never learned from one backend or selected by
majority agreement.

Linux executes every valid case against, in order:

1. the deterministic ECMAScript module;
2. the validated import-free core WebAssembly module;
3. the audited Linux x86-64 native object after driver-owned link and typed execution.

All three observations must equal the fixed typed result. Windows executes the complete portable
JavaScript and WebAssembly corpus. A Windows request for `native` or `all` must fail with
`ZRYNA-N4002` and publish no bundle.

## Rejection and boundary evidence

The public invalid corpus freezes target-independent ordered diagnostic codes for numeric
conditions, assignment to `const`, use before declaration, recursive calls, bare imports, path
escape, missing modules, portable case collisions, and import cycles. Every target selection must
return the same diagnostics and leave no final bundle.

Bare imports are rejected by the syntax provider before a verified import DTO exists. The public
compiler therefore reports the stable provider boundary `ZRYNA-F1103`; it does not misclassify a
generic provider rejection as module resolution.

Internal evidence remains mandatory for claims that cannot be represented by accepted source:

- the Universal IR verifier rejects irreducible control flow with `ZRYNA-I2020`;
- every pipeline phase failure is atomic and publishes no final bundle;
- source replacement and portable path races fail closed;
- all 37 historical resource limits are bound one-for-one to their numeric value, an executing
  no-shell command, and an exact/+1 test selector.

## Determinism and provenance

Repeated same-stem builds compare complete manifest and artifact bytes. The manifest binds the
profile, graph identity, ordered sources and edges, selected targets, artifact digests, results,
and diagnostics. Native linked executable bytes are reproducible only within the documented pinned
toolchain and host boundary; the compiler-owned native object is the cross-run artifact oracle.

The successful create-only directory rename remains the only publication commit point.

## Local and CI gates

Use the smallest gate that covers the change:

```text
pnpm m2:registry  # canonical registry, fixtures, provenance, and structural drift
pnpm m2:quick     # ordered focused boundary suites without a shell
pnpm m2:check     # full public corpus followed by all internal evidence
```

`m2:check` uses bounded output, explicit timeouts, and frozen `node`/`cargo` argument vectors with
`shell: false`. It never invokes a platform command shim through a shell. GitHub Actions runs the
same gate on `ubuntu-latest` and `windows-latest` after preflight. The stable aggregate `m2` context
passes only when both platform jobs and the retained `m0` aggregate pass.

This closes Issue #56's executable conformance gate. M2 milestone closure remains separate: Issue
#57 must export authenticated compiler documentation, synchronize the website, deploy the exact
bundle, and verify the live commit and digest.
