# Zryna CLI

Fail-closed command-line entrypoint for architecture checks and the M1 `I32V1` compiler slice.

The CLI provides `architecture check`, `doctor`, and explicit `build` and `run` commands for
`javascript`, `webassembly`, `native`, and `all`. Build and run require one workspace-relative
`.zry` entrypoint, one explicit target, and an exact Node.js 22.22.1 executable. Run additionally
requires one scalar-ABI export and canonical repeated `--arg=i32:<VALUE>` arguments. Boolean
execution remains profile-gated.

Every compiler command runs architecture validation first and uses one verified program for all
selected backends. Complete create-only bundles are committed below `.zryna/out`; `all` reports
ordered results. The repository-owned [M1 conformance suite](../../docs/M1_CONFORMANCE.md) compares
those public observations with fixed expected values and each other.

See the [complete CLI reference](../../docs/CLI.md) for syntax, target and platform limits, bundle
and manifest layout, atomic publication, examples, JSON behavior, and stable exit statuses.
