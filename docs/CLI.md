# Zryna CLI reference

Status: implemented for the M1 `I32V1` build and run slice. The CLI is a thin request parser and
renderer over `zryna-driver`; it does not own compiler semantics or backend behavior.

## Commands

```text
zryna architecture check [--root <PATH>] [--json]
zryna doctor             [--root <PATH>] [--json]
zryna build <ENTRYPOINT> --target <javascript|webassembly|native|all> --node <PATH> [--root <PATH>] [--name <STEM>] [--json]
zryna run   <ENTRYPOINT> --target <javascript|webassembly|native|all> --export <NAME> --node <PATH> [--arg=i32:<VALUE> ...] [--root <PATH>] [--name <STEM>] [--json]
```

`architecture check` and `doctor` run the same mandatory fail-closed workspace gate. Every
`build` and `run` performs that gate before reading source, creating output, discovering a target
runtime, or doing target work. There is no bypass flag.
Architecture and doctor `--json` output retains the existing deterministic `ValidationReport`
shape containing its `diagnostics` array.

`ENTRYPOINT` is exactly one workspace-relative `.zry` path. Absolute paths, traversal,
normalized-away components, backslashes, links or Windows reparse points, paths outside `--root`,
and multiple source files are rejected. `--target` is mandatory, exact, lowercase, and has no
alias or default. `--root` defaults to the current directory; the driver requires its resolved
workspace root to be an absolute real directory. `--name` defaults to the entrypoint stem and must
be 1 to 128 ASCII letters, digits, underscores, or hyphens, begin with a letter or underscore, and
avoid Windows-reserved device names. `--node` is mandatory and supplies an absolute path to a real
regular executable, with no symbolic link or Windows reparse point, for the validated exact
Node.js 22.22.1 runtime used by the authenticated TypeScript frontend and JavaScript or WebAssembly
execution. A native-only build still needs Node for the frontend; it does not use Node as its
target runtime.

`run` requires one exact logical export. Each repeated `--arg` is a typed scalar written as
`i32:<VALUE>`. Negative shell arguments are most robust as `--arg=i32:-1`. The decimal spelling is
canonical: whitespace, a plus sign, leading zeroes other than `0`, numeric prefixes, fractions,
exponents, negative zero, and values outside `-2147483648` through `2147483647` are rejected.
Arity and types are checked through the program's sealed scalar ABI before target execution.
Boolean source and invocation remain rejected by the current `I32V1` profile.

## Targets

| Selection | `build` artifact | `run` artifact and execution | Supported host |
| --- | --- | --- | --- |
| `javascript` | deterministic `.mjs` | sealed module through Node.js | Linux, Windows |
| `webassembly` | validated import-free `.wasm` | direct standard WebAssembly API through Node.js | Linux, Windows |
| `native` | audited Linux x86-64 `.o` | invocation-specific audited `.elf` | Linux x86-64 for run |
| `all` | `.mjs`, `.wasm`, `.o` | `.mjs`, `.wasm`, `.elf` | Linux x86-64 for run |

`build native` emits a relocatable object and does not invent `main`. `run native` generates the
sealed scalar-ABI harness for the requested invocation. `all` analyzes source exactly once,
constructs one `VerifiedProgram`, and dispatches that authority in the fixed order JavaScript,
WebAssembly, native. All run targets receive the same verified invocation and report ordered typed
observations. The repository's [M1 conformance suite](M1_CONFORMANCE.md) compares those public
observations with fixed expected values and the committed manifest without creating a second
runtime semantics authority.

## Output bundles

The public output root is `<root>/.zryna/out`:

```text
.zryna/out/<stem>.build/
  zryna-manifest-v1.json
  javascript/<stem>.mjs
  webassembly/<stem>.wasm
  native/<stem>.o

.zryna/out/<stem>.run/
  zryna-manifest-v1.json
  javascript/<stem>.mjs
  webassembly/<stem>.wasm
  native/<stem>.elf
```

Only selected target paths exist. Build and run bundles with the same stem may coexist. A second
command of the same kind is create-only and fails without changing the existing bundle. The Linux
native run artifact is mode `0755`; Windows native run is unsupported.

The driver writes and synchronizes every selected artifact, result, and manifest inside one new
sibling transaction directory. On Unix it sets the transaction and target directories to mode
`0700`. On Windows those directories inherit ACLs from the validated compiler-owned output root,
so the workspace and `.zryna/out` must already be private to the invoking principal. The driver
revalidates containment and commits the complete bundle with one create-only, same-filesystem
directory rename to the absent final path. A failure leaves no advertised bundle. It never replaces
or modifies a pre-existing path.

