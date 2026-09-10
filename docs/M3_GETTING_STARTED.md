# Your first M3 programs

Zryna v0.1.0 is an experimental, source-based Developer Preview. It is not
production-ready. It provides the documented repository-local M1–M3 workflows; it does not
include standalone binaries, package installation, or stable compatibility guarantees. The
`v0.1.0` tag and pre-release are not yet published. This guide uses exact public
`--profile data-ownership-v1` and scalar observations. Owned values remain inside Zryna; a String
example does not print a string. Run commands from the compiler repository root. See
[public boundaries](M3_PUBLIC_PROFILE.md) and the
[preview policy](DEVELOPER_PREVIEW.md).

## Setup

Follow the [checkout and tool installation steps](GETTING_STARTED.md#prepare-the-checkout):
Git, Rust 1.97.1, pnpm 11.18.0 and exact Node.js 22.22.1. Then run:

```sh
pnpm install --frozen-lockfile
cargo build --locked -p zryna
```

In Bash, set `NODE` to the absolute direct Node.js 22.22.1 executable, for example
`NODE="$(node -p "process.execPath")"`; verify `"$NODE" --version` prints `v22.22.1`.
On PowerShell, use `$NODE = node -p "process.execPath"` and `& $NODE --version`.
Create `.zryna/cache/m3-guide` (`mkdir -p .zryna/cache/m3-guide` in Bash, or
`New-Item -ItemType Directory -Force .zryna/cache/m3-guide` in PowerShell).
This compiler-owned scratch directory avoids adding undeclared workspace-root files.

Each example below gives the complete `main.zry` source and, where imported, `math.zry`.
Save them in `.zryna/cache/m3-guide`. Each command works in Bash and PowerShell after setting
`NODE`. JavaScript and WebAssembly runs are supported on Linux x86-64 and Windows x64.
Replace `--target javascript` with `--target webassembly` and choose a fresh `--name` to
observe the same value. On Linux x86-64 with the documented GNU toolchain, `--target all`
runs JavaScript, WebAssembly and native together. Windows `all` runs reject with `ZRYNA-N4002`.

## First program: a Pair

Save the following Pair source, then build it before running:

```sh
cargo run --locked -p zryna -- build .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-pair-build-1 --node "$NODE"
```

The build prints `.zryna/out/m3-pair-build-1.build/zryna-manifest-v3.json`. The bundle contains
the manifest and `javascript/m3-pair-build-1.mjs`. The manifest binds source hashes, both
layout fingerprints, runtime ABI identity and artifact hashes.

### pair: Struct fields and arithmetic

`main.zry`:

```zry
interface Pair extends ZrynaStruct { left: i32; right: i32; }
export function score(left: i32, right: i32): i32 { const pair: Pair = Pair({ left, right }); return pair.left * 31 + pair.right; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-pair-run-1 --export score --arg=i32:2 --arg=i32:3 --node "$NODE"
```

Expected output: `javascript: i32 65`.

### array: Fixed arrays

`main.zry`:

```zry
export function score(): i32 { const values: FixedArray<i32, 3> = FixedArray<i32, 3>([7, 11, 19]); return values[0] * 100 + values[1] * 10 + values[2]; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-array-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 829`.

### enum: Exhaustive enum matching

`main.zry`:

```zry
import { score as enumScore } from "./math.zry";
export function score(value: i32): i32 { return enumScore(value); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
interface Choice extends ZrynaEnum { none: ZrynaNone; value: i32; }
export function score(value: i32): i32 { const selected: Choice = Choice.value(value); return match(selected, { "Choice.none": () => 0, "Choice.value": (item) => item * 3 }); }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-enum-run-1 --export score --arg=i32:11 --node "$NODE"
```

Expected output: `javascript: i32 33`.

### string: String clone, move, replacement and concatenation

`main.zry`:

```zry
import { text } from "./math.zry";
export function score(): i32 { return text(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
export function text(): i32 { let first: String = "héllo"; const copy: String = clone(first); const moved: String = copy; first = "new"; const joined: String = concat(first, moved); return 17; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-string-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 17`.

### vec: Vec push, clone and indexing

`main.zry`:

```zry
function values(): i32 { let items: Vec<i32> = Vec<i32>([7, 9]); push(items, 13); const copied: Vec<i32> = clone(items); return copied[2]; }
export function score(): i32 { return values(); }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-vec-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 13`.

### owned-aggregate: Owned struct clone and cleanup

`main.zry`:

```zry
import { aggregate } from "./math.zry";
export function score(): i32 { return aggregate(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
interface TextPair extends ZrynaStruct { left: String; right: String; }
export function aggregate(): i32 { const pair: TextPair = TextPair({ left: "left", right: "right" }); const copied: TextPair = clone(pair); const moved: TextPair = copied; return 29; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-owned-aggregate-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 29`.

### borrow: Lexical exclusive borrow

`main.zry`:

```zry
function write(): i32 {
  let root: i32 = 7;
  {
    const alias: BorrowMut<i32> = borrowMut(root);
    const before: i32 = alias;
    alias = 9;
    const after: i32 = alias;
  }
  return root;
}

export function score(): i32 { return write(); }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-borrow-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 9`.

### handles: Shared and Weak clone and release

`main.zry`:

```zry
import { handles } from "./math.zry";
export function score(): i32 { return handles(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
export function handles(): i32 { const owner: Shared<i32> = shared(7); const copy: Shared<i32> = clone(owner); const weak: Weak<i32> = downgrade(copy); const weakCopy: Weak<i32> = clone(weak); return 23; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-handles-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 23`.

### weak-live: Weak upgrade: live

`main.zry`:

```zry
import { check } from "./math.zry";
export function score(): i32 { return check(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
export function check(): i32 { const owner: Shared<i32> = shared(7); const weak: Weak<i32> = downgrade(owner); upgradeWeak(weak, (value) => { return 31; }, () => { return 37; }); }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-weak-live-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 31`.

### weak-expired: Weak upgrade: expired

`main.zry`:

```zry
import { check } from "./math.zry";
export function score(): i32 { return check(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
function expired(): Weak<i32> { const owner: Shared<i32> = shared(7); return downgrade(owner); }
export function check(): i32 { const weak: Weak<i32> = expired(); upgradeWeak(weak, (value) => { return 31; }, () => { return 37; }); }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-weak-expired-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 37`.

### owned-vec: Owned Vec cleanup

`main.zry`:

```zry
import { aggregate } from "./math.zry";
export function score(): i32 { return aggregate(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
export function aggregate(): i32 { const values: Vec<String> = Vec<String>(["left", "right"]); const copied: Vec<String> = clone(values); return 41; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-owned-vec-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 41`.

### owned-shared: Shared owned payload cleanup

`main.zry`:

```zry
import { aggregate } from "./math.zry";
export function score(): i32 { return aggregate(); }
```

`math.zry` (the imported dependency; its internal exports are not public entry exports):

```zry
export function aggregate(): i32 { const owner: Shared<String> = shared("payload"); const copied: Shared<String> = clone(owner); return 43; }
```

Run once with a fresh name:

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-owned-shared-run-1 --export score  --node "$NODE"
```

Expected output: `javascript: i32 43`.

The String/owned aggregate examples return scalar sentinels after the owned operations. The
separate conformance fault corpus checks allocation failures and exact cleanup order; a sentinel
alone does not prove cleanup. The live Weak example takes the success arm (`31`); the expired
example returns a weak handle after its last strong owner is dropped and takes the expired arm
(`37`). Overflow is a typed refcount trap tested with bounded private injection; do not attempt
host exhaustion. Lexical borrow aliases end at the closing block and cannot escape.

## Safe create-only rerun

Running the same command again with the same `--name` fails and preserves the existing bundle.
Choose a fresh name, for example `--name m3-pair-run-2`, to retain both results. Build and run
names use separate `.build` and `.run` directories. If you intentionally want to recreate only
that generated Pair run, first inspect `.zryna/out/m3-pair-run-1.run`, remove only that known
bundle, and repeat the original command. Never delete source files or the whole output tree
merely to retry one program. Every example is tested through the same create-only public route.

## Common rejected programs and corrections

### Use after move

Rejected private function:

```zry
function invalid(): String { const text: String = "owned"; const moved: String = text; return text; }
```

`text` was consumed by the move (`ZRYNA-M3011`). Return `moved`, or explicitly clone `text`
when initializing `moved` so `text` remains owned. The String example above demonstrates the
valid explicit-clone-then-move pattern in a complete executable two-module program.

### Conflicting borrows

Complete rejected `main.zry`:

```zry
function bad(): i32 {
  let root: i32 = 7;
  {
    const exclusiveAlias: BorrowMut<i32> = borrowMut(root);
    const sharedAlias: Borrow<i32> = borrow(root);
    exclusiveAlias = 9;
    const copy: i32 = sharedAlias;
  }
  return root;
}
```

A `Borrow<i32>` created while an overlapping `BorrowMut<i32>` is active rejects with
`ZRYNA-M3017`. Keep the aliases in separate lexical blocks. The complete `borrow` example
above uses one exclusive alias and reads the owner only after that alias ends.

### Missing name

```zry
export function score(): i32 { return missing; }
```

This rejects with `ZRYNA-M3002`. Declare an exact typed local first, or return an in-range literal.

### Owned public signature

```zry
export function bad(value: String): String { return value; }
```

Entry exports must use the scalar ABI. Move the owned work to a private function or an imported
dependency and expose an `i32`/`bool` result, as in the complete String example. Do not erase the
type annotation, use `any`, or reinterpret a pointer to bypass the restriction.

### Bounds are a typed runtime trap

The Vec example reads index `2` after pushing a third value. Reading index `2` from a two-element
Vec is source-valid but traps with `zryna.trap.bounds-v1`; use an in-range index or push the missing
element first. Add `--json` to inspect the exact `results[].outcome` record. A complete trapped
run exits 0 and publishes manifest v3; `ok: true` does not mean the program returned a scalar.

## Executable corrections

These complete replacements use the same workspace paths and tool setup as the examples above.

### corrected-move

This fixes use-after-move by cloning explicitly before transfer, keeping `text` owned. Save as
`main.zry`:

```zry
import { corrected } from "./math.zry";
export function score(): i32 { return corrected(); }
```

Save as `math.zry`:

```zry
export function corrected(): i32 { const text: String = "owned"; const moved: String = clone(text); const joined: String = concat(text, moved); return 17; }
```

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-corrected-move-1 --export score --node "$NODE"
```

Expected: `javascript: i32 17`. This also shows the correction for the owned public signature:
keep owned work in the dependency and expose a scalar entry wrapper. The two-module String example
above is another complete valid form. The lexical `borrow` example is the complete correction for
conflicting borrows: one exclusive alias ends before owner access.

### corrected-name

Replace `main.zry` with this complete source:

```zry
export function score(): i32 { const value: i32 = 7; return value; }
```

```sh
cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-corrected-name-1 --export score --node "$NODE"
```

Expected: `javascript: i32 7`. No `math.zry` import is needed.

## Continue safely

M3 adds no public owned ABI, user-defined generics, escaping references, tracing GC, raw pointers,
FFI, threads, WASI/Components, Windows native execution or production certification. Unsupported
syntax and ownership combinations fail closed. The [public profile](M3_PUBLIC_PROFILE.md) gives
host/toolchain and resource boundaries; [conformance](M3_CONFORMANCE.md) describes the independent
oracles and exact-limit proofs. The [M1/M2 guide](GETTING_STARTED.md) remains valid unchanged.
