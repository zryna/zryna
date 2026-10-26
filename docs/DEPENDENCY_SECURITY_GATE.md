# Dependency advisory gate

The dependency gate inventories every registry package in `Cargo.lock` and
`pnpm-lock.yaml`, captures an OSV advisory snapshot, and checks the reviewed MSAL
backport when its original version-range finding is present. It runs on Linux
and Windows in the Dependency security workflow.

Install the frozen workspace first. The gate requires Python 3.11 or newer on
Linux and Python 3.12 or newer on Windows, Node 22.22.1, and the workspace's
selected YAML parser. Supply canonical absolute
paths and a fresh evidence directory outside the checkout:

```text
pnpm security:dependencies --root /absolute/checkout --node /absolute/node --output /absolute/private/new-capture
```

The command reads dependencies and executes fixed local regression checks. Its
output directory is create-only. Keep failed captures; a diagnosed new attempt
uses a new directory. Standard proxy configuration and TLS trust apply.

`PRE_QUERY_FREEZE.json` records the exact inputs, tools and queries before the
requests. Raw responses, acquisition status, advisory details and SDK test
output remain available alongside `REPORT.json`. Exit 0 means the observed
dependency gate passed; exit 1 means unresolved advisories; exit 2 means invalid
inputs, acquisition or regression evidence. Missing, inconsistent, paginated or
unsupported responses fail the gate.

File reads compare stable named and opened identities. On Windows, matching
creation timestamps use `st_birthtime_ns`; named and descriptor `st_ctime_ns`
have different meanings. Each API's full before/after mode and timestamps remain
checked. The only mode translation is CPython's filename-derived execute bits
for `.exe`, `.bat`, `.cmd`, and `.com`; other identity changes reject.

## Reviewed MSAL source fix

The packaging dependency still reports version `@azure/msal-node@6.0.1`. The
finding `GHSA-xcx6-jx8p-x3h5` remains in the raw advisory results. Its source fix
is backported from upstream commit
`f550958586475dbfeb1cd44be14a2d653728850e` through the pinned pnpm patch
[msal-node-6.0.1-auth-code.patch](../scripts/dependency-patches/msal-node-6.0.1-auth-code.patch).

The sole accepted backport disposition requires the exact reviewed advisory,
patch and seven installed source/artifact afterimages, plus an actual execution
of the eleven CJS/ESM security controls. Material checks run before and after
that execution. Repository line-ending variants are explicitly pinned. The
known batch/detail timestamp precision pair is also fixed to the reviewed
advisory; changed timestamps or advisory content require review.

Any other finding remains unresolved. There is no advisory-ignore option or
caller-supplied pass record. The SDK controls use synthetic inputs with network
and authentication subprocesses denied; credentialed publishing and other Azure
authentication flows have separate qualification requirements.

## Scope

The report always retains `sourceAuthenticated: false`,
`candidateSecurityQualified: false`, `closureStatus: blocked`, and null
performance thresholds. Registered workspace packages are listed separately
from registry advisories. First-party security, OS/SDK/provider advisories,
runtime isolation, performance, reproduction and final source/host
authentication have their own gates. This command alone cannot close #418 or
qualify a release.
