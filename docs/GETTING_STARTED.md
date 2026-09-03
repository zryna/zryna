# Run your first Zryna programs

Zryna is experimental. This walkthrough uses the existing public M1 scalar and M2 control-flow
profiles, not the internal M3 ownership profile. Run commands from the compiler repository root.
You do not need to run the contributor regression suite after every program edit.

## Prepare the checkout

Install Rust **1.97.1** (the repository pins it), pnpm **11.18.0**, and Node.js **22.22.1**.
Then install the locked frontend dependencies:

```bash
pnpm install --frozen-lockfile
```

In every command below, replace `/absolute/path/to/node` with your real Node executable.
It must be an absolute path to a regular executable, not a PATH shim, symbolic link or Windows
reparse point. Even native compilation needs Node for the frontend. Quote paths containing spaces.
Source paths, unlike the Node path, must be workspace-relative and use `/` separators.

The first `cargo run` builds the compiler and can take time. Subsequent invocations reuse it.
Every build/run checks the workspace architecture first; there is no bypass flag.

## Add two numbers

Open `examples/universal/add.zry` in your checkout ([repository source](https://github.com/zryna/zryna/blob/main/examples/universal/add.zry)):

```typescript
export function add(a: i32, b: i32): i32 {
  return a + b;
}
```

The function takes two explicitly typed signed 32-bit integers and returns their sum. `export`
makes it callable through the CLI. First **build** a JavaScript module without invoking it:

```bash
cargo run --locked -p zryna -- build examples/universal/add.zry --target javascript --name hello-add --node /absolute/path/to/node
```

Then **run** the export with typed arguments:

```bash
cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name hello-add --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
```

The result is `i32:42`. `i32` arithmetic wraps at signed 32-bit boundaries; it is not arbitrary-
precision arithmetic. This example needs no `--profile`: omission selects M1.

Inspect the generated module and manifest:

| Command | Committed files |
| --- | --- |
| build | `.zryna/out/hello-add.build/javascript/hello-add.mjs` and `zryna-manifest-v1.json` in that build directory |
| run | `.zryna/out/hello-add.run/javascript/hello-add.mjs` and `zryna-manifest-v1.json` in that run directory |

The manifest records the source hash, selected artifacts and, for run, typed invocation/results.
Build and run bundles may share a name because their directories differ. These are compiler
results, not a general-purpose console I/O facility.

## Choose a branch and call another module

`examples/control-flow/main.zry` ([source](https://github.com/zryna/zryna/blob/main/examples/control-flow/main.zry)) imports `double` from
`math.zry` ([source](https://github.com/zryna/zryna/blob/main/examples/control-flow/math.zry)). `choose` doubles its integer when `enabled` is true
and otherwise returns the integer unchanged.

Select M2 explicitly to enable Boolean arguments, relative imports and this control flow:

```bash
cargo run --locked -p zryna -- run examples/control-flow/main.zry --profile control-flow-v1 --target javascript --name hello-choose-true --export choose --arg=bool:true --arg=i32:21 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/control-flow/main.zry --profile control-flow-v1 --target javascript --name hello-choose-false --export choose --arg=bool:false --arg=i32:21 --node /absolute/path/to/node
```

The results are `i32:42` and `i32:21`, respectively. Each `.run` directory contains its JavaScript
module and **`zryna-manifest-v2.json`**; M2 records both source modules and their import graph.
Omitting the profile does not automatically select M2. See the
[accepted control flow](M2_CONTROL_FLOW_SEMANTICS.md#accepted-control-flow) and
[module rules](M2_MODULE_CLOSURE.md) before extending the example.

## Fix a rejected invocation

This deliberately omits the second argument to `add`:

```bash
cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name hello-arity --export add --arg=i32:20 --node /absolute/path/to/node
```

It fails with `ZRYNA-B2102` (exit status 3) before execution because the export requires two
arguments; it publishes no run bundle.
Supply the missing argument to correct it:

```bash
cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name hello-arity --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
```

Arguments must match the declared types too: `bool:true` is not an `i32`. For negative integers,
use the unambiguous form `--arg=i32:-1`. The [CLI reference](CLI.md) defines canonical spelling,
diagnostics and exit statuses.

## Edit and run again without overwriting output

Repeating the successful `hello-add` run above deliberately fails: its `.run` directory already
exists (`ZRYNA-C1009`, exit status 4). Output publication is **create-only**, even if the source
and arguments are unchanged.
The old module and manifest are preserved. There is no overwrite or watch flag.

After changing an argument or editing a supported expression in your local example, choose a fresh
name. For example, this invocation needs no source edit:

```bash
cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name hello-add-again --export add --arg=i32:1 --arg=i32:2 --node /absolute/path/to/node
```

It returns `i32:3`. On a second pass through the walkthrough, choose unused names throughout.
Do not reuse an M1 bundle name for an M2 request of the same command kind either.

## Targets and limits

This walkthrough's invocations were checked on Linux x86-64 with Node.js 22.22.1. Values above are
observed typed results, not copied terminal transcripts. No Windows execution is claimed here.
The authoritative [target/platform table](CLI.md#targets) documents JavaScript and core WebAssembly
on Linux/Windows. `webassembly` selects a `.wasm` module executed through Node, not a browser app.
Native object output targets Linux x86-64; native run (and therefore `all` run) requires Linux
x86-64 and the documented GNU toolchain. There is no automatic fallback on unsupported hosts.
See [the existing alternative-target commands](CLI.md#examples) and platform prerequisites before
using those targets; this walkthrough does not validate a new platform configuration.

M2 supports the documented scalar subset, not all TypeScript. Packages, general I/O, browser/WASI
integration, watch mode and editor tooling are not supplied by these examples. Public
`--profile data-ownership-v1` tutorials must wait for the M3 conformance/activation gates #89/#90;
internal ownership tests do not enable that CLI profile. Consult the [roadmap](ROADMAP.md) for
those separate capabilities.
