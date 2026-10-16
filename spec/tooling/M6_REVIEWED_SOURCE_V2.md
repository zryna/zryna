# M6 reviewed-source playground route v2

Version: 2. This evidence contract specifies a separate source-checkout composition for issue #410.
Execution and publication require their respective actual evidence described below. Final support
requires the matching independently selected final record and durable receipts; public launcher
admission additionally requires the original opaque compiler-publication capability.
This document records no completed acceptance, publication, supported launcher or milestone closure.

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
`reviewed-source-v1` and an independently approved `composition_sha256`. The approval pins
both runtime image identities separately. Its image-independent policy digest binds that
domain, complete material manifests and volume provenance, actual compiler executable,
binding template, WIT closure, CPUs and seccomp policy.
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

## Operator invocation

This is an operator template for the implemented API, not a published installer or a supported
public service. Public support requires final runtime qualification, immutable publication
and the complete matching acceptance record. No command below acquires authority from an adjacent manifest.

Before starting, the operator independently authenticates the bootstrap source (including the
publication verifier and its transitive imports), this session script, Node 22.22.1 and the
selected Git commit/tree. Use a private nonroot Linux amd64 host with the reviewed local Engine
and kernel capabilities. Prepare and review the exact compiler/evaluator images and sealed
material volumes with the [private preparation helpers](../../examples/playground/restricted/daemon/preparation/README.md).
Their returned records are unapproved observations. Approval follows full inventory review,
provisioning-writer removal and writable-alias exclusion; generating a plan or importing an
image does not approve it. There is no automatic generic Docker installation command.

For public admission the independently reviewed input set contains:

| Input | Required identity and authority |
| --- | --- |
| Compiler publication | Actual immutable `v0.1.0-playground-compiler.1` assets, exact repository/workflow/push identity and independently selected integration commit/tree |
| Publication policy | Closed version-1 policy binding source/tag object, envelope, archive, binary and provenance digests; the released `candidate-policy.json` is review input, not approval |
| Verifier and trust root | Independently pinned offline verifier executable, owner, size/hash and trusted-root bytes/hash; authentication does not disable transparency-log or certificate checks |
| Launcher descriptor | Closed `zryna.playground-source-launcher.v2` record with `compositionKind: reviewed-source-v1`, `source.kind: git-revision`, exact source inventory, Node/compiler/template/WIT and authenticated publication-receipt identity |
| Daemon approval | Independently approved closed version-2 record for that composition, both runtime images, sealed materials/provenance and fixed policy controls |
| Receipt file | Exact bytes of `JSON.stringify(compilerPublicationReceipt(capability))`, without a newline, whose descriptor was independently approved for this composition |

The verifier policy has exactly `version`, `path`, `bytes`, `sha256`, `ownerUid`,
`trustedRootBytes` and `trustedRootSha256`. The publication policy and launcher descriptor use
the closed schemas in the reviewed verifier and `launcher-policy.mjs`; do not invent missing
fields or copy a working-snapshot candidate into the public domain. Changing publication
status/receipt changes the composition digest and requires a fresh independent approval.
The original opaque capability remains in memory in the original pretrusted module instance;
serializing its receipt and importing it later cannot restore that authority.

Save the following as `m6-source-session.mjs` at the root of the already authenticated checkout.
Replace only `inputPin` with the independently approved descriptor of an operator input file.
That file has exactly seven keys: `policy`, `verifierPolicy`, `envelope`, `bundle`, `archive`,
`trustedRoot`, `reviewedLauncher`. Every value is an independently approved ordinary-file
capture descriptor with exactly `path` (absolute), `bytes`, `sha256` and `ownerUid`.
These are trusted operator inputs, never request parameters or browser inputs. The descriptor
for the input file must come through a trusted channel; hashing an untrusted input file now
cannot approve it. Retain and review the actual receipt file before approving the final
launcher descriptor. This script deliberately does not generate an approval.

```js
import { captureFile } from './examples/playground/restricted/daemon/launcher-capture.mjs';
import { closedJson, exact } from './examples/playground/restricted/daemon/launcher-policy.mjs';
import { authenticateCompilerPublication } from
  './examples/playground/restricted/daemon/compiler-publication-capability.mjs';
import { createPublishedSourceLauncher } from
  './examples/playground/restricted/daemon/launcher.mjs';

// Operator-reviewed constants; zero bytes makes this unconfigured template reject.
const inputPin = { path: '/operator/reviewed-inputs.json', bytes: 0,
  sha256: 'REPLACE_WITH_INDEPENDENTLY_APPROVED_SHA256', ownerUid: 1000 };
const limits = { policy: 262144, verifierPolicy: 262144, envelope: 262144,
  bundle: 1048576, archive: 513 * 1024 * 1024, trustedRoot: 262144,
  reviewedLauncher: 262144 };
const captures = [];
let service;
let signal;
let interrupted = false;
const stopped = new Promise(resolve => { signal = () => { interrupted = true; resolve(); }; });
const failures = [];
process.once('SIGINT', signal);
process.once('SIGTERM', signal);
try {
  const input = captureFile(inputPin, 262144); captures.push(input);
  const pins = closedJson(input.bytes, 262144);
  exact(pins, Object.keys(limits));
  const data = {};
  for (const [name, maximum] of Object.entries(limits)) {
    const file = captureFile(pins[name], maximum); captures.push(file);
    data[name] = file.bytes;
  }
  const publication = await authenticateCompilerPublication({
    policy: closedJson(data.policy, limits.policy),
    verifierPolicy: closedJson(data.verifierPolicy, limits.verifierPolicy),
    envelope: data.envelope, bundle: data.bundle, archive: data.archive,
    trustedRoot: data.trustedRoot,
  });
  for (const file of captures) file.revalidate();
  service = await createPublishedSourceLauncher(
    closedJson(data.reviewedLauncher, limits.reviewedLauncher), publication);
  for (const file of captures) file.revalidate();
  if (interrupted) throw new Error('Operator interrupted startup');
  await service.launch();
  // Private terminal only: the fragment is a secret capability. Do not retain this URL in logs.
  process.stdout.write(`${service.takeLaunchUrl()}\n`);
  await stopped;
} catch (primary) { failures.push(primary); }
finally {
  try { if (service) await service.stop(); } catch (cleanup) { failures.push(cleanup); }
  for (const file of captures.reverse()) {
    try { file.close(); } catch (cleanup) { failures.push(cleanup); }
  }
  process.removeListener('SIGINT', signal);
  process.removeListener('SIGTERM', signal);
}
if (failures.length) throw new AggregateError(failures, 'Source session failed');
```

