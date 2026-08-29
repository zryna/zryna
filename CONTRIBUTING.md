# Contributing

Zryna accepts focused changes that preserve the architecture contract and accurately describe implemented behavior.

Before changing a component:

1. read `README.md`, `docs/ARCHITECTURE.md`, and `docs/STRICT_WORKSPACE.md`;
2. confirm that the component is registered in `zryna.workspace.json`;
3. preserve the declared dependency direction;
4. add stable diagnostics and tests for rejected input;
5. avoid claims for unsupported compiler or platform behavior.

Run every required check before submitting a change:

```bash
cargo fetch --locked
cargo run -p zryna -- architecture check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm adapter:check
pnpm adapter:test
```

New components must be created through the planned canonical creation command once it is available. Until then, component additions require a focused architecture proposal and simultaneous updates to both workspace manifests, documentation, tests, and CI.
