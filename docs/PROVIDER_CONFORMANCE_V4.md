# Provider conformance v4

Status: frozen provider-neutral syntax corpus. This suite compares protocol-v4 providers; it does
not select a provider for the public CLI or implement a native frontend.

## Authority and compatibility

`tests/provider-conformance-v4.json` is the machine-readable authority. It pins the canonical
request, source, and snapshot bytes; the exact case order; the protocol and capability tuple; and
the current bootstrap authority (`typescript-6` at `6.0.3`). The registry digest is frozen in
`scripts/check-provider-conformance-v4.mjs`.

The TypeScript 6 worker remains authoritative while it is the bootstrap provider. Running the
suite without arguments therefore requires its exact handshake identity and version. A future
provider runs the same cases with:

```sh
node scripts/check-provider-conformance-v4.mjs -- <executable> [arguments...]
```

An alternate provider may report its own bounded identity and version. It must still advertise
protocol `4` and exactly the syntax-only capability tuple. No provider-specific expected snapshot,
diagnostic code, fixture ordering, budget override, or source rewrite is permitted.

## Corpus and comparison

The six sessions cover:

- a successful one-file snapshot;
- malformed duplicate request fields followed by a successful handshake;
- unsupported syntax and its stable `ZRYNA-F2002` category;
- the first request beyond the fixed JSON-depth budget and `ZRYNA-F1002`;
- a provider error followed by a successful analysis in the same process; and
- reverse-ordered multi-file input with canonical path/FileId output ordering.

Every session runs twice in a fresh provider process. Complete stdout bytes must repeat exactly,
so response order and diagnostic text cannot drift between identical runs. Successful responses
must equal the frozen JSON snapshots structurally. Diagnostics compare the stable code across
providers while also requiring each provider's message to repeat exactly within its two runs.

For successful snapshots, every output FileId/path pair must match the canonical sorted source
map. Every span must select a valid UTF-8 boundary inside that exact source, and every carried
identifier or module-specifier value is checked against its source bytes. The snapshot schema
remains an additional closed structural boundary. A focused Rust integration test decodes both
goldens and requires the authoritative protocol-v4 verifier to bind them to those exact source
maps; changed source bytes reject. The resulting opaque verified view remains the compiler
authority for admission.

## Registry closure

The registry and fixture tree fail closed. Validation rejects a changed registry digest, unknown
fields, missing or extra fixture files, symlinks, duplicate IDs, portable case-colliding paths,
reordered artifacts or cases, unreferenced material, invalid references, and mismatched fixture
digests. Tests independently exercise missing, extra, reordered, case-colliding, and tampered
cases rather than relying only on the frozen registry hash.

Run `pnpm provider:conformance:v4`. The dedicated workflow runs the same command on Linux and
Windows. Existing syntax quick, documentation, preflight, and M0 gates remain unchanged and
mandatory before integration.

## Boundary

This suite grants no name or module resolution, semantic diagnostics, type or ownership checking,
layout, IR, backend, runtime, public provider selection, or native frontend capability. Adding a
provider requires an independent implementation of the frozen wire contract; it does not permit a
special corpus mode or replacement of the bootstrap adapter.
