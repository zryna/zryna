# Downloadable distribution release v1

Status: contract candidate for [#406](https://github.com/zryna/zryna/issues/406). It defines an
outer release record and validation boundary but does not publish a release, create a tag, enable
downloadable compiler support, or mark any prerequisite complete. The source-only `v0.1.0`
Developer Preview remains separately owned by #421.

## Ownership and dependencies

This contract owns protected candidate/tag workflow identity, release subjects, detached SBOM and
provenance records, signed checksums and notes, reproducibility evidence, immutable asset
publication, verification, and revocation. #422 owns archive assembly, the finite installed
inventory, bundled runtime/provider admission, and the build receipt consumed here. #404 owns the
frozen build-plan execution evidence. Their records are authenticated inputs; this contract does
not redefine them or treat planned work as complete.

The synthetic [package release v1](../package/PACKAGE_RELEASE_V1.md) fixture remains unchanged. Its
small textual materials, unverified fixture signature, and comparison-only reproduction record
are not a production archive, SPDX document, SLSA statement, signature, or publication authority.

## Candidate identity

The proposed first downloadable compiler version is `0.2.0`, with tag `v0.2.0`. This avoids
collision with the reserved source-only `v0.1.0` preview and preserves the package authority's
existing strict three-decimal compiler compatibility grammar. The envelope independently requires
`channel: beta` and `publication: prerelease`; the version does not assert stability. The value is
a review proposal until an exact candidate is accepted; no workflow may create it speculatively.
Every production record binds one annotated tag-object SHA, its full peeled source commit and
tree, `refs/tags/<tag>`, the tagged workflow bytes, source epoch, workflow run ID and attempt, and
every required successful job at the same commit.

The canonical `zryna.release-tag-receipt.v1` is the first protected-workflow handoff. Its producer
accepts only the exact protected tag push context, requires the tag object to point directly to the
release commit, and requires the isolated checkout HEAD to equal that peeled commit. It reads the
tagged `.github/workflows/release.yml` as an ordinary non-executable Git blob and records its size
and SHA-256 identity together with the tag object, commit, tree, and commit epoch. Lightweight or
nested tags, mutable checkout content, wrong workflow refs, and symlink, executable, missing, or
oversized workflow entries are rejected before build or signing work begins. This local Git-object
receipt binds the inputs used by later jobs; repository ruleset enforcement and remote ref
immutability remain separately authenticated hosted checks.

Version 1 accepts only the existing three-decimal compiler version and binds beta/prerelease status
as separate closed fields. Stable publication remains outside this issue. The schema requires one
Windows x64 subject and one Linux x86-64 subject in canonical target order.
The initial qualification hosts are intentionally narrower than the bundled Node runtime's
upstream portability boundary:

| Target | Initial qualification host | Qualification still required |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 24.04 x86-64 | CLI and Node loader/import audit plus clean-host install/build/run |
| `x86_64-pc-windows-msvc` | Windows Server 2022 x64 with OS UCRT | CLI and Node PE import audit plus clean-host install/build/run |

Node 22.22.1 upstream records separately identify Linux kernel 4.18/glibc 2.28 and Windows
10/Server 2016 as runtime boundaries. They are dependency evidence, not Zryna support claims.
`ubuntu-latest` and `windows-latest` aliases do not establish a stable baseline. Any broader
support requires explicit compiled-CLI and bundled-Node loader audits plus observed clean-host
proof on the named platform.

## Subject and envelope graph

For each target, `subjects` binds the archive, #422 build receipt, detached SPDX SBOM, detached
provenance statement, exported attestation bundle, admitted platform baseline, and two-build
reproduction result. Version 1 requires byte-identical archives: both clean-build digests equal the
published archive digest. A future platform-qualified equivalence rule needs a separately reviewed
schema version and may not silently normalize timestamps or signatures.

Each clean assembly starts from a canonical
[`zryna.distribution-build-input.v1`](../../schemas/zryna-distribution-build-input-v1.schema.json)
record. It binds the exact source commit/tree/epoch and target baseline to authenticated toolchain
identities, #422's canonical `metadata/materials.json` and prepared `metadata/distribution.json`,
the compiled CLI bytes, the architecture receipt, the successful preassembly-gate receipt, and the
distribution recipe. Material entries are not duplicated in this outer handoff: their finite list
is owned by the separately hashed materials record and bound again by the prepared distribution.
The descriptor's `fileCount` counts `materials.json` entries; `archiveFileCount` counts every
regular archive file, including the CLI and metadata, and is at most 512. The former must be lower
than the latter. Neither declared count is evidence by itself: the workflow opens the retained
materials bytes, verifies their digest, schema, exact entries, origins, and count through #422's
authority, and verifies the full distribution inventory before compilation and assembly.
Fixed logical paths keep the handoff independent of runner-private filesystem names. The protected
gate set is recorded at the same source commit; skipped, stale, missing, duplicate, or unsorted
inputs are rejected before assembly.

The prepared distribution digest is embedded into the CLI at compilation. Assembly verifies that
embedded identity against the authenticated prepared input before creating postbuild inventory,
checksums, or a build receipt. The embedded digest is a binding value, not self-authenticating
evidence: the installation route must first authenticate the release/archive and then use it to
verify the prepared distribution record and installed bytes.

The canonical `zryna.source-build-receipt.v1` is identical across the two assemblies. It records
the exact source commit and tree, the exact JSON architecture command, the pinned Rust and Cargo
versions and executable SHA-256 identities, SHA-256 identities of the canonical Git bytes for
`Cargo.lock`, `Cargo.toml`,
`rust-toolchain.toml`, and `zryna.workspace.json`, and the successful empty-diagnostic report. Run
IDs, attempts, URLs, timestamps, runner paths, and randomized evidence are forbidden. Those hosted
identities belong to the separately authenticated preassembly-gate receipt and outer provenance;
they do not enter `metadata/distribution.json` or the compiled distribution digest.
The full canonical gate receipt contains repository/workflow/run identity, source commit, and the
ordered successful required jobs, but not its own path, size, or digest. Its build-input descriptor
holds those values; validation bounds and hashes the retained gate bytes before requiring exact
field-for-field projection, avoiding a self-hash cycle.

These records become evidence only through their protected producers. Against an isolated exact
tagged checkout, `create-source-build-receipt.mjs` derives HEAD/tree and the four input byte
identities from Git and rejects every tracked, untracked, or ignored workspace difference. Tooling
dependencies remain outside that source checkout. Rustup must select the supplied absolute Cargo
and rustc paths from its exact `1.97.1` toolchain; link, reparse, or proxy aliases are rejected.
The producer matches both executable SHA-256 identities before and after execution, rejects Cargo
configuration in its controlled Cargo home and checkout ancestors, removes ambient Cargo/Rust
overrides, and requires the exact full version outputs. It runs the fixed architecture command
offline without a shell and emits a receipt only for its exact empty-diagnostic result. Separately,
`create-preassembly-gates.mjs` uses the protected workflow token to select exactly one successful
manual CI run for the tagged commit, reads the current `main` required-status policy, and retrieves
the exact selected attempt's job and check-run IDs. Every required job must be completed and
successful at that commit and attempt, and the live required context set must exactly equal the
contract set. GitHub requests require identity encoding; responses have a fixed deadline,
prechecked declared length, and a bounded streaming body. A caller-authored JSON document,
matching pair of documents, or serialized empty
diagnostic array is not evidence that either command or hosted run occurred.

#422's embedded graph avoids cycles as follows:

- `metadata/distribution.json` binds source, target, recipe, and material identities;
- `metadata/inventory.json` lists every embedded regular file except itself and
  `metadata/checksums.sha256`;
- `metadata/checksums.sha256` covers every other embedded file, including the inventory, and
  excludes only itself;
- the external build receipt binds the complete archive and embedded inventory digests.

The outer envelope never enters the deterministic archive. Its checksum entries cover both
archives, both build receipts, both SPDX documents, both provenance statements, both exported
attestation bundles, the release notes, and the revocation policy. They exclude exactly
`SHA256SUMS`, `SHA256SUMS.sigstore.json`, and `RELEASE_NOTES.md.sigstore.json`, avoiding self-hash
and signature cycles. `SHA256SUMS.sigstore.json` signs the checksum document;
`RELEASE_NOTES.md.sigstore.json` signs the notes; and
`zryna-release-envelope-v1.sigstore.json` signs the canonical envelope while appearing in that
envelope only as an expected allowlist name, not a self-referential digest. GitHub's release
attestation and immutability cover the complete publication inventory, including the detached
bundles.

`assetAllowlist` is the exact sorted union of the envelope filename and every bound artifact path.
No extra release asset, generated source archive, debug file, installer, editor package, native
frontend, credential, or executable outside #422's audited archives is allowed.

## Signing and verification

Signing uses GitHub Actions OIDC with exact issuer
`https://token.actions.githubusercontent.com`. The certificate identity is derived from the record
and must equal:

```text
https://github.com/zryna/zryna/.github/workflows/release.yml@refs/tags/<tag>
```

Consumers verify the immutable GitHub release and local asset, the GitHub artifact attestation
restricted to `zryna/zryna/.github/workflows/release.yml`, each Sigstore bundle against the exact
issuer and identity above, the canonical envelope, every checksum entry, both SBOM/provenance
subject sets, and the installed archive inventory. A wildcard signer identity is not acceptable.

The checked JavaScript validators enforce canonical record shape, cross-field identities,
declared-digest bindings, coverage, ordering, and resource bounds. Shape/projection validation does
not authenticate its caller or a remote run. Only the protected producers above establish the
command and hosted observations, and their outputs remain subject to outer signing and provenance.
The validators do not perform a cryptographic
signature, transparency-log, attestation, SPDX, provenance, archive-byte, or hosted-release
verification. The protected workflow must invoke separately pinned verifiers for those subjects
and record their observed results.

## Protected publication

Release candidate work is read-only with respect to repository contents. Clean builds receive only
`contents: read`. The tag verification/signature job receives `contents: read`, `actions: read`,
`id-token: write`, and `attestations: write` so it can retrieve the exact candidate run and create
the release attestations and signatures. Only the final publication job receives
`contents: write`, plus `actions: read` to retrieve the exact signed bundle. It uses the protected
`binary-release` environment, an exact tag, and candidate artifacts selected by immutable run
identity and digest. It does not rebuild or fetch mutable materials.

Before activation, repository settings must provide an active `refs/tags/v*` ruleset restricting
creation, update, and deletion to an explicitly reviewed maintainer actor; a `binary-release`
environment limited to matching tags with no administrator bypass; and release immutability. The
environment must use an accountable approval configuration supported by the repository and agreed
for release. If required reviewers are configured, their exact actor IDs are reviewed and
self-review is disabled. This contract does not invent an additional external-human approval
prerequisite.

The one environment-gated job holding `contents: write` creates a draft, uploads exactly the
allowlist, re-reads and verifies the draft, then publishes once. Read-only preparation and signing
jobs perform no release mutation. Existing tags or releases, lightweight or wrongly peeled tags,
wrong commit/tree,
pending/skipped gates, missing attestations, digest drift, or extra assets stop publication.

The checked-in `release.yml` already fixes this authority and job ordering, but remains deliberately
fail closed while the accepted recipe digest, authenticated material acquisition, exact compile
recipe, and deterministic SPDX producer remain unapproved. The bounded build orchestrator,
two-replica byte comparison, evidence preparation, independent signature verification, and
four-phase draft publisher are present as reviewable inactive integrations; none is production
evidence. The admission job verifies the tag and required CI evidence before running
`check-release-readiness.mjs`; that check rejects the current tree before any build.
Removing or bypassing the readiness failure is not activation evidence. Activation requires one
reviewed change that fills every named prerequisite, freezes the exact recipe digest, and completes
the protected settings and dry-run evidence described here.

The canonical envelope is limited to 262,144 UTF-8 bytes, 32 nested containers, 1,024 total
containers, and 4,096 total JSON values, all inclusive. The CLI opens a direct regular file,
uses a nonblocking descriptor acquisition on POSIX before confirming regular-file identity,
retains and revalidates that identity, bounds size before allocation, reads at most one byte beyond
the observed size, and rejects mutation during the read. Text depth/container scanning and
iterative in-memory bounds run before JSON parsing, schema validation, or recursive
canonicalization. The in-memory API also measures exact canonical serialized bytes after the
bounded traversal and before schema error construction. This retained read is not a sandbox
against hostile ancestor directories or privileged same-host filesystem mutation.

## Revocation and recovery

Before publication, a failed candidate publishes nothing. After publication, a defective or
compromised release, tag, assets, attestations, and audit history are preserved. The release and
download documentation are marked affected, recommendation stops, and a signed revocation record
names every affected digest. A correction uses a new version and tag. Moving, deleting and
recreating the old tag, replacing an asset, or presenting a documentation edit as compiler
rollback is forbidden.

The checked schemas are the outer
[`zryna.distribution-release.v1`](../../schemas/zryna-distribution-release-v1.schema.json) envelope
and per-target
[`zryna.distribution-build-input.v1`](../../schemas/zryna-distribution-build-input-v1.schema.json)
handoff. Run their independent boundary cases with:

```bash
pnpm release:contract
```
