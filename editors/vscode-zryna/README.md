# Zryna Developer Preview

Thin local editor integration for Zryna's verified scalar diagnostics, definitions and formatter.
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
code --install-extension /absolute/compiler/checkout/.zryna/out/zryna-0.1.0.vsix
```

Set these **user settings** to absolute paths:

- `zryna.serverPath`: the built `target/debug/zryna-language-server` executable (`.exe` on Windows).
- `zryna.compilerRoot`: the trusted matching compiler checkout with its pinned adapter installed.
- `zryna.nodePath`: the exact Node.js 22.22.1 executable.

Open the edited project's folder and a `.zry` file. Use **Format Document**, **Format Selection** or
**Go to Definition**. The extension serves one active document at a time; changing files creates a
fresh connection. Range formatting accepts complete functions only and never expands a selection.
Formatting has one canonical two-space/LF style. Comments and token spellings remain unchanged.

## Compatibility

| Extension | Editor | Server |
| --- | --- | --- |
| 0.1.0 | VS Code-compatible API >=1.82.0 | Matching source-built `zryna-language-server` 0.2.3 with `scalar-format-v1` |
| 0.1.0 | Same | Public released v0.2.3 binaries are incompatible: no formatting capability |

The extension verifies the exact capability before sending source. A package-version match alone
is insufficient. No compiler binary release is included; a future distribution release is needed
for standalone installation. The package is VSIX-compatible with VS Code/Open VSX clients, but
marketplace publication requires a configured publisher and credentials and is still pending.

## Boundaries

The extension runs only in trusted local-file workspaces. Executable paths come exclusively from
user settings; workspace overrides are ignored. It downloads nothing, executes no project code,
provides no debugging/build commands, transmits no source over a network, and has no telemetry or
runtime dependencies. Compiler diagnostics remain inert plain text. Editor APIs apply explicitly
requested edits; the extension provides no general filesystem write service.

Formatting supports the existing protocol-v2 i32 function/return/reference/literal/addition slice.
Imports, parenthesized expressions, control flow, ownership/data syntax and classes remain
unsupported. Malformed/unavailable revisions, partial ranges, cancellation and stale documents
produce no edits. See the [language server and format contract](https://github.com/zryna/zryna/blob/main/docs/LANGUAGE_SERVER.md).
