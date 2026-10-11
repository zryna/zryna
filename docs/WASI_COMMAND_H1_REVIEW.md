# Bounded WASI command review and integration record

This follow-up introduces #400's bounded public activation upon integration. Delta review,
exact-revision gates and maintainer integration acceptance govern that integration. It does not close
#400, #401 or #402 merely by being present in the repository.

## Historical decisions and independent baseline review

The owner accepted five bounded H1 decisions on 2026-10-09, as recorded in the
[contract](WASI_COMMAND_H1_CONTRACT_DRAFT.md#recorded-bounded-owner-acceptance). Implementation
preceded that record; later acceptance does not erase the historical prerequisite gap.

On 2026-10-11, four independent agent reviewers examined the complete cumulative
[PR #513](https://github.com/zryna/zryna/pull/513) candidate
`eff2bdad6bd1b4800391870a725f1fad9e45e5e6`, tree
`16231bbcc635fedc59cfb3cd3477e1891b8e9b5c`, against main
`d6f8a61406fd3f0353ffd36725042b1cf43a6d21`. Coverage was closed over all 315 changed paths:
86 backend, 80 runtime/CLI, 60 compiler and 89 documentation/distribution/fixture paths.
Review used exact candidate contents and cumulative diffs, including the original compiler
extractions. These were agent reviews, not GitHub approvals or a maintainer integration decision.

The baseline review kept an existing vector-growth allocation concern unconfirmed,
identified missing public activation documentation, and recorded nonblocking test-hardening
observations. Subsequent ordinary source diagnostics confirmed two defects: an indexed scalar
operand triggered an internal preparation assertion, and appending to a cloned vector could
change a separately owned neighboring vector. These findings supplement the baseline review.

Scalar preparation now obtains the child's typed result within its own preparation range,
preserving the result assertion and trailing indexed cleanup. Vector clones now record capacity
equal to the initialized length actually allocated; later appends grow storage when required.
The separately authored instruction reference requires the matching capacity store. Extracting
the vector helper keeps both production modules within the existing size limit.

Working-tree regression evidence recorded both failures before correction. After correction,
the four-case scalar regression, 20 existing scalar tests, eight command-semantic tests and the
real command-core vector-growth regression passed. A separate reviewer checked the vector
delta, its preserved extraction, instruction reference and CLI case; the extraction's initial
module-path compile error was corrected and its failed receipt preserved. These focused checks
do not replace final exact-revision gates.

## Baseline execution evidence and limits

All 37 hosted checks for the exact baseline `eff2bdad6bd1b4800391870a725f1fad9e45e5e6`
concluded successfully. This records the baseline, not successful checks for later edits.

The installed H1 workflow [run 37989652166](https://github.com/zryna/zryna/actions/runs/37989652166)
ran 29 actual relocated invocations on each host:
[Ubuntu 24.04](https://github.com/zryna/zryna/actions/runs/37989652166/job/114020260986) and
[Windows Server 2022](https://github.com/zryna/zryna/actions/runs/37989652166/job/114020261174).
Its uploaded closed four-file receipt inventories bind the same baseline head and tree.
Downloaded receipt ZIPs were checked against GitHub artifact digests:

| Host | Artifact | ZIP SHA-256 |
| --- | --- | --- |
| Ubuntu 24.04 | 11645146836 | `250ece344e2ac5c80e48ce4212c35913493ed697f86c52a80fae591bf6ff6c31` |
| Windows Server 2022 | 11644698484 | `4a37f8bb6db414dba098f62c6c8468bd76d586fdf5647c7d879bcfc15ee38a9d` |

Positive, missing, malformed, mode-rejection, existing-output and installation-tamper cases
had the expected recorded exit phases. These receipts establish actual candidate content
verification and archive round-trip observations. Complete large archives were not downloaded
for this review. No signature authentication, release-recipe execution, archive reproducibility
or production admission was established; receipt `productionAdmission` remains `forbidden`.

## Contract coverage

The [contract's fifteen required rows](WASI_COMMAND_H1_CONTRACT_DRAFT.md#fixed-design-and-later-execution-fixtures)
remain separate obligations. Their owning test locations are listed below; this inventory
alone is not an execution certificate. Final checks must run on the revised candidate.

| Required row | Owning test evidence |
| --- | --- |
| Pure | `apps/zryna/tests/wasi_command.rs`; driver `command_h1_runtime/tests/execution.rs`; backend `command_h1_v1/component_tests.rs` asserts zero callbacks |
| Found | CLI fixed examples; driver `tests/execution.rs` and `tests/consume.rs`; backend `language_tests.rs` checks conversion independently of the public Boolean result |
| Missing | CLI and driver execution tests distinguish declared WIT `err` from denial |
| Present-empty | CLI and driver execution tests keep input `present`, distinct from Missing |
| Omitted grant | Driver `tests/preparation.rs` rejects before engine creation; CLI publishes no bundle |
| Changed authority | Backend artifact tests, driver `profile_composition/command_h1/tests/rejection.rs` and `command_request/capture/tests/` retain issuer, source, world and file authority |
| Malformed | Driver `command_request/tests/rejection.rs` closes every record and checks deterministic recovery |
| Exact and first-extra bounds | Driver request `tests/bounds.rs`; `tests/wit-capability-contract.test.mjs` independently retains registry 128/129 entries and 65,536/65,537 bytes |
| Revoked | Driver host/preparation tests and manifest tests require no copied value, absent run return and authentic denial |
| Static limits | Driver preparation/request/composition tests bind the one-key reservation separately from value presence or repeated reads |
| Fatal categories | Driver session/execution/deadline tests; backend interface tests bind identities to actual component/core/instruction coordinates |
| Manifest | Driver `command_h1_runtime/manifest/tests.rs` and `tests/rejection.rs` check closed fields, outcomes and privacy; decoded JSON confers no execution authority |
| Denied probes | Driver `tests/denied_probes.rs` executes independent typed probes against production callbacks under empty and H1 policies; backend process-import audit rejects outside-world imports |
| Malformed component | Backend component/environment/storage independent mutation tests reject wrong topology, types, canonical options and instructions |
| Cleanup | Backend language cleanup/frontier/storage resource tests; driver deadline/denial tests verify Store consumption, owned watchdog join and fresh recovery |

Denial probes cannot construct a sealed production artifact or manifest. Ordinary installed
positive runs do not replace malformed-component, denied-callback or cleanup boundary tests.
Protocol-v4 fixture snapshots were inspected against their source spans and token text; provider
regeneration was not performed during the documentation/distribution review.

## Support introduced upon integration

The change admits one source file and sole `main(): bool`, empty grants or one literal-key
owner-private environment request, fixed pinned WIT/runtime limits, and create-only execution
records. The [walkthrough](WASI_COMMAND_GETTING_STARTED.md) gives checkout and newly verified
complete-installation commands. Evidence is limited to supervised Linux x86-64 and hosted
Ubuntu 24.04 x86-64 / Windows Server 2022 x64. Existing immutable beta archives remain excluded.
There is no new grant category, dependency-import route, source ABI, Windows desktop,
performance, general plain-cloud, production or signed-release claim.

Follow-up scalar preparation, backend regression fixtures and activation documentation are a
new delta. Their final review and required exact-revision checks are recorded with the pull
request before integration and #400 closure; baseline receipts do not qualify later edits.
Independent review does not replace the maintainer's integration
acceptance. Server acceptance remains #401; the fixed whole-M4 registry, all-profile support
matrix, website and live publication verification remain #402.
