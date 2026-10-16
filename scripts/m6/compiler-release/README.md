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
build. The actual app normal/build cargo-tree identity set must exactly equal the
metadata-selected closure; workspace-wide feature overapproximation rejects.
No target/guest can choose commands, Cargo flags or dependencies.

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

The push lane independently authenticates the selected CI, tag, current protected
main, ruleset, packet and review. It rebuilds two independent release replicas,
compares actual bytes with the qualified candidate, creates the compiler-only
archive through the existing unsigned producer, signs with real GitHub OIDC and
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
finite evidence index. Expiring Actions artifacts are transport only. The signing
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
