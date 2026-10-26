# Native source-candidate installation

`native-inventory-v2.mjs` and `native-distribution-v2.mjs` implement the separate
`zryna.native-distribution.v2` content grammar. The original distribution v1,
tagged releases and signature verification remain unchanged. These helpers do not
acquire source, run a compiler, authenticate an initial execution or publish a release.

The pinned `native-recipe-v2.json` admits an actual `refs/heads/main` or safe
lowercase `refs/heads/codex/<components>` or `refs/heads/fix/<components>` source candidate,
qualified Linux x86-64/Windows x64 archive baselines, the existing exact Cargo lock
and complete approved target-specific Rust notice closure. No Node executable,
TypeScript/bootstrap file, arbitrary helper executable or SDK payload is admitted.
Build/archive tooling may use pinned Node; the installed compiler provider does not.
JavaScript/WebAssembly execution and native SDK admission retain their separate requirements.

The trusted source producer supplies actual architecture command receipt bytes and
captured `LICENSE` plus every supplier notice returned by `nativeRustMaterials(target)`.
`prepareNativePayload(identity, capturedMaterials, architectureReceipt)` verifies
those complete bytes and returns the canonical prepared manifest and payload.
`identity` contains `version`, `source`, `target` and the exact native `recipe`
descriptor. Preparation creates no executable or publication authority.

The producer compiles the ordinary CLI with `ZRYNA_DISTRIBUTION_SHA256` set to the
SHA-256 of that exact prepared manifest, retaining the actual source/toolchain/build
and acceptance evidence. `assembleNativeInstallation(prepared, compiledCli)` checks
the original target-specific executable identity marker, creates the complete
inventory/checksums and uses the original deterministic archive encoders. The
running compiler's existing retained-file and running-image admission checks consume
that same manifest digest; adjacent JSON cannot mint an installation capability.

`verifyNativeArchive(archive, independentlySelectedSubject)` reads the entire archive,
checks the exact supplier/source closure, original executable marker and all inventory,
checksum, mode, topology and resource rules. Its returned bytes are content verification,
not an outer signature or a substitute for independent source selection. Preserve the
actual relocated no-Node CLI, both-provider parity and platform results separately.

Focused content-boundary checks:

```sh
node --test tests/native-distribution-v2.test.mjs
cargo test --locked -p zryna-driver --lib distribution::manifest::native
```

Fixture/schema checks do not establish a completed native installation or #414 acceptance.

The producer observes an attached branch using `git symbolic-ref HEAD`. A detached
GitHub checkout must retain its actual event file and event environment: same-repository
pull-request head SHA/ref, push after/ref, or workflow-dispatch SHA/ref must match the
selected commit and admitted branch. Synthetic PR refs, tags, arbitrary feature branches,
unsafe components and a supplied source-ref override cannot label source as main.
The two candidate namespaces retain the same lowercase ASCII component grammar,
200-byte ref bound, 64-byte component bound and total depth12; a branch spelling
adds no source, worktree, execution or production-release authority.
The retained source-ref evidence pins the original event bytes when they are used.
