# Current native-provider acceptance inputs

This registry freezes the current 114 source inputs: three M1, fourteen M2 and
ninety-seven M3 sources. It binds the three unchanged conformance registries and
the existing native v4 corpus census. The known invalid borrow operand retains
its original `Y4002` syntax-verifier obligation; source syntax comparison does
not turn it into accepted compiler input.

The 69-root M3 BUILD oracle is the complete unchanged JSON document from PR539
commit `9cd08ce928e9bcc0eb97a3adfa5662114a017603`. All original source hashes are
checked against the current fixtures before use. It independently fixes 33
accepted and 36 rejected roots and all 67 ordered diagnostics. Two current clone-prefix roots have a separate whole-file-pinned owner authority:
`legacy_frontier_tests.rs` authenticates their snapshots, verifies ownership IR,
emits Wasm, and executes clone failures and successful recovery. Their two
accepted BUILD decisions supplement the original oracle; the historical 69-root
document and its decisions remain unchanged. The invalid borrow source remains
excluded from accepted BUILD roots.

The worker lane performs a real admitted handshake before sending source, then
compares the entire correlated native/bootstrap analysis frame: either the
candidate document with ordered byte spans and diagnostics, or the closed
provider rejection with its exact code and message. It retains every original
command and complete output. The raw comparer records only observed owner exit;
it makes no descendant-cleanup claim. The original Rust worker process
containment remains separately required.
This compares raw candidates; original syntax/IR verification remains mandatory
in its separate owning compiler gates.

The CLI lane invokes the actual compiler through native, bootstrap and comparison
selection for all 88 classified roots. Accepted programs must publish real complete
bundles; every manifest and artifact byte is hashed and compared. Rejected roots
must produce the frozen diagnostics and no bundle. Task-owned bundles are retained
outside the checkout before the same stem is reused for the next provider.

```text
node --test tests/native-provider-public/registry.test.mjs tests/native-provider-public/worker.test.mjs
node scripts/run-native-provider-public.mjs --cli /absolute/zryna --node /absolute/node --output /new/external/evidence --lane all
```

Use pinned Node22.22.1 and a clean committed candidate. `--lane worker` and
`--lane cli` execute individual lanes. A failure retains its real captures and
exits nonzero. No compiler acceptance run has been performed merely by adding
these files or running their synthetic hostile transport/registry controls.

The report always lists unexecuted verified-IR, diagnostic-boundary,
runtime/cleanup, ordinary installed no-Node, reproducible-installation and full
Linux/Windows M0–M4 lanes as `not-run`. This runner cannot close #414 or activate
public support. Historical 190/220 and all thirty blocked identities are preserved
as historical evidence; passing new comparisons does not alter those receipts.
JavaScript/WebAssembly target execution requires its separately explicit runtime;
removing the frontend Node dependency does not remove that target-runtime contract.

The current-head workflow is `.github/workflows/native-provider-public.yml`.
It checks out the actual candidate once and runs the same worker/BUILD census on
`ubuntu-24.04` and `windows-2022`, with pinned Rust 1.97.1 and Node 22.22.1.
It separately executes the native worker controls, original corpus verifier,
live frozen diagnostic/resource differential (including its explicit ignored
provider tests), existing source-bound IR/backend controls, and the two ordinary
clone-prefix owner tests. Complete command captures and failure logs are uploaded
on either outcome. These existing representative IR controls do not establish a
complete frozen-corpus IR lane; that lane stays explicitly unexecuted in the
scoped comparison report until its own actual complete execution is available.
Ordinary installed no-Node and reproducibility proof also require their own real
package producer and physical installation executions. Full M0–M4 remains in the
main CI authorities, and a green scoped workflow does not substitute for them.

Current fault successors live in
`ownership_commands/conformance/native_fault_parity.rs`. They run every one of
the original 26 logical fault rows through both frontends and the original
source-bound closure, ownership verifier, ABI preparation and fault executor.
Each provider must independently match the original typed outcome and complete
ordered cleanup trace, then both complete manifests and artifact bytes must
match. This gives 156 actual observations on Linux and 104 on Windows; Windows
native execution retains its separate unsupported-host rejection. Linux also
runs both providers at all 12 existing physical allocation ordinals and checks
the original allocation trap category, cleanup oracle and zero-live-owner
finalizer. The native counterpart invokes the production lexer/parser and
original syntax verifier in process; the actual worker transport and CLI are
qualified separately. These tests create current successor evidence only when
executed successfully; they do not rewrite the historical blocked receipt.

Every selected Rust workflow target must emit one actual successful summary
with a nonzero required census and zero failed or ignored cases.
`verify-rust-log.mjs` rejects Cargo's otherwise successful zero-match filters.
The complete frozen IR integration target requires the explicit
`native-provider-qualification` feature and the actual absolute
`ZRYNA_NATIVE_CLI`; ordinary library suites require no hidden CLI fixture.

The separate current M1 BUILD oracle binds actual explicit-bootstrap full
diagnostics and unchanged original IR/semantic owner files. The original M1
registry specifies diagnostic-code presence, so it never established a count
of one for the gated Boolean source. The original IR owner emits three ordered
`I1006` diagnostics: the Boolean parameter, Boolean result, and Boolean reference
expression at bytes 53–58. All three full diagnostics are now frozen and checked;
the original M1/M2/M3 registries and M2/M3 BUILD oracles remain unchanged.
