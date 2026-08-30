# Universal examples

Small programs used to prove behaviorally equivalent JavaScript, WebAssembly, and native
artifacts one governed slice at a time.

`add.zry` currently passes the authenticated frontend, strict semantics, verified `I32V1` IR,
direct JavaScript pipeline, and direct core WebAssembly pipeline. Its generated `.mjs` module is
imported under Node.js and checked for ordinary values, signed 32-bit wrapping boundaries, strict
host carriers, exact arity, and stable exports. Its generated import-free `.wasm` module is
validated, inspected, instantiated, and executed through Node's standard WebAssembly API with the
same wrapping boundaries. It also emits a separately published audited Linux x86-64 `.o`. For an
invocation executable, the driver emits another audited object in memory, links it with one typed
generated harness, audits and create-only publishes `.elf`, then executes its retained sealed
snapshot under bounded process controls. Ordinary and signed 32-bit wrapping values travel through
the full-width four-byte result channel.

The public CLI now exposes this exact `I32V1` slice. From the workspace root, replace the Node path
with an exact Node.js 22.22.1 executable:

```bash
cargo run --locked -p zryna -- build examples/universal/add.zry --target javascript --name add-js --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target webassembly --name add-wasm --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target native --name add-native --node /absolute/path/to/node
cargo run --locked -p zryna -- build examples/universal/add.zry --target all --name add-all --node /absolute/path/to/node

cargo run --locked -p zryna -- run examples/universal/add.zry --target javascript --name add-js --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target webassembly --name add-wasm --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target native --name add-native --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target all --name add-wrap --export add --arg=i32:2147483647 --arg=i32:1 --node /absolute/path/to/node --json
```

The last command reports ordered results from all three targets; result equivalence is not enforced
until Issue #20. Each successful command commits one complete create-only bundle below
`.zryna/out`; see the [CLI reference](../../docs/CLI.md) for exact paths and limits.
