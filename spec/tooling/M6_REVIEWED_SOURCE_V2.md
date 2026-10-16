# M6 reviewed-source playground route v2

Version: 2. Status: implementation and runtime qualification are pending. This document
specifies a separate source-checkout composition for issue #410. It records no completed
acceptance, publication, supported launcher or milestone closure.

The original issue requires published compiler artifacts, actual restricted browser evaluation,
one exact tooling support matrix, runnable exported documentation and complete required gates.
The proposed [standalone toolkit v1](M6_CONFORMANCE_V1.md) remains a separate distribution
route. A v2 source composition cannot create or replace its signed archive capability.
The merged [source trust policy](../package/SOURCE_TRUST_V0.md), public browser profile,
compiler semantics, diagnostics, ABI and component verification retain their existing owners.

## Published compiler and source identity

The new product is `zryna-playground-compiler` version `0.1.0`, targeting
`x86_64-unknown-linux-gnu`. Its executable must be built from the recorded clean integration
commit/tree, actually published and independently authenticated before public admission.
Protected publication uses an annotated `v0.1.0-playground-compiler.1` tag and
`.github/workflows/playground-compiler-release.yml`. Verification requires the exact workflow,
repository, tag, source commit and push trigger under the GitHub Actions OIDC issuer,
complete finite archive inventory, binary identity and independent verifier/trusted-root pins.
A source build used before publication is a qualification candidate; it cannot establish a
published compiler or public support pass. The public launcher consumes verified published
bytes and does not silently rebuild them.

The immutable compiler `v0.2.3` remains at its original source and artifact identities.
Its separately verified release envelope and complete archive authenticate the reused Node
and legacy nine-file TypeScript provider closure. Rebuilt or newer provider modules cannot
inherit that release's identity. The new playground executable is a distinct product built
from the integration source; it is never relabeled the original released compiler binary.
Server/VSIX `0.5.0` and setup `0.1.0-candidate.3` also retain their original product identities.

The remaining wrapper, UI, supervisor, binding and runtime composition needs independently
selected exact source commit/tree and a complete finite byte inventory. Executed bootstrap
code and Node are explicitly trusted operator inputs. An adjacent manifest or a self-computed
hash cannot authenticate the bootstrap that already executed. Copying approved source into a
private retained snapshot protects consumption; it does not by itself authenticate Git history.

## Separate approval and receipt domain

The closed daemon operator record is version 2, with `composition_kind` exactly
`reviewed-source-v1` and an independently approved `composition_sha256`. Its policy digest
binds that domain, both runtime image identities, complete material manifests and volume
provenance, actual compiler executable, binding template, WIT closure, CPUs and seccomp policy.
Approval comes through the operator's trusted channel before guest effects.

For this separately identified route only, the existing wire field
`authority.toolkitSha256` carries the approved source composition digest. This compatibility
field asserts no signed toolkit archive or outer publisher authentication. The source launcher
and its publication verifier must retain the route and actual published compiler capability;
a plain metadata object cannot upgrade a candidate to public admission. Legacy v1 signature,
archive and retained-capability APIs remain unchanged and cannot accept this substitution.

## Containment and trusted material storage

The fixed local Docker Engine socket and peer owner, immutable minimal runtime images and
operator-sealed role-specific material volumes implement the source policy's containment
boundary. Engine, kernel and operator administration are trusted infrastructure. No source
request can select a daemon, image, tool, path, argument vector, target, package or grant.

Persistent material volumes are not intrinsically immutable. The operator must finish and
remove every provisioning writer before approval, prevent writable aliases during use and
independently approve volume name, creation identity, nonce, manifest digest and storage totals.
Every material runtime mount is read-only with copy-up disabled. Before any source upload, the actual
production auditor verifies complete canonical inventory, ordinary single-link files, ownership,
modes, sizes, hashes, retained and named identity, and absence of missing or extra entries.
The driver also verifies actual read-only mounts and exact daemon volume provenance.

