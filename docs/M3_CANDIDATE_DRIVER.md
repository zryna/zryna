# M3 DataOwnershipV1 candidate driver

Status: implemented as an internal candidate boundary. It does not activate a public profile.
The public CLI continues to accept only the default M1 profile and exact
`--profile control-flow-v1`; exact `data-ownership-v1` is rejected before workspace effects with
`ZRYNA-C3401`.

## One authenticated route

An internal build or run request fixes an absolute workspace root, one portable protocol-v4 entry
module, a portable artifact stem, an exact target selection, and an absolute direct Node.js
runtime. A run additionally fixes one logical export and an ordered typed scalar argument vector.
The driver then:

1. validates the request and workspace before source or target work;
2. discovers the bounded protocol-v4 module closure and authenticates one final source map,
   snapshot, source order, edge order, and graph digest;
3. lowers that closure once to one verifier-sealed `DataOwnershipV1` program retaining both layout
   authorities and ownership-runtime ABI v1;
4. prepares selected artifacts in fixed JavaScript, WebAssembly, native order;
5. for a run, validates the invocation once and executes every selected target inside the private
   publication transaction; and
6. writes and audits one manifest before a single create-only directory commit.

Backends cannot substitute source, layout, ABI, invocation, or diagnostic authority. Unsupported
targets, platforms, features, runtimes, toolchains, or malformed authorities fail closed.

## Manifest v3

Every candidate bundle contains exactly `zryna-manifest-v3.json`. Its canonical top-level field
order is `version`, `profile`, `protocol_version`, `command`, `entrypoint`, `graph_sha256`,
`sources`, `edges`, `layouts`, `runtime_abi`, `stem`, `targets`, `artifacts`, `invocation`,
`results`, and `diagnostics`. The fixed identities are manifest version `3`, protocol version `4`,
and profile `zryna-data-ownership-v1-candidate`.

The manifest binds:

- the canonical entrypoint, path-ordered source identities, named import edges, and graph digest;
- the type-universe, linear32, and Linux x86-64 layout fingerprints;
- the ownership-runtime ABI identifier, version, and identity digest;
- selected targets in JavaScript, WebAssembly, native order;
- every exact artifact filename, size, SHA-256 digest, kind, scalar ABI, and target metadata;
- Linux target triple and object/runtime/harness identities where applicable;
- the optional typed invocation, ordered typed outcomes, and stable provider diagnostics.

The strict decoder accepts only canonical pretty JSON with one trailing newline and a maximum size
of 32 MiB. It rejects malformed data, unknown, missing, duplicate, or reordered fields,
non-canonical bytes, inconsistent graph or layout identities, invalid target order, mismatched
artifact metadata, and incompatible invocation or result records.

## Atomic publication and failure behavior

Selected target files are written beneath one new private transaction directory. A build stages
`.mjs`, `.wasm`, and/or the audited `.o`; a run stages `.mjs`, `.wasm`, and/or the audited `.elf`.
Run harnesses and native execution copies stay private and are removed before inventory audit.
The driver verifies bytes, checksums, modes, paths, results, and the canonical manifest, then
publishes the whole `<stem>.build` or `<stem>.run` directory with one same-filesystem create-only
rename. Existing destinations are never replaced.

Source, backend, malformed execution, staging, manifest, audit, cleanup, or commit failure
exposes no final bundle. A typed language trap is a complete run observation and may publish
a complete bundle with its exact trap identity and optional bounded logical cleanup trace. Ordinary rollback removes the private transaction. Process or machine termination may
leave an unadvertised private transaction directory; it cannot be mistaken for a committed
bundle.

## Executable evidence

Focused repository tests cover:

- `ownership_pipeline::tests` for authenticated multi-module closure, fixed dispatch, invocation
  validation, cycle and resource rejection;
- `ownership_manifest::tests` for deterministic replay, strict decoding, checksums, target
  metadata, hostile bytes, and the exact manifest-size boundary;
- `ownership_publication::tests` for actual all-target bundles, create-only collisions, rollback,
  injected phase failures, modes, and exact inventory;
- `ownership_commands::tests::all_targets_execute_and_publish_one_typed_candidate_run` for actual Linux
  x86-64 JavaScript, WebAssembly, and native execution from one two-module candidate request; and
- `data_ownership_candidate_profile_is_rejected_before_workspace_effects` for both public CLI
  profile spellings, deterministic `ZRYNA-C3401`, and no output creation.

## Retained exclusions

This candidate is not public CLI availability or general aggregate ABI. The
[fixed-oracle gates](M3_CONFORMANCE.md) exercise this candidate under Issue #89; typed trap and fixed cleanup evidence are
documented with the corpus and resource boundaries. Public profile activation, authenticated
website material, and release provenance remain Issue #90. Windows or macOS native execution,
WASI, Components, FFI, threads, raw pointers, custom allocators, and freestanding targets remain
unsupported.
