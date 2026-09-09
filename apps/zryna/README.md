# Zryna CLI

Fail-closed command-line entrypoint for architecture checks and the M1 `I32V1` compiler slice.

The CLI provides `architecture check`, `doctor`, and explicit `build` and `run` commands for
`javascript`, `webassembly`, `native`, and `all`. `build` additionally accepts the default-M1-only
`component` selection. Build and run require one workspace-relative
`.zry` entrypoint, one explicit target, and an exact Node.js 22.22.1 executable. Run additionally
requires one scalar-ABI export and canonical repeated `--arg=i32:<VALUE>` arguments. Boolean
execution remains profile-gated.

`component` publishes one audited, import-free Component Model artifact and its manifest. It is
not included in `all`, cannot be combined with an explicit profile, and cannot be selected by
`run`; no component host or browser/WASI execution is activated.

Every compiler command runs architecture validation first and uses one verified program for all
selected backends. Complete create-only bundles are committed below `.zryna/out`; `all` reports
ordered results. The repository-owned [M1 conformance suite](../../docs/M1_CONFORMANCE.md) compares
those public observations with fixed expected values and each other.

Exact `--profile data-ownership-v1` selects the shared conformant M3 driver and manifest v3.
Public observations remain typed `i32`/`bool`; owned values stay internal. See the
[public M3 contract](../../docs/M3_PUBLIC_PROFILE.md).

See the [complete CLI reference](../../docs/CLI.md) for syntax, target and platform limits, bundle
and manifest layout, atomic publication, examples, JSON behavior, and stable exit statuses.