| Material role | Files | Complete file bytes | Individual file bytes |
| --- | --- | --- | --- |
| Compiler | At most 64 | At most 192 MiB | At most 128 MiB |
| Evaluator | At most 512 | At most 1 GiB | At most 512 MiB |

Docker archive copy ignores the destination root directory metadata. During preparation,
a fresh empty volume receives the fixed runtime image's empty `0:0` mode `0555` material
directory through Docker's create-time volume initialization. Only that never-started,
nonroot provisioning holder uses `NoCopy:false`. Before any archive upload, a bounded
complete archive readback must contain exactly the empty material root at `0:0:0555`.
The source-verified file stream then populates the volume. The holder must remain created
with PID zero and must be removed with a confirmed independent not-found readback before
approval. No user code or preparation process executes in that holder.

Guest compiler/browser material mounts remain read-only with `NoCopy:true`; the production
auditor still checks the complete installed inventory before source upload. A failed,
partial or stale preparation cannot activate a previous candidate approval.

These are separate finite trusted tool storage bounds, including pinned Node and complete
Chrome. They do not increase guest writable scratch or replace resource execution proof.
Manifests allow at most 512 directories, depth 16 and 240-byte portable relative paths.
Root/directories are `0:0` mode `0555`; ordinary files are `0:0`, mode `0444` or `0555`.
Symlinks, hard-link aliases, special files and unknown inventories reject.

Compiler and evaluator run in separate fresh containers under UID/GID `65534`, all OCI
capabilities dropped, no-new-privileges, read-only roots, private PID/cgroup namespaces,
network namespace `none`, reviewed default-deny amd64 seccomp and finite writable tmpfs.
Chrome retains its real browser sandbox; no sandbox-disabling switch is admitted. Browser
private user-namespace capabilities are distinct from privileges in an ancestor namespace.

The evaluator protects `/proc/bus`, `/proc/fs`, `/proc/irq`, `/proc/sys` and
`/proc/sysrq-trigger` read-only. Its private proc mount permits the kernel-gated writes to
the browser process's own new user namespace maps and deny-setgroups control that Chromium
needs to establish its sandbox. Making the entire proc mount read-only prevents those
writes and rejects sandbox startup. Compiler and collector roles retain the whole read-only
proc mount. No host proc mount, OCI capability, namespace flag or seccomp rule is added.

| Guest boundary | Required unchanged bound |
| --- | --- |
| Source/request | 4096 UTF-8 source bytes; 32768 request bytes; fixed `src/main.zry` |
| Output | 1277956 compile frame bytes; 4096 stderr bytes; component 1048576 bytes; each binding 65536 bytes |
| Diagnostics | 65536 report bytes; 256 records; 4096 bytes per message/guidance field |
| Compiler | 10-second whole operation; 512 MiB memory; zero swap; 32 tasks; two CPUs by quota and effective cpuset |
| Evaluator | 5-second whole operation; 1 GiB memory; zero swap; 256 tasks; two CPUs by quota and effective cpuset |
| Cleanup | One shared additional 2-second deadline for complete owned-resource removal |
| Operation storage | Separate input/output tmpfs of 2 MiB each; compiler scratch 64 MiB; evaluator scratch 128 MiB |
| Browser shared memory | 64 MiB; compiler shared memory 1 MiB |

Deadlines include runtime creation, material audit, staging, startup, execution and output
collection. Cleanup has its separate shared budget; neither phase renews per API call.
No queue is admitted. Cancellation after cleanup and before result settlement must still
suppress success. Unknown cleanup retains owner, primary and cleanup failures and poisons
further admission. Kill, process exit or empty population alone cannot acknowledge cleanup.

Quiescence also proves that the original fixed, single-threaded PID1 has no unreaped
children. On a fully visible proc mount, a bounded scan of at most 8192 process stat
records includes zombies, with the same PID1 identity and whole-cgroup task count one
checked before and after. Hidden process mounts, denied or malformed reads and expired
observations reject; only a vanished process entry may be skipped. This supports kernels
that omit `task/*/children` without requiring host tracing privileges or changing guest limits.

