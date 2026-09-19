# Zryna Developer Preview

Thin local editor integration for Zryna's verified scalar diagnostics, definitions and formatter.
Lexical syntax highlighting and explicit **Zryna: Run Saved File** are included.
This is an initial **scalar-only** package. It does not format the compiler's M2 control-flow/modules
or M3 data/ownership profiles. Unsupported or incomplete input receives no edits.

## Installation

Build the matching reviewed compiler source with Node.js 22.22.1, pnpm 11.18.0 and Rust 1.97.1:

```text
pnpm install --frozen-lockfile
pnpm preflight
pnpm m0:check
cargo build --locked -p zryna-language-server
pnpm editor:check
pnpm editor:package
code --install-extension /absolute/compiler/checkout/.zryna/out/zryna-0.2.0.vsix
```

Set these **user settings** to absolute paths:

- `zryna.serverPath`: the built `target/debug/zryna-language-server` executable (`.exe` on Windows).
- `zryna.compilerRoot`: the trusted matching compiler checkout with its pinned adapter installed.
- `zryna.nodePath`: the exact Node.js 22.22.1 executable.
- `zryna.compilerPath`: the separately installed official v0.2.3 `zryna` executable for Run.
  Run uses the installation's runtime and does not pass source-checkout or Node overrides.

Open the edited project's folder and a `.zry` file. Use **Format Document**, **Format Selection** or
**Go to Definition**. The extension serves one active document at a time; changing files creates a
fresh connection. Range formatting accepts complete functions only and never expands a selection.
Formatting has one canonical two-space/LF style. Comments and token spellings remain unchanged.

## Compatibility

| Extension | Editor | Server |
| --- | --- | --- |
| 0.2.0 | VS Code-compatible API >=1.82.0 | Matching source-built `zryna-language-server` 0.2.3 with `scalar-format-v1` |
| 0.2.0 | Same | Public released v0.2.3 server is incompatible with formatting; the installed compiler supports Run |

The extension verifies the exact capability before sending source. A package-version match alone
is insufficient. No compiler binary release is included; a future distribution release is needed
for standalone installation. The package is VSIX-compatible with VS Code/Open VSX clients, but
marketplace publication requires a configured publisher and credentials and is still pending.

## Boundaries

The extension runs only in trusted local-file workspaces. Executable paths come exclusively from
user settings; workspace overrides are ignored. It downloads nothing,
provides no debugging commands, transmits no source over a network, and has no telemetry or
runtime dependencies. Compiler diagnostics remain inert plain text. Editor APIs apply explicitly
requested edits; the extension provides no general filesystem write service.

Formatting supports the existing protocol-v2 i32 function/return/reference/literal/addition slice.
Imports, parenthesized expressions, control flow, ownership/data syntax and classes remain
unsupported. Malformed/unavailable revisions, partial ranges, cancellation and stale documents
produce no edits. See the [language server and format contract](https://github.com/zryna/zryna/blob/main/docs/LANGUAGE_SERVER.md).

## Run and inspect output

Save the active `.zry` file, then choose **Zryna: Run Saved File** from the command palette or editor
title. Choose an exported function, JavaScript or WebAssembly, and each required i32 argument.
There is no default export or argument. Cancel any picker to stop without starting the compiler.
The progress notification supports cancellation. Unsaved/closed/virtual files and untrusted
workspaces are rejected. Opening, editing, formatting and saving never execute project code.

Run supports the installed compiler's one-file `i32-v1` profile: explicit i32 parameters/result,
integer literals, references and addition. The package source limit is **1024 UTF-8 bytes**.
Arguments must be canonical decimal integers between -2147483648 and 2147483647. The picker is
lexical discovery, not semantic validation; the compiler remains authoritative and may reject an
offered function or the complete program. Highlighting does not imply language/profile support.
M2/M3 programs, imports, bool arguments and native Windows executables are not supported by Run.

Each invocation uses a fresh compiler-created project in extension global storage under `runs`.
It copies saved bytes into `src/main.zry`, updates the generated manifest size/SHA-256, and invokes
the compiler package resolver to refresh the authenticated lock. It never adds helper programs to
the package. Original source and existing projects remain untouched. Each direct compiler process
has a 120-second timeout and 1 MiB captured-output limit.

**Zryna Run** output shows the saved path/hash, invocation, compiler results/diagnostics and actual
artifact location. Results refer to that snapshot even if the editor changes during execution.
**Zryna: Open Generated JavaScript** opens the last successful JavaScript artifact.
**Zryna: Reveal Run Output** selects the actual `.mjs` or `.wasm` file in the OS file explorer.
Output is under `<run-project>/.zryna/out/result.run/javascript/result.mjs` or
`webassembly/result.wasm`; manifest invocation, source identity and artifact hashes are checked.
Run state is retained for inspection, including failed/cancelled scaffolds; the output channel
identifies the project. Remove old run directories manually when no run is active.

This explicit execution scope follows #457 and supersedes #409's earlier no-execution boundary
only for this command. Formatter expansion and marketplace publication remain open in #409.
