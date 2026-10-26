# M7 bounded generics candidate host and evidence matrix

This page describes the source-checkout #416 candidate, selected only by exact
`--profile bounded-generics-v1`. It is a documentation table, not a new support-report
schema or execution receipt. The [public profile reference](M7_PUBLIC_PROFILE.md) owns
command and outcome details; [complete programs](M7_GETTING_STARTED.md) provide reproducible
inputs. The normally integrated [#415 language contract](../spec/language/BOUNDED_GENERICS_OPTION_RESULT_V1.md)
and [ABI boundary](../spec/abi/GENERIC_VALUE_BOUNDARIES_V1.md) remain authoritative.

## Candidate admission

| Surface | Linux x86-64 | Windows x64 |
| --- | --- | --- |
| Native-frontend `build` JavaScript/core Wasm | Admitted without Node | Admitted without Node |
| Native-frontend `build native` or `build all` | Linux ELF relocatable object; no linker required | Same Linux ELF object; no Windows-native object claim |
| JavaScript/core Wasm `run` | Explicit Node.js 22.22.1 | Explicit Node.js 22.22.1; hosted qualification pending |
| Native `run` | Documented captured GNU toolchain/libc/loader contract | Rejected before target execution |
| `run all` | All three target invocations prepared before execution | Rejected by native host admission |
| Bootstrap/compare frontend | Explicit Node.js 22.22.1 and pinned bootstrap | Same frontend requirements; hosted qualification pending |
| Template-only library with zero scalar exports | Build allowed; Run requires a scalar wrapper | Same source rule; hosted qualification pending |
| Standalone package project / installed distribution | Excluded for this selector | Excluded for this selector |
| Components, WASI command/server, foreign-library imports | Separate contracts; excluded for this selector | Separate contracts; excluded for this selector |

Only nongeneric `bool`/`i32` entry signatures cross the host boundary. Source-module template
visibility does not create a host export. Generic values, Option/Result, aggregate ownership
and loans remain internal. No public generic C ABI, JavaScript object bridge, WIT mapping
or serialization is introduced. A backend or a decoded manifest cannot grant source admission.

The versioned generic manifest is `zryna-generic-manifest-v1.json`, format
`zryna.generic-bundle.v1`, with syntax protocol v5. Its source/module/instance fingerprints,
scalar ABI, artifacts and completed outcomes belong to this profile. Build publishes ordinary
artifacts; Run uses separately audited typed invocation carriers and publishes only after
verified cleanup and retained source/provider checks. Five exact controlled source-trap
identities are distinct from engine errors, host signals, timeouts and malformed carriers.

## Qualification required for this candidate

| Evidence | Requirement and current disposition |
| --- | --- |
| Local Linux integration | Finite source, independent authority, target/fault/resource and public-route tests have run during development. Their exact revisions/commands/counts belong to retained receipts; this table is not an all-gates receipt. |
| Canonical M0 | Existing Linux and Windows Rust jobs run the unchanged canonical command set, including `cargo test --locked --workspace`. Actual job conclusions/counts must be retained. |
| Finite public packet | Required: 11 Linux tests; 12 Windows public tests + 3 portable-deep + 3 shared ignored ABI = 18. Exact nonzero names and summaries must match the checked registry; these numbers are obligations, not observed passes. |
| Deep ownership/faults | Existing Linux same-seal JS/Wasm/native gates and separate Windows-only portable JS/Wasm extraction. Depths 0/1/8192/65536 are finite witnesses, not an additional source quota or a Windows native-run claim. |
| Resource boundaries | Normal workspace owner tests plus shared ABI ignored controls: Linux preflight already includes them; Windows packet explicitly executes three. Exact/first-extra/overflow/retry evidence stays separate from source/runtime fault observations. |
| Hosted Windows | Pending for the current candidate. No Linux result, test listing, cfg-disabled target or zero-test summary substitutes for actual Windows execution. |
| Final integration | Both platform Rust receipts, Linux preflight, final canonical M0 aggregate and all branch-required checks must succeed for the tested source revision/tree. Pending, failed, ignored, cancelled and unrun checks remain explicit. |

The six complete ordinary JavaScript String/Vec runtime frontier tests belong to
`cargo test --locked -p zryna-driver --test generic_execution_v1_js_accounting`.
The unchanged M0 `cargo test --locked --workspace` discovers this integration target on
both Linux and Windows: all six run once, without ignored tests or feature/platform guards.
They retain the original authenticated sources, both provider packets, scripts, oracles,
and one shared process owner with its 60-second child deadline and 64 KiB streams.
The M2 JavaScript backend package keeps its backend boundary/unit controls; these M7 runtime
frontiers are exercised by the driver workspace gate without an extra finite-packet replay.

The [conformance record](M7_GENERIC_CONFORMANCE.md#current-416-candidate-evidence) explains
how the finite packet reuses existing full-suite coverage. Its
[registry](../tests/m7-generic-acceptance-v1.json) fixes commands, test names, host selection
and pinned tools. Existing source limits, ABI17, core-Wasm memory, compiler work budgets,
child deadlines and cleanup bounds are unchanged; this integration introduces no CPU fuel
or general source termination guarantee.

CI preserves the existing GitHub merge checkout. Receipts bind actual clean HEAD and tree
before/after the commands to `github.sha`, with the PR-head SHA recorded separately. Required
hosted logs and conclusions must corroborate supplied workflow metadata. Before merge, the
integration owner must compare the tested merge tree with the current PR-head tree and verify
the branch is up to date with main. Neither a prior checkpoint nor a different tree qualifies
this candidate.

## Preserved historical contracts

The [v0.1.0 preview matrix](DEVELOPER_PREVIEW.md), [fixed M4 support registry](M4_SUPPORT.md)
and existing `zryna support --json` retain their contract, row inventory, support-report schema and scope.
Operational source descriptors and their derived support digests may be refreshed only after
explicit source review through the existing M4 registry/projection workflow; that provenance
refresh is not execution qualification. This separate page does not add a support-report
version, update immutable distributions, change
#415 fixture digests, or authenticate a website/release publication. The
[private continuation history](M7_GENERIC_CONTINUATION.md) remains a record of its original
checkpoints. Current #416 completion and all-platform qualification are not asserted.
