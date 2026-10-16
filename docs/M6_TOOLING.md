# M6 tooling support and evidence

Current reviewed-source support is determined by the independently selected, immutable
`zryna.m6-source-evidence.v2` final record and its actual receipts, authenticated compiler
publication and deployed documentation. They must identify the same integration commit/tree,
artifact versions, composition and documentation manifest. See [final acceptance](#final-acceptance-record)
and the [record verification contract](../spec/tooling/M6_REVIEWED_SOURCE_V2.md#detached-v2-record-validation).
Treat an integration as under verification when that complete matching evidence is unavailable.

This source page defines the bounded tooling surface and its acceptance conditions; it does
not record completed gates or activate debugging or additional query forms. The separate
standalone toolkit v1 retains its own publication and acceptance requirements.

The [M6 conformance contract v1](../spec/tooling/M6_CONFORMANCE_V1.md) defines the common evidence
identity and complete acceptance matrix. The separate [reviewed-source route v2](../spec/tooling/M6_REVIEWED_SOURCE_V2.md)
preserves those obligations while specifying its own source, publication and daemon authority. Existing [architecture](ARCHITECTURE.md),
[standalone projects](STANDALONE_PROJECTS.md), [language-server and editor contracts](LANGUAGE_SERVER.md),
and [portable setup](PORTABLE_SETUP.md) retain their own compatibility and release status.

## Version and support matrix

Component versions identify different products; they are not interchangeable release numbers.

| Surface | Exact version or profile | Documented behavior | Status and evidence boundary |
| --- | --- | --- | --- |
| Standalone compiler/project | Compiler 0.2.3; generated `i32-v1` project | `new`, frozen source authentication, create-only JavaScript/core WebAssembly build and run on Linux/Windows | Existing project contract; final M6 integration must rerun its applicable project and portability gates. |
| Language server | Server 0.5.0; scalar protocol v2, explicit `control-flow-v1` v3 or `data-ownership-v1` v4 | Bounded stdio diagnostics, full-text revisions, negotiated coordinates and formatting; scalar definition only | Existing LSP contract; no build or execution method. Final support requires matching LSP gates in the selected integration record. |
| Formatter | `scalar-format-v1`, `control-flow-format-v1`, `data-ownership-format-v1` | Compiler-admitted document/range edits, token/comment preservation, idempotence and revision checks | Delivered through the matching LSP/editor. There is no `zryna fmt` CLI command. Final support requires matching formatter gates in the selected integration record. |
| Editor extension | VSIX 0.5.0; VS Code-compatible API >=1.82.0 | Diagnostics, scalar definition, M1–M3 formatting; explicit saved-file Run for scalar/M2 JavaScript or core WebAssembly | Separately published extension with historical installed-host evidence. Publication does not authenticate an outer setup or close the new integration. |
| Portable setup | 0.1.0-candidate.3; compiler 0.2.3, server/VSIX 0.5.0 | Reviewed complete installation, isolated editor profile, explicit compiler/editor exercises and relocation | Internal review/user-acceptance candidate. No public outer publisher signature; production admission remains forbidden. |
| Fixed-component example | Existing `browser-component-v1` bundle | Local execution of fixed `examples/universal/add.zry` with editable scalar inputs | Independently documented [example](../examples/playground/README.md). It does not compile edited source or establish source-playground acceptance. |
| Reviewed-source playground v2 | `reviewed-source-v1` composition; playground compiler 0.1.0; `restricted-browser-v1` transport; `browser-component-v1` output | Operator-reviewed source launcher API, dedicated Docker compiler/browser, one-file compilation and scalar evaluation | Public support requires the authenticated published compiler, independently approved matching composition and complete final integration evidence. A source-only qualification does not establish public admission. |
| Standalone toolkit v1 | Proposed 0.1.0, proposed `playground-v0.1.0` immutable prerelease | Separate authenticated compiler/material/binding/WIT/browser/policy composition | Not published. Outer publisher authentication, durable artifacts and final integration receipts are pending. |

The extension requires the exact selected analysis/formatting capability and matching server
source identity before transmitting source. An immutable compiler release's older server is not
made compatible by the compiler version number. M3 formatting uses the trusted saved-import root;
M3 editor Run, M2/M3 definition, completion, rename and debugging remain unavailable. See the
[editor compatibility table](LANGUAGE_SERVER.md#editor-installation-and-compatibility).

Standalone `--project-root` admission retains its documented default `i32-v1` restrictions;
the playground does not add component profiles to generated standalone packages. Existing
compiler M2/M3 capabilities retain their separate profiles and gates. No milestone or profile
is promoted by success of this restricted tooling slice.

Portable packaging targets remain Windows x64 with the Windows Server 2022 baseline and
Ubuntu 24.04 x64. macOS, other architectures, remote/virtual workspaces and native Windows
program output are outside this portable workflow. Packaging/transport support alone does not
establish a native graphical editor or restricted execution-host acceptance result.

## Preserve historical evidence

The [publication and installed-host evidence](LANGUAGE_SERVER.md#publication-and-installed-host-evidence)
records the reviewed 0.5.0 VSIX and its exact hash, registry installations, and nine real Windows
extension-host checks for each installed package. Those tests used test source
`2f2f64e723924cd329a987e7ac9833149fd0811b` and setup source
`bf8706ab4a8a606ec99d816e8df90d1c4c1f0f31`.

The same record identifies 33 historical hosted checks at
`3fadcbd3d0112dd8cd79bfaa10bd334a18594acc`. Those checks establish the recorded setup revision's
Linux/Windows reproduction and portable acceptance. They do not establish Linux graphical
extension-host acceptance, current playground isolation or a pass at the final M6 integration.

The [portable candidate verification rules](PORTABLE_SETUP.md#verify-before-running-anything)
remain in force. Authentication of its nested compiler does not authenticate the new outer
setup, server or VSIX. Preserve original artifact versions, source revisions, hashes and status.
If the final integration intentionally reuses an immutable artifact, identify those exact bytes
and rerun the applicable integration exercise; never replace a published version with different bytes.

## Existing runnable project/editor workflow

For a reviewer-verified portable candidate, select the absolute installed compiler executable
as described in [portable setup](PORTABLE_SETUP.md). From a separate writable practice directory:

```text
<compiler> --version
<compiler> new hello
<compiler> run src/main.zry --project-root hello --target javascript --export main --name m6-first-js
<compiler> run src/main.zry --project-root hello --target webassembly --export main --name m6-first-wasm
```

`<compiler>` is the verified `compiler/bin/zryna` executable, or `zryna.exe` on Windows. The
documented generated source returns 42; this is the expected observation, not a new test result.
Choose fresh output names when repeating commands because publication is create-only.
Source-checkout users use the explicit compiler/project roots and pinned Node path in
[standalone projects](STANDALONE_PROJECTS.md#build-and-run-from-a-source-checkout).

For an editor exercise, install the exact published 0.5.0 package or the reviewed VSIX, configure
the matching server/installation in user settings, and open a trusted local practice folder.
Select the explicit profile, request Format Document twice, and exercise the documented scalar/M2
Run pickers after saving. Invalid source must yield actual compiler diagnostics and no formatting
edits. Follow [portable setup's editor exercise](PORTABLE_SETUP.md#editor-exercise) for concrete
sources, exports and expected values. Opening, editing, formatting and saving never execute code.

## Reviewed-source playground v2 workflow and limits

The [operator invocation and publication inputs](../spec/tooling/M6_REVIEWED_SOURCE_V2.md#operator-invocation)
describe the implemented source launcher API. It requires a pretrusted Node/bootstrap,
independently approved complete source composition, exact runtime images and sealed material
volumes, and a closed daemon approval. Public admission additionally requires the original
opaque capability from actual playground-compiler publication verification. A downloaded
policy JSON, a source build or a candidate receipt cannot replace that capability.

After the operator starts the loopback session, open its privately retrieved launch URL, edit
admitted source, compile, inspect actual diagnostics, choose a returned export, enter canonical
signed i32 values and run. Inspect the observed value and component hash. Editing or Cancel
invalidates pending observations; another operation waits for complete owned cleanup.
The guide gives a concrete `add(20, 22)` exercise with expected result 42. Those instructions
are reproducible inputs. Actual acceptance and public publication require the matching selected evidence.

The source is exact editable UTF-8 at fixed logical path `src/main.zry`, passed through the
scalar compiler, mandatory IR verifier, component audit and deterministic bindings. Only
returned exports with scalar i32 parameters/results are admitted. Inputs span -2147483648
through 2147483647; negative zero, spaces, leading zeros, plus signs, fractions and exponents reject.

| Source v2 boundary | Required finite restriction | Final acceptance status |
| --- | --- | --- |
| Source/request | 4096 UTF-8 source bytes inclusive; 32768 request bytes; fixed source/revision carrier | Exact/first-extra and malformed-carrier evidence required. |
| Compiler output | 1277956 frame bytes; 4096 stderr bytes; component <=1048576 bytes; each binding <=65536 bytes | Actual output and hostile-boundary evidence required. |
| Diagnostics | Compiler schema 1; <=65536 report bytes and 256 records; message/guidance <=4096 bytes each | Actual CLI/report parity required; LSP diagnostics v2 is unchanged. |
| Compiler | Whole operation 10 seconds; 512 MiB memory; zero swap; 32 tasks; two CPUs | Actual containment, resource and deadline execution proof required. Threads count as tasks. |
| Evaluator | Whole operation 5 seconds; 1 GiB memory; zero swap; 256 tasks; two CPUs; real Chrome sandbox | Actual browser process-tree, cancellation and resource proof required. |
| Cleanup | One shared additional 2 seconds; one operation, no queue | Complete owned cleanup required before acknowledgement. Unknown cleanup poisons admission. |
| Writable storage | Separate input/output named local-driver tmpfs volumes, 2 MiB each; compiler scratch 64 MiB; evaluator scratch 128 MiB; browser shared memory 64 MiB | Actual mount, quota and boundary observations required. |
| Trusted materials | Compiler <=64 files/192 MiB; evaluator <=512 files/1 GiB; sealed read-only volumes with complete per-operation audit | Inventory/configuration checks do not replace execution proof. |

The reviewed execution-host baseline is nonroot Linux amd64 with the fixed local Docker
Engine, authenticated socket peer and actual kernel/cgroup/namespace/seccomp capabilities.
The source provider validates the complete runtime rather than accepting an arbitrary daemon
or a distribution package by name. Chrome 153.0.8010.12 and playwright-core 1.63.0 are selected
material pins. Windows transport tests establish transport behavior or rejection, not a Windows
restricted execution host. This route does not require the v1 proposal's bubblewrap provider.

Packages, imports, source-selected files/commands/targets, native execution, ambient host grants
and arbitrary dependency fetching remain excluded. Host availability failures are separate
from source diagnostics. All source/diagnostic/error text is inert; keyboard, labels, focus,
announcements and Unicode navigation require actual browser accessibility evidence.

## Separate standalone toolkit v1 proposal

The [v1 contract](../spec/tooling/M6_CONFORMANCE_V1.md) retains its signed whole-toolkit archive,
proposed `playground-v0.1.0` publisher and protected documentation channel. Its proposed
nonroot Ubuntu 24.04 x86-64 host uses a separately reviewed delegated cgroup v2 domain and
bubblewrap 0.12 acquisition route. Its bounded toolkit archive and inherited staging/helper
limits belong to that proposal; they are not source v2 installation prerequisites or proof.
No standalone toolkit publication or supported launcher is advertised. Source v2 qualification
cannot mint a v1 archive capability or relabel its historical receipts.

## Final acceptance record

All advertised surfaces must be exercised against one authenticated integration commit and tree,
with exact per-artifact source/version/hash identities. The [v1 matrix](../spec/tooling/M6_CONFORMANCE_V1.md) and
[v2 successor mapping](../spec/tooling/M6_REVIEWED_SOURCE_V2.md#explicit-successor-mapping) require complete commands, platforms/profiles, nonzero counts, durable logs/documents/artifacts,
signature/provenance receipts and required hosted results. Earlier binaries or passing unit fixtures
cannot supply a missing final integration row.

Required proof includes actual CLI source-diagnostic/scalar parity across the editable
corpus; exact resource and malformed/expired/unsupported rejection boundaries; revision, cancellation,
isolation and authority substitution; real browser accessibility; runnable exported documentation;
project, LSP, formatter and installed-extension integration; and all applicable Linux/Windows
hosted gates. No row is marked passed by this page. Missing launcher/publication, browser baseline
or final receipts keeps restricted playground and M6 completion under verification.
