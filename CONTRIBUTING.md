# Contributing

Zryna accepts focused changes that preserve the architecture contract and accurately describe implemented behavior.

Use the [task-oriented code navigation map](docs/CODE_NAVIGATION.md) to find component entrypoints and focused tests.

Before changing a component:

1. read `README.md`, `docs/ARCHITECTURE.md`, and `docs/STRICT_WORKSPACE.md`;
2. confirm that the component is registered in `zryna.workspace.json`;
3. preserve the declared dependency direction;
4. add stable diagnostics and tests for rejected input;
5. avoid claims for unsupported compiler or platform behavior.

Run every required check before submitting a change:

```bash
pnpm install --frozen-lockfile
pnpm preflight
pnpm m0:check
```

Run `pnpm preflight` during the edit loop. It stops at the first portable contract, formatting,
workspace-check, frontend, or syntax failure and normally reuses warm local build state. It is a
fast diagnostic gate, not a substitute for the complete Linux and Windows `pnpm m0:check` proof
required before merge.

The canonical M0 gate includes locked Rust dependency fetching, architecture validation,
formatting, strict Clippy, workspace tests and doc-tests, warning-free rustdoc, adapter and protocol
checks, and the conformance-registry self-tests on both supported CI operating systems.

New components must be created through the planned canonical creation command once it is available. Until then, component additions require a focused architecture proposal and simultaneous updates to both workspace manifests, documentation, tests, and CI.