After the fixed guest execution and its descendants are quiescent, a trusted idle collector
holds the output volume read-only before the producer namespace stops. This preserves the
named tmpfs contents across producer removal. Collection still starts only after producer
stop and wait; the complete stdout/stderr inventory and all owned cleanup checks remain required.

## Explicit successor mapping

The [v2 registry](../../scripts/m6/source-registry.mjs) preserves all 23 v1 gate/platform
rows. All original project, LSP, formatter, installed editor, setup, playground, transport,
containment, browser, accessibility, documentation, canonical and hosted case obligations remain.
Linux and Windows tooling/hosted claims each need actual exact-source receipts. Unsupported
host rejection is required negative evidence and cannot supply positive runtime acceptance.

Only these proposed implementation-specific identities change on this separate route:

| v1 proposal identity | v2 source-route identity and required proof |
| --- | --- |
| Signed whole `playground-toolkit` | Independently reviewed finite source composition plus independently authenticated actually published playground compiler |
| `bubblewrap-linux` / `python-linux` | Fixed daemon supervisor / Docker Engine, their complete runtime inputs, actual readbacks and containment/cleanup proof |
| Publication `outer-signature` case | `published-compiler-signature`, including exact protected builder/tag/source and complete archive verification; remaining source composition retains independent operator approval |
| Direct Ubuntu/delegated cgroup provider | Reviewed fixed Linux amd64 daemon provider; actual kernel limits, namespaces, denial and unsupported-host evidence remain mandatory |

The nested original compiler signature, immutable artifacts, independent download and actual
runnable playground exercise remain required publication cases. This mapping does not waive
any original issue gate or activate the unimplemented standalone toolkit route.

## Required actual acceptance

The exact editable corpus must match independently executed CLI diagnostics and scalar
observations. Generated audited artifacts and real browser execution must supply results;
preset oracles, synthetic frames and a handwritten evaluator cannot do so. Malformed,
oversized, expired, unsupported and substituted-authority requests must remain bounded.
Real cancellation, material substitutions, resource boundaries and first-extra failures,
process descendants, late responses, cleanup failures and recovery need actual observations.
Configuration readbacks alone do not establish resource boundary execution.

Accessibility requires actual browser keyboard, focus, labels, announcements, diagnostic
navigation, inert hostile text and accessibility-tree observations. Installed editor evidence
must authenticate the installed selected VSIX payload and extension path; a repository
extension development host is subordinate evidence. A Linux virtual-display editor run is
native Linux GUI evidence and cannot establish a Windows or physical desktop result.

One complete final record must bind clean integration source, all product/source/material
identities, exact commands, inputs, nonzero required counts, raw retained observations,
primary/cleanup failures, actual hosted job conclusions and exported-document hashes.
Receipts and artifacts require immutable durable retention; expiring CI artifacts alone do
not establish publication. Actual runnable documentation must describe the same tested
profiles, versions, limits and support state. No absent, skipped or synthetic result is a pass.

Implementation may integrate while #410 remains open. Publication and subsequent final
acceptance must identify that integration honestly and rerun affected gates for later changes.
Only complete reviewed evidence, genuine required hosted success and consistent published
documentation permit closing #410 and claiming M6 complete.

## Detached v2 record validation

`scripts/m6/source-evidence.mjs` validates the separate canonical
`zryna.m6-source-evidence.v2` record against an independently selected record
digest, clean integration commit/tree, full artifact inventory and all 23
platform gates. Original case and material obligations remain mandatory; only
the documented publication-signature successor changes its case name.
A playground-compiler artifact identifies the published archive. The publication
section binds its archive hash and distinct executable hash, the reviewed source
composition and documentation manifest. An original compiler-publication
capability additionally binds the authenticated archive and executable to the
record's integration commit/tree; a receipt dictionary cannot supply it.

This validator creates no authentication or execution capability. Successful
synthetic schema tests do not establish actual gate execution, retained raw
receipts, a final signature, immutable publication, documentation deployment or
M6 support. The standalone v1 record and toolkit closure APIs retain their own
format, registry, signature and opaque-capability requirements.
