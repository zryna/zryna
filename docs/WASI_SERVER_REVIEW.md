# Bounded WASI server review and integration record

This change introduces #401's bounded public source-checkout server support upon reviewed
integration after #400. The owning [PR #548](https://github.com/zryna/zryna/pull/548) must pass
fresh exact-revision local and supported-host checks before integration and ticket closure.
This record alone does not close #401 or the whole-M4 publication issue #402.

## Accepted scope and review

The owner approved the finite loopback status-only slice and delegated the separately approved
one-read clock/status follow-up. The [status contract](WASI_SERVER_STATUS_V1.md) and
[clock contract](WASI_SERVER_CLOCK_STATUS_V1.md) retain those separate profiles and approvals.
Both admit one portable lowercase `.zry` file and one no-argument i32 status export. The status
is 200–599 and the body is empty. Clock/status performs one actual guest monotonic-clock read
per admitted request, discards its timestamp and introduces no source clock intrinsic.

On 2026-10-11, four independent agent reviews covered all 95 server-dependent paths at
`2dfee16257bf9c0812bb3d47c49e724637ddd0a7`, tree
`8c84e38cb1ddce689f21114f0b5c7e03d8247df3`, against the original command candidate
`eff2bdad6bd1b4800391870a725f1fad9e45e5e6`: 33 backend/runtime, 28 compiler/CLI,
17 transport and 17 documentation/example paths. These are source reviews, not GitHub approvals.

The compiler review found that source selection lacked the documented lowercase `.zry` check.
Two new actual CLI regressions first demonstrated that valid status source in a `.txt` file
started a listener and reached the service deadline under both profiles. Selection now rejects
that input before workspace compilation, listener readiness or result publication. The corrected
regressions each execute `.txt`, `.js`, `.ZRY` and extensionless cases with otherwise valid
source and approvals. Existing positive HTTP status, guest-read and observed-teardown tests
remain part of the same suite. These focused working-tree checks do not qualify the final revision.

The follow-up normally merges the completed #400 main
`3ac602b8b5ce862b48010e73fcfe552bc651b4ef` into the preserved server history. Its source tree
contains the accepted command scalar-preparation and vector-clone corrections. Conflict
reconciliation uses the exact common content baseline above; shared documentation preserves
the integrated command contract and introduces server support separately. No old passing
receipt qualifies this combined revision.

## Acceptance and required execution

| Obligation | Owning tests and proof |
| --- | --- |
| Startup, bounded requests, repeated start and deterministic shutdown | `apps/zryna/tests/wasi_server.rs`, `wasi_server/source_tests.rs`, driver `server_transport` and `server_lifecycle` tests |
| Malformed requests and exact/first-extra header and body bounds | Actual CLI wire requests and driver transport framing/boundary tests |
| Guest denial, one granted clock read, private-input bounds and revocation | `wasi_server/clock_tests.rs`, driver `server_runtime` admission, execution and capability probes |
| Cancellation, request/service deadlines and resource exhaustion | Actual CLI cancellation/deadline cases and driver runtime/lifecycle tests |
| Store, resource and listener cleanup before publication | Observed teardown assertions in runtime/lifecycle/transport and actual CLI result records |
| Forced host termination | Actual CLI kill cases prove OS listener/private-handle release and absent graceful result |
| Command and browser preservation | Actual command CLI/backend tests, full M0/M2/M3 and three pinned browser/transport proofs per supported host |
| Component/world identity and malformed artifacts | Backend `component_server` audits and source-bound driver composition tests |

Final validation retains frozen dependency installation, locked Cargo fetch, canonical build,
full preflight and M0/M2/M3, server CLI/runtime/lifecycle/transport, backend, command and
documentation checks. The pinned browser runner executes two real-browser agreement proofs
and one independent full-ceiling transport proof. Fresh installed-command qualification preserves
the completed command surface; it does not add installed-server support. Required Linux x64
and Windows x64 results, actual test outcomes, exact source/tree identities and receipt hashes
are recorded with PR #548 before integration. Ignored, skipped, failed and unrun outcomes stay
separate from executed passes.

The preserved server candidate's original local M0 F1108 worker-cleanup failure and full-M3
unchanged-600-second timeout remain failed historical runs. Later focused cleanup probes and
hosted passes do not erase them. Fresh combined-revision gates are required.

## Support and authority limits

Support is limited to the documented source-checkout profiles on the qualified Linux x64 and
Windows x64 hosts. Listener authority requires owner-private configuration and a separate
configuration-bound approval for literal `127.0.0.1`. Serial requests, finite byte/time/attempt
budgets, fixed fuel/memory and separate guest approval remain enforced. Status-only has empty
guest grants; clock/status admits only its separately approved one-read arrangement.

Successful result publication is create-only and follows joined, observed Store/resource/listener
teardown and the final authority check. Revocation suppresses the record. Forced termination
proves OS authority release, supplies no graceful Store-cleanup certificate and may retain a
private uncommitted transaction directory.

Installed server execution, immutable beta server support, public internet service, signed release
and production admission remain outside this contract. The fixed whole-M4 conformance registry,
all-profile support matrix and actual website/live publication remain the separate #402 acceptance.
