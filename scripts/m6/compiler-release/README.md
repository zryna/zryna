# Compiler-only protected publisher

This lane prepares the distinct `zryna-playground-compiler` 0.1.0 prerelease. It
neither publishes the old standalone toolkit nor completes #410 acceptance.
Nothing here has been executed as a hosted release merely by merging the files.

The workflow's read-only `workflow_dispatch` requires an actual successful full
CI run ID/attempt on this exact current main revision. It builds two independent
release replicas and provides `compiler-assembled-RUN-ATTEMPT` with an inert exact
`compiler.bin`, source/build/material identities and real reproduction evidence.
Candidate building needs no qualification packet and grants no public authority.
All original crate archives and every executed extracted dependency source file
are independently checked against the approved material union before and after
build. The actual fixed app normal/build Cargo tree selects identities from the
workspace metadata; only absent edges explicitly declared optional with matching
dependency aliases, targets and kinds may be pruned. Unknown identities, missing
mandatory dependencies, ambiguous identities and unreachable extras reject. This
uses Cargo's package-scoped feature resolution without reimplementing it. Both raw
metadata and tree are retained and rerun immediately after the build; their exact
selection and all authenticated selected caches must remain unchanged. Replica
receipt/tree-proof v2 records this before/after check and all seven actual commands.
No target/guest can choose commands, Cargo flags or dependencies.
The completed Cargo executable may be hardlinked to its `deps` entry. Only that
fixed build output permits multiple links while being captured through retained,
unchanged file and ancestor identities. The exported `compiler.bin`, publication
materials and all other inputs retain their existing single-link checks.

An authorized maintainer separately obtains the actual candidate, qualifies that
exact release binary/source/material composition on a supported operator host,
and independently reviews the complete raw observations. Import
`qualificationRegistry`, `qualificationRegistrySha256` and
`createQualificationPacket` from `qualification.mjs` for the closed prepublication
case inventory and canonical packet encoding. The registry preserves playground,
transport Linux/Windows, containment, browser and accessibility cases. All other
owning tooling, final publication, docs and 23-row obligations remain required by
the final source-route evidence contract.

The packet is a canonical epoch-zero tar/gzip rooted at
`m6-compiler-qualification`, with `qualification.json`, `review.json` and exactly
its `raw/` inventory. The review must name a real retained read-only confirmation.
The parser checks closed records, complete case coverage and byte correspondence;
it does not derive runtime truth from arbitrary raw text. Actual semantic
validation and material/case correspondence belong to the independent operator
review selected by the maintainer's protected annotated tag. Synthetic schema
fixtures cannot qualify an actual runtime or acquire publication capabilities.

Only the authorized maintainer uploads that reviewed packet as asset
`m6-compiler-qualification.tar.gz` on a separate temporary draft release named
`m6-qualification-*` in zryna/zryna. The draft is mutable private transport, not
published/durable evidence. After reviewing its exact asset ID and packet/review
hashes, the maintainer creates the protected annotated final tag
`v0.1.0-playground-compiler.1`, directly naming the exact qualified main commit.
Its message ends in one blank line and exactly these five trailers, with no spaces
between colon and value:

```
M6-Full-CI-Run:POSITIVE_RUN_ID
M6-Full-CI-Attempt:POSITIVE_ATTEMPT
M6-Qualification-Asset:POSITIVE_ASSET_ID
M6-Qualification-SHA256:LOWERCASE_PACKET_SHA256
M6-Qualification-Review-SHA256:LOWERCASE_REVIEW_JSON_SHA256
```

The push lane has two admission phases. Read-only `admit` authenticates selected
CI, tag, current protected main and ruleset, recording
`protected-tag-source-admitted` with `qualification:null`. The read-only build
jobs create two independent replicas; read-only `assemble` explicitly defers
private-packet access and produces unsigned, unqualified compiler-only bytes.
These bytes and candidate-policy JSON grant no qualification or runtime authority.
A contents-read token cannot inspect the private draft; these jobs never attempt it.
The trusted CLI selects deferral, while the admission and assembly library APIs
still require full qualification by default for publishing.

Only the publisher, with its existing contents-write permission, freshly reads
and authenticates the complete private packet/review. It requires its fresh
`protected-tag-admitted` admission with nonnull qualification to agree with the
original source-only admission's source, selection and run. Original complete
68-case and actual replica-binary validation precede archive production, signing
and release mutations. It independently reassembles and compares all three exact
unsigned files from read-only assembly, then signs with real GitHub OIDC and
public Sigstore, and verifies the exact new workflow/tag/source identity. Only
the publisher job has write/OIDC permissions. There is no new environment approval
or administrative credential requirement.

The final release is created once as a draft, receives exactly seven checked
assets, is published once and must actually report `immutable:true`. An existing
final release is never edited, deleted or overwritten. The original immutable
v0.2.3 release readback is a historical-release observation; it does not prove
current repository settings. Reading those settings requires Administration:read,
which standard workflow permissions do not supply. If the newly published release
is mutable, the lane fails and retains the created release/evidence without
claiming public acceptance or deleting/reverting it.

The immutable assets include the entire reviewed packet/raw evidence, API tag/CI
observations, both actual build/material proofs and reproduction, under a signed
finite evidence index. Evidence index/layout version 2 keeps the two complete replica
inventories in exactly `replica1-evidence.tar.gz` and `replica2-evidence.tar.gz` inside
`publication-evidence.tar.gz`. Each canonical nested archive retains every original
metadata, command-output and license byte (excluding the separately published compiler
binary), with a complete hashed inventory; neither replica's duplicate licenses are
dropped. Nesting stops at those two fixed subjects. `decodePublicationEvidence` verifies
both levels, exact inventories and source/epoch with unchanged 400-file per-replica,
500-file outer, 96-MiB compressed archive and 96-MiB aggregate expanded-byte bounds
(including the generated layout record). The shared 512-file distribution limit remains
unchanged. A flat v1 archive cannot masquerade as this v2 layout.
Expiring Actions artifacts are transport only. The signing
bundle/index avoid archive/self-hash cycles; final publication/API observations
and failure logs are retained separately for the owning independent acceptance.

After actual publication, a separate fresh official acquisition must authenticate
the complete release, use the opaque compiler-publication capability, exercise
that exact published artifact, complete all original final gates and publish the
exact runnable documentation. Candidate-policy JSON is review output and never
self-approves runtime activation. No tag, draft upload, release creation, workflow
trigger, deployment or artifact execution is performed by these unit tests.

Validate this lane with:

```
node --test --test-concurrency=1 scripts/m6/compiler-release/*.test.mjs
```
