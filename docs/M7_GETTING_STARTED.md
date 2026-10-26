# Bounded generics: first source-checkout programs

Run these commands from a Zryna source checkout after its documented setup. Use the built
`zryna` executable, or replace `zryna` with `cargo run --locked -p zryna --`.
JavaScript and WebAssembly execution requires an explicitly selected Node.js 22.22.1
executable. Set `NODE` to its absolute path. Native execution requires Linux x86-64 and
the documented system GNU toolchain. Windows supports JavaScript/WebAssembly execution
and emission of the Linux native object.

Create `examples/generics` for the files below. Use a fresh `--name` for every command;
existing output is never overwritten.

### scalar-clone

Save this complete file as `examples/generics/main.zry`:

```zry
export function entry(): i32 {
  const value: i32 = 7;
  const copied: i32 = clone(value);
  return value + copied;
}
```

Build all three ordinary target artifacts with the native frontend:

```sh
zryna build examples/generics/main.zry --profile bounded-generics-v1 --target all --name generics-build-1
```

Run the scalar wrapper:

```sh
zryna run examples/generics/main.zry --profile=bounded-generics-v1 --target javascript --export entry --node "$NODE" --name generics-run-1 --json
```

The result is `i32:14`. The bundle contains `zryna-generic-manifest-v1.json` and the
selected ordinary target artifacts. A run result describes the completed invocation;
its private host transport is not an additional source export.

### module-entry

Save this complete file as `examples/generics/a.zry`:

```zry
export function identity<T extends ZrynaValue>(value: T): T {
  return value;
}
```

Save this complete file as `examples/generics/main.zry`:

```zry
import { identity } from "./a.zry";
export function entry(): i32 {
  return identity<i32>(7);
}
```

```sh
zryna run examples/generics/main.zry --profile bounded-generics-v1 --target webassembly --export entry --node "$NODE" --name generics-module-1 --json
```

The result is `i32:7`. The imported template is visible to source compilation. Only
`entry` is a callable scalar export; selecting `identity` or `identity<i32>` as a
host export fails with `ZRYNA-M7005` at the original function name. Select `entry` to
correct that request. Explicit relative `.zry` imports retain the selected main module
even when another module sorts first.

### mixed-catalogue

Save this complete file as `examples/generics/main.zry`:

```zry
interface Node<T extends ZrynaValue> extends ZrynaEnum { leaf: T; branch: Vec<Node<T>>; }
interface Pair<T extends ZrynaValue> extends ZrynaStruct { value: T; }
interface Signal extends ZrynaEnum { quiet: ZrynaNone; yes: String; }
export function entry(): i32 {
  const leaf: Node<String> = Node.leaf<String>("n");
  const children: Vec<Node<String>> = Vec<Node<String>>([leaf]);
  const recursiveRoot: Node<String> = Node.branch<String>(children);
  { const root0: bool = true; const loan0: Borrow<bool> = borrow(root0); const copied0: bool = clone(loan0); }
  { const root1: i32 = 7; const loan1: Borrow<i32> = borrow(root1); const copied1: i32 = clone(loan1); }
  { const root2: String = "text"; const loan2: Borrow<String> = borrow(root2); const copied2: String = clone(loan2); }
  { const root3: Pair<String> = Pair<String>({ value: "field" }); const loan3: Borrow<Pair<String>> = borrow(root3); const copied3: Pair<String> = clone(loan3); }
  { const root4: Signal = Signal.yes("payload"); const loan4: Borrow<Signal> = borrow(root4); const copied4: Signal = clone(loan4); }
  { const root5: Option<String> = Option.some<String>("option"); const loan5: Borrow<Option<String>> = borrow(root5); const copied5: Option<String> = clone(loan5); }
  { const root6: Result<String, i32> = Result.ok<String, i32>("result"); const loan6: Borrow<Result<String, i32>> = borrow(root6); const copied6: Result<String, i32> = clone(loan6); }
  { const root7: FixedArray<String, 2> = FixedArray<String, 2>(["a", "b"]); const loan7: Borrow<FixedArray<String, 2>> = borrow(root7); const copied7: FixedArray<String, 2> = clone(loan7); }
  { const root8: Vec<String> = Vec<String>(["element"]); const loan8: Borrow<Vec<String>> = borrow(root8); const copied8: Vec<String> = clone(loan8); }
  { const root9: Shared<String> = shared("shared"); const loan9: Borrow<Shared<String>> = borrow(root9); const copied9: Shared<String> = clone(loan9); }
  { const owner: Shared<String> = shared("weak"); const root10: Weak<String> = downgrade(owner); const loan10: Borrow<Weak<String>> = borrow(root10); const copied10: Weak<String> = clone(loan10); }
  const recursiveLoan: Borrow<Node<String>> = borrow(recursiveRoot);
  const recursiveCopy: Node<String> = clone(recursiveLoan);
  return 97;
}
```

```sh
zryna run examples/generics/main.zry --profile bounded-generics-v1 --target javascript --export entry --node "$NODE" --name generics-mixed-1 --json
```

The result is `i32:97`. This example keeps generic structs/enums, Option, Result,
arrays, vectors, shared owners and weak owners inside the program. Exported signatures
accept and return only `bool` and `i32`. Generic values do not cross the host ABI.

### corrected-opaque-clone

An opaque `ZrynaValue` type parameter does not promise the `Clone` capability. This
complete program is rejected with `ZRYNA-M7002` at `clone`, including when `forbidden`
is unused:

```zry
function forbidden<T extends ZrynaValue>(value: T): T {
  return clone(value);
}
export function entry(): i32 {
  return 7;
}
```

Move the value into the return instead of cloning it. Save this corrected complete
file as `examples/generics/main.zry`:

```zry
function forbidden<T extends ZrynaValue>(value: T): T {
  return value;
}
export function entry(): i32 {
  return 7;
}
```

```sh
zryna run examples/generics/main.zry --profile bounded-generics-v1 --target javascript --export entry --node "$NODE" --name generics-corrected-1 --json
```

The corrected result is `i32:7`.

For a wrapper with parameters, pass canonical arguments such as `--arg=i32:-7` and
`--arg=bool:true` in declaration order. Wrong count or type is rejected before execution.
`--provider bootstrap` and `--provider compare` also require `--node "$NODE"`; comparison
requires both verified frontend observations to agree.

This profile currently selects source-checkout builds. Standalone package projects,
installed distributions, components, WASI commands, servers and foreign-library imports
have separate admission contracts. See [the public profile reference](M7_PUBLIC_PROFILE.md).
