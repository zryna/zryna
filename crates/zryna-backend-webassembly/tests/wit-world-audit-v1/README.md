# Pinned WIT dependency evidence

This fixture tree is the offline input for the WebAssembly backend's Issue #383 audit. It does
not define a new world, emit a component, generate bindings, instantiate a runtime, or grant a
host capability.

## Parser pin

The implementation uses `wit-parser` exactly `0.258.0` with default features disabled and only
`std` enabled. The crates.io archive SHA-256 is
`ff4daaa3cd97ae49ecd0a99dc009d453f93e0f083dd3be38c0f24a83a93e37ac`. The release was published
from `bytecodealliance/wasm-tools` commit
`5c6d31c78f8bd503f558441ef9c732950a141d1a`, the same source and version as the repository's
existing exact `wasmparser` and `wasm-encoder` pins.

## WASI source acquisition

Every file under `dependencies/` is copied byte-for-byte from
`https://github.com/WebAssembly/WASI` release `v0.2.12`, aggregate commit
`281ba75fafcd50961ef55f9e52747afcc9b71ede`. The proposal directories are ordinary trees in that
commit, not Git submodules. Their exact Git tree identities are:

| Package | Source tree | Git tree |
| --- | --- | --- |
| `wasi:cli@0.2.12` | `proposals/cli/wit` | `002c0173a32555455a5f010361350493057adb3f` |
| `wasi:clocks@0.2.12` | `proposals/clocks/wit` | `0d1dfc07efd87c46054bd465ed76f2f1c22c4a95` |
| `wasi:filesystem@0.2.12` | `proposals/filesystem/wit` | `a0881fa7db9ff31c932906ba1bb6eada2c2314c2` |
| `wasi:http@0.2.12` | `proposals/http/wit` | `4b47908b22e6ce408d65a9302850df61c31e869b` |
| `wasi:io@0.2.12` | `proposals/io/wit` | `886c85402019949ada35f388e6ae9f6c4dadb20d` |
| `wasi:random@0.2.12` | `proposals/random/wit` | `bd1b0e57834a3e4571e051e950e108b1c877113e` |
| `wasi:sockets@0.2.12` | `proposals/sockets/wit` | `4daf071639f0b99067c882ee247ca214ef69a924` |

The upstream `deps.lock` files authenticate the locally referenced package archives with these
SHA-256 values: CLI `c312a448b90753dbcf351472ac1130268b763a0c6d3a0f11678e69784638bbb7`, clocks
`85b95504b5f627086433c75a634ef86cc5a346f000acd158b94cb01d508b1992`, filesystem
`1833958cf47f63ea6ff67cc5ff82ac64069d794007d7c69ce493febce5309962`, IO
`b45dcb3986b694bd2de3b4c092ee3cc3b0e7c654a93e5dee0c5b32e349d1b0d8`, random
`11a4642d55267644bd11f302661c044f6cb1c43a6aea5c4f919fdc1e49840551`, and sockets
`2ee88acd6247b6dc8ebeddab95f43ab8a7b587c58b211e6d8022c3b8ac9ff984`. HTTP is a root proposal
for this closure, so its source-tree identity and the per-file SHA-256 pins in
`src/wit_world_audit/pins.rs` authenticate it directly. `WASI-LICENSE.md` is the upstream license
notice at SHA-256 `0416590f3f47381bb5b9b467c27824d1228fac58b8031d693130865273ffefcb`.

## Bounds

Authentication occurs before parsing. One call accepts at most 34 files, 32 KiB per source,
16 KiB for the local root, 256 KiB total, and 96 bytes per logical path. Resolution is capped at
8 packages, 32 interfaces, 16 worlds, and 4,096 types. The accepted closure is 34 files and
141,709 bytes including the 1,129-byte local root; the vendored WASI inputs are 140,580 bytes.
Input order is normalized before authentication and every call constructs a fresh resolver.

The accepted root explicitly declares 0/13/4 imports for browser/command/server. The pinned parser
elaborates interface type dependencies into exact resolved closures of 0/16/8: command additionally
observes `wasi:io/error`, `wasi:io/poll`, and `wasi:io/streams`; server additionally observes those
three interfaces and `wasi:http/types`. All are `0.2.12`. Tests assert both complete lists and first
prove that the unmodified vendored dependency sources pass the independent resolved audit.
Elaboration records dependency topology only; it does not change the registry or grant authority.

The `hostile/` files are intentionally unauthenticated replacements used to prove that version,
world, broadening and syntax substitutions fail closed. They are not alternate accepted sources.
