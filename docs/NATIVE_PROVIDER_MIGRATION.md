# Native compiler provider migration

Current source checkouts select the embedded native frontend for ordinary M1, M2 and
M3 compilation. `--provider native` is explicit; omitting the flag selects the same
provider. Compilation requires no Node.js or pnpm. The driver still verifies source-bound
frontend IR before semantic analysis, lowering and backend dispatch.

```sh
cargo build --locked -p zryna
./target/debug/zryna build examples/universal/add.zry --target javascript --name native-add
./target/debug/zryna build examples/universal/add.zry --target webassembly --name native-wasm
```

The explicit compatibility route is `--provider bootstrap --node /absolute/path/to/node`.
It requires the exact Node.js 22.22.1 executable and authenticated bootstrap files.
`--provider compare` requires both providers and rejects a mismatch before publication.
Neither selector changes the language profile, backend, source map or target observation
contract. The current source-checkout WASI H1 and bounded server routes also use the native
frontend by default; their existing guest, host, grant and listener approvals still apply.

JavaScript and WebAssembly **execution** still requires the separately selected Node
runtime. Linux native execution still requires the original authenticated SDK capability.
Native `build` emits a Linux ELF object on both qualified hosts; Windows native `run`
retains the existing unsupported-host rejection. Embedded frontend selection grants no SDK, runtime, filesystem or network authority.

## Native source-candidate installation

The separate `zryna.native-distribution.v2` format contains the ordinary compiled CLI,
complete Rust supplier notices and source/architecture metadata. Its finite inventory admits
no Node executable, TypeScript bootstrap, pnpm, SDK or arbitrary helper program. The compiler
retains the original build-bound manifest SHA, current-running-image identity, complete file
hashes, modes, topology and alias checks. The original v1 distribution grammar, signatures,
artifact identities and bundled bootstrap default remain unchanged.

The source candidate uses the pinned [native recipe](../scripts/distribution/native-recipe-v2.json)
and an actual main, safe `codex/*`, or safe `fix/*` branch ref with the selected commit/tree.
The same bounded lowercase component grammar and exact clean-source proof apply to both
candidate namespaces; their names grant no release or execution authority. This is separate
from immutable version 0.2.3 release artifacts. It is not an authenticated outer production
release, and this change does not publish or replace any release/tag.

A trusted producer can build and verify an installation from a clean, committed checkout:

```sh
node scripts/distribution/produce-native-installation.mjs \
  --source /absolute/clean-checkout --output /absolute/fresh-evidence-root --jobs 2
```

Build/archive tooling uses Node.js 22.22.1 and Rust 1.97.1. Prepare locked Cargo dependencies
before execution; the producer builds offline with controlled Cargo/Rustup homes and rejects
Cargo configuration overrides. It captures the actual architecture command and whole Rust
supplier inputs, builds two independent compiler graphs with deterministic path/link flags,
and requires identical compiler and archive bytes. Cargo's normal CLI/deps hardlink and
Rustup's finite multicall shim links are captured through a producer-only stable bounded read:
every original link must be explained by the observed allowed alias closure, all bytes/identities
are retained in the proof, and a fresh single-link copy supplies packaging/tool execution. A copy
never counts as an independent rebuild; installed files retain their original single-link rule. It then independently verifies the complete
archive, extracts and relocates it, and invokes its ordinary CLI from an unrelated directory
with an empty PATH and cleared environment.

The installed smoke compiles declared M1–M3 projects to JavaScript/WebAssembly on both hosts
and Linux native objects on both hosts. Separate coordinator runtime observations check fixed results;
Linux also runs one native result through the unchanged authenticated SDK. Omitted-provider
compilation must pass, and explicit bootstrap/compare must fail because their payloads are
absent. The ordinary installed pure H1 command also runs with explicit native and omitted
provider selections, without Node or a package manifest. Both actual runs must return typed
`ok`, confirm teardown, and retain identical component and entire manifest bytes. Explicit
bootstrap/compare H1 requests must fail with `ZRYNA-C4103` without publication. This coverage
is established only by actual passing retained command results on each host.
The [two-host workflow](../.github/workflows/native-provider-install.yml) retains the
actual two archives, command outputs, source/recipe pins, reproduction record and installed
observations. A passing `RESULT.json` is created only after those commands pass. Failure
records do not certify cleanup or turn an incomplete run into success.

The separate [public parity workflow](../.github/workflows/native-provider-public.yml)
compares the complete frozen worker and ordinary CLI roots against the explicit bootstrap
provider. Workflow definitions, schema fixtures and historical private activation evidence do
not establish passing current-revision qualification. Read the actual retained results from
both hosts before claiming installation, reproduction, corpus parity or issue acceptance.
