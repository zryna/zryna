# Performance platform linkage

This is one preparatory part of #418. It links a completed GitHub Actions run to
reviewed workflow bytes, four exact jobs and six exact artifact descriptors. It
does not authenticate archive contents, admit baselines, freeze thresholds, or
qualify performance. Reports retain `sourceAuthenticated: false`,
`artifactsVerified: false`, `performanceQualified: false`, and `thresholds: null`.

The existing `cli-performance.yml` entry calls the local reusable collector. The
collector keeps the same Linux/Windows matrix, two fresh rounds per host, pinned
tools, read permissions, 60-minute budget and failure retention. Distinct
concurrency groups prevent the caller and collector from cancelling each other.
The producer retains both workflow source pins. Earlier direct runs with empty
`referenced_workflows` cannot be retroactively linked through another CI run.

Prepare canonical `zryna.cli-performance-hosted-expectations.v1` JSON and review
its complete bytes outside the candidate. Supply its SHA-256 separately:

```sh
node scripts/stability-gates/verify-cli-performance-platform.mjs \
  --expectations /private/expectations.json \
  --expectations-sha256 REVIEWED_SHA256 \
  --output /private/new-platform-report.json
```

Use the normal GitHub API credential environment; do not put credentials in the
expectations or command arguments. This command uses live bounded API reads and
creates the output once. It accepts no saved API response or approval override.
With the managed cloud proxy, invoke Node 22 with `--use-env-proxy` and preserve
the configured proxy and CA trust.

Expectations bind repository numeric identity, candidate commit/tree/branch,
execution commit/tree/parents, first run attempt, event and selected PR, both
workflow byte lengths/digests, and every job/check/artifact ID. PR execution must
be the attached synthetic merge with explicit base and candidate parents. Both
candidate and execution workflow blobs must match the same reviewed bytes.
The exact same-run reusable reference is required. Jobs and checks must complete
successfully, mandatory collection/retention steps must execute on their owning
hosts, and artifacts must match source/run/repository, digest, size and lifetime.
Missing, duplicate, extra or retried roles fail. An inspected JSON report cannot
substitute for the internal live-link selection.

Remaining #418 work includes bounded archive verification, complete source/raw
measurement replay, reviewed host and variance policy, externally anchored
threshold authority, fresh held-out measurements, resource-owner evidence and
the final supported-platform closure report. Existing M4 schemas and archive
bounds are unchanged. Unit fixtures are synthetic metadata, not hosted proof.

GitHub can clear `run.pull_requests` after PR closure. An empty association still
requires the exact selected PR number in the same-run reusable execution ref and
the immutable execution commit with its ordered base/candidate Git parents.
Nonempty associations retain their complete existing checks.
