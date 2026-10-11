# M4 documentation publication and live evidence

This contract separates authenticated compiler execution, official documentation artifact
verification, website import, website CI and observed production publication. Preparing these
scripts does not pass any gate. An authenticated documentation artifact is not a signed public
compiler release and cannot activate installed server, browser or component selectors.

## Exact main execution before documentation export

Run the complete CI workflow on actual `refs/heads/main` at the final merged compiler source.
Use its `workflow_dispatch` with the full trusted ancestor `structure_base`; ordinary path-routed
PR success and a matching tree under another commit cannot certify the merge revision.
All required foundation and routed contract jobs plus all six M4 jobs must complete successfully
in one exact CI run and attempt. Required outcomes may not be skipped. Authenticate actual
checkout commit/tree, workflow blobs, runner facts, executable/material identities, bounded test
accounting, artifacts and all registry case mappings. Source gaps remain blockers.

An independent reviewer supplies canonical `zryna.m4-hosted-expectations.v1` bytes and their
SHA256, using authenticated run/job/check/artifact IDs from that explicit run. Source, workflow,
tool and material pins must be independently reviewed; the receipt cannot select its own trust
expectations. Preserve the observed host fields and original absolute evidence directory.
Do not substitute `latest`, synthesize GITHUB context, or use an artifact's self-declared digest.

Dispatch `.github/workflows/documentation.yml` on that actual current main revision with
`ci_run_id`, `ci_attempt`, `expectations_sha256` and `expectations_json`. The last input must retain
canonical two-space JSON and its final LF; the workflow reads it as bounded data through an
environment variable, never shell interpolation. The dispatch identity and compiler HEAD must
remain equal. The verifier authenticates completed CI through the Actions API, downloads all six
exact archives using normal TLS and independently pinned API digests, checks hostile ZIP inputs,
and independently validates every retained receipt. It rechecks current main before export.
A current-main advance requires fresh final-source evidence.

The official export preserves native `GITHUB_SHA`/`GITHUB_REF` and the existing clean-source
provenance verifier. Its create-only output includes actual documentation run ID and attempt.
`zryna.docs.bundle.v1` remains Markdown-only: support JSON is part of the authenticated M4 support
page. No JSON sidecar, fake official PR source or importer exception is allowed. The separate
metadata gate artifact and documentation artifact are retained for 90 days; retain the original
six source archives independently before expiry. Retention is availability, not authentication.

## Authenticate the uploaded artifact

After the documentation run completes successfully, independently review and pin canonical
`zryna.m4-docs-artifact-expectations.v1` with source commit/tree/ref, registry/matrix/manifest
SHA256, exact reviewed documentation workflow SHA256, hosted expectations SHA256, documentation
run ID/attempt and metadata/document archive descriptors. Each archive descriptor records ID,
exact bytes, SHA256 and expiry. Do not copy a candidate's authority claims as expected data.

Run from that exact clean compiler source:

```sh
node scripts/m4/verify-docs-artifact.mjs \
  --expectations /outside-checkout/reviewed-docs-artifact-expectations.json \
  --expectations-sha256 <independently-reviewed-expectations-sha256> \
  --output /outside-checkout/create-only-retained-publication
```

The verifier authenticates the successful exact docs run/job/check and both API archive metadata,
including chronology within the selected attempt. It hashes actual downloaded ZIP bytes, rejects
extra or hostile entries, checks externally pinned hosted expectations in the metadata archive,
revalidates the original full CI and six M4 evidence archives, then validates the complete docs
manifest and exact canonical registry matrix in its authenticated Markdown. Computed eligibility
covers this documentation artifact only; it does not certify the website or milestone closure.

## Website import and authenticated deployment

The website repository uses `src/content/compiler-data/next/` plus the canonical
`src/content/compiler-data/compiler-docs.lock.json`. Vendor only the verified actual main docs
artifact and review its exact source, manifest digest and complete document/route inventory.
Run the existing `tools/docs/import-bundle.mjs --write next` after normal bundle/lock validation;
keep immutable 0.1.0 imports and the separate 0.2.1 tutorial unchanged. Raw HTML remains forbidden.
Update authored status/provenance/getting-started/roadmap pages with precise bounded M4 support
and actual run/attempt/artifact/archive identities. The support/conformance routes are
`/reference/compiler/next/reference/m4-support/` and `.../m4-conformance/`.

Use the final website revision's enhanced published-content checker in dist and served modes.
The checker validates the existing canonical lock/import pipeline, exact canonical matrix and
registry digest, complete routes and examples, and bounds/hashes every actual response. Its own
Git commit/tree identifies checker source. It does not identify the current production website
commit or image from compiler text. Require fresh successful website Windows and Linux CI on
the exact final main website source before its authorized production deployment.

Live closure additionally requires independent authorized provider evidence for the actual
website commit/tree, deployment/application ID, image digest and deployment time, associated
with the observed served response bytes. Collect current provider facts using existing access;
absence of a named deploy secret in source does not establish an access blocker. Do not invent
credentials, API URLs, application IDs or deploy jobs. A health response does not supply source
identity. Keep deployment provenance unknown until actual evidence establishes it.

The final external publication receipt binds compiler source/tree/registry/matrix; exact full CI,
docs run/attempt and API artifact/archive/manifest identities; website source/tree/import lock
and both hosted checks; observed provider/source/image/deployment identity; and bounded live
URLs/status/security/redirect/source/matrix/response hashes from that deployment. Closure needs
all of these. Static support equality or successful docs export alone cannot close #402/M4.