The successful rename is the commit point; directory-entry crash durability after that point is
not claimed.

`zryna-manifest-v1.json` is deterministic UTF-8 JSON with `version` equal to `1` and `profile`
equal to `zryna-m1-cli-v1`. Its top-level fields are `version`, `profile`, `command`, `entrypoint`,
`source_sha256`, `stem`, `targets`, `artifacts`, `invocation`, `results`, and `diagnostics`.
`source_sha256` is the lowercase SHA-256 digest of the exact source bytes. Targets, artifacts,
results, and diagnostics are in stable order. Each artifact records its `target`, `kind`, bundle-
relative `/`-separated `path`, `bytes`, and lowercase `sha256`. Artifact kinds are
`ecmascript-module`, `core-webassembly-module`, `linux-x86-64-relocatable-object`, and
`linux-x86-64-invocation-executable` as applicable. `invocation` is `null` for `build`; for `run`
it records `export` plus ordered arguments as `{ "type": "i32", "value": n }`. `results` is empty
for build and records each run target and its typed `outcome` in target order. The manifest contains
no absolute or temporary path, timestamp, process id, inherited environment value, credential, or
raw external-tool output.

## Output and exit status

Without `--json`, a successful build prints the committed manifest path. A successful run prints
one ordered `<target>: i32 <value>` observation per selected target. Non-fatal diagnostics go to
standard error and use their stable Zryna codes. With `--json`, every parsed build or run command
emits exactly one versioned JSON document containing `version`, `ok`, `command`, the portable
workspace-relative committed `manifest` path (or `null` on failure), ordered `results`, and
`diagnostics`. Machine
consumers must use its structured fields rather than parse text output.

| Status | Meaning |
| ---: | --- |
| `0` | complete success and bundle commit |
| `1` | workspace or mandatory architecture failure |
| `2` | CLI syntax or request validation failure |
| `3` | frontend, source, semantic, scalar ABI, or IR rejection |
| `4` | target preparation, tool, audit, transaction, or commit failure |
| `5` | execution or result-frame failure |
| `6` | cleanup could not be confirmed |

Stable diagnostic codes are the precise failure identity. Clap parse errors retain status `2`.
No failure status advertises a partial bundle.

## Runtime and process limits

The JavaScript and WebAssembly target host validates that `node --version` exits successfully,
writes exactly `v22.22.1\n` to standard output, and writes nothing to standard error. The runtime
path is inspected before and after discovery and again before execution. Zryna passes literal
argument vectors and never invokes a shell.

Each Node probe or target execution has a five-second hard deadline followed by a separate bounded
five-second cleanup deadline. Standard input is closed. A run result is exactly four
standard-output bytes and standard error must be empty; version output is capped at 64 bytes and
target standard error at 16 KiB. Unix runs in a new process group with a cleared environment
containing only `LANG=C`, `LC_ALL=C`, and `TZ=UTC`. Windows uses a kill-on-close job with a cleared
environment retaining only the required `SystemRoot` and `WINDIR` values. Windows requires the
job-wide termination request to succeed and the leader to be reaped before the deadline. Timeout,
overflow, abnormal exit, or malformed result framing fails without a bundle; inability to complete
the documented cleanup steps uses exit status `6`.

Native discovery, linking, and execution use the separate hard caps and controlled environment in
the [native executable contract](../spec/native-semantics/EXECUTABLE.md). Generated artifacts are
not a security sandbox, and the dynamically linked native executable requires the validated host's
CRT, libc, and loader.

## Examples

From the workspace root, using an exact Node.js 22.22.1 executable:

```bash
cargo run --locked -p zryna -- build examples/universal/add.zry --target javascript --name add-js --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target webassembly --name add-wasm --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target native --name add-native --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target all --name add-all --node /absolute/path/to/node

cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name add-js --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target webassembly --name add-wasm --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target native --name add-native --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target all --name add-all --export add --arg=i32:2147483647 --arg=i32:1 --node /absolute/path/to/node --json
```

On Windows, pass the absolute direct executable path, for example
`--node C:\Tools\node-v22.22.1\node.exe`; a PATH shim, symbolic link, or reparse point is rejected.

The last invocation reports three ordered `i32` observations with value `-2147483648`; the checked
M1 differential suite requires those observations and the manifest to agree. Package resolution,
multiple source files, watch mode, incremental or remote builds, Boolean execution, browser
execution, WASI, Windows or macOS native execution, static native executables, overwrite behavior,
and runtime-enforced cross-target comparison remain outside the current slice.
M1 closure evidence includes website publication of versioned status and reference data from the
authenticated compiler documentation bundle tracked in Issue #21.