Run it with the independently approved absolute Node executable and script paths, without
inherited Node loader options. For example, after replacing the two operator-selected paths:

```text
env -i PATH=/usr/bin:/bin /absolute/pinned/node /absolute/reviewed-checkout/m6-source-session.mjs
```

Open the printed loopback URL privately on that host. The fragment must not be copied into
shared logs, tickets or durable acceptance evidence. `takeLaunchUrl()` succeeds once. The
listener exposes the fixed source UI on `127.0.0.1`; it does not accept a user-selected host,
Engine, guest command or filesystem root. Stop with Ctrl-C and await `service.stop()`.
If creation, operation or cleanup rejects, preserve the actual failure and sanitized
`service.recovery()` information when available, then use the exact owned-resource recovery
procedure. Do not force-delete uncertain objects or submit another operation after unknown cleanup.

Private prepublication qualification instead calls `createSourceLauncher(reviewedCandidate)`
with an independently approved `qualification-candidate` descriptor and matching candidate
receipt. It returns `qualificationOnly: true`. It neither authenticates a published compiler
nor supplies a public launcher capability. A working snapshot is admitted only in that private
domain; the public source and compiler commit/tree must match exactly.

### Editable-source exercise

In the source editor enter:

```zryna
export function add(a: i32, b: i32): i32 {
  return a + b;
}
```

Compile, select the returned `add` export, enter `20` and `22`, and run. The expected scalar
observation is `42`; retain the actual component identity and bounded result receipt without
the launch secret. Edit the return expression to `a - b`, compile again and run the same
arguments; the expected value is `-2`. An edit must invalidate the previous executable/result.
Introduce a syntax error and compile: actual bounded compiler diagnostics must appear, with
no newly runnable component. Restore the valid source before the next operation. Cancel an
in-flight operation and await complete cleanup; a late success must not replace cancellation.
These are exercise inputs and expected behavior, not recorded passes or a replacement for
the complete corpus, accessibility and containment obligations.

## Publication and exported documentation

The compiler publisher reserves exactly seven immutable release assets:
`compiler-envelope.json`, `compiler-envelope.sigstore.json`, `candidate-policy.json`,
`zryna-playground-compiler-0.1.0-x86_64-unknown-linux-gnu.tar.gz`,
`publication-evidence.tar.gz`, `publication-evidence-index.json` and
`publication-evidence-index.sigstore.json`. This list specifies the planned publication;
it does not assert that the release exists. The evidence index/layout are version 2: two
fixed canonical nested replica evidence archives retain every selected crate/license,
metadata, dependency-tree proof and before/after build-command capture within the unchanged
finite aggregate limits. They are inert evidence, not a second executable delivery route.

The final source, runnable guide and website registry must land before the exact protected
compiler source is frozen. The publisher requires successful complete CI on that same main
commit and an independently reviewed actual qualification packet. Publishing at one commit
then editing the integration while reusing the old compiler capability cannot establish the
required source/compiler identity. The separate 23-gate record still needs its own complete
actual receipts and durable retention; the seven compiler assets do not publish those gates.

The website registry includes this document and the tooling support page. From the final
clean `main` checkout, after replacing `EXACT_MAIN_COMMIT` with its independently reviewed
40-character commit and choosing a fresh child of `.zryna/out/docs`, export with:

```text
/absolute/pinned/node scripts/docs/export.mjs --channel next --source-commit EXACT_MAIN_COMMIT --source-ref refs/heads/main --output .zryna/out/docs/m6-next
```

The exporter verifies Git provenance and emits the exact manifest digest. Validate against
independently retained expectations:

```text
/absolute/pinned/node scripts/docs/check.mjs --bundle .zryna/out/docs/m6-next --expected-manifest-sha256 REVIEWED_MANIFEST_SHA256 --expected-channel next --expected-source-commit EXACT_MAIN_COMMIT --expected-source-ref refs/heads/main
```

For official acquisition, the existing manual `documentation.yml` workflow additionally requires
the successful exact-main full CI run/attempt and independently reviewed canonical hosted
expectations JSON/digest; see [documentation publication](../../docs/DOCUMENTATION_BUNDLES.md).
It retains the authenticated bundle and gate metadata as finite-lived CI artifacts. Independently
validate every document, retain that digest with the actual runnable-guide
exercise and final gate record, and use the website's authenticated bundle-import process.
Export alone is not deployment. `next` identifies the exact main source documentation; it
claims no compiler/toolkit release. The protected `playground-0.1.0` channel and
`export-playground.mjs` remain exclusively the original standalone v1 route and its
`playground-v0.1.0` protected source. The new compiler tag does not authorize that channel.

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
