# Release notes

## 0.2.0

- Add lexical syntax highlighting.
- Add explicit saved-source scalar Run with export/target selection and validated i32 arguments.
- Show compiler results/errors and open generated JavaScript or reveal actual Wasm/output.
- Preserve trusted-workspace and user-level executable settings; never run on open/save.

## 0.1.0 — initial local preview

- Scalar diagnostics and Go to Definition through the compiler-owned language server.
- Deterministic document and complete-function range formatting for scalar-format-v1.
- Explicit user-configured local compiler/runtime paths and capability compatibility checks.
- Cancellation, stale-result rejection, bounded transport and inert diagnostic rendering.

This package requires the matching source build; public v0.2.3 binaries do not provide formatting.
M2/M3 formatting and marketplace publication remain pending. A built or installed VSIX is not a
marketplace release. No new compiler tag or binary release accompanies this package.
