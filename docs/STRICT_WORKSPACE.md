# Strict workspace contract

`zryna.workspace.json` is the authoritative repository registry. The JSON Schema improves editor feedback, but the Rust architecture engine remains authoritative because it also checks the real filesystem and Cargo dependency graph.

## Fail-closed policy

The check fails when it cannot prove correctness. It rejects:

- missing, invalid, or unknown contract fields;
- absolute paths, traversal, backslashes, duplicate roots, and case-insensitive collisions;
- unregistered root entries or Rust workspace members;
- immediate component entries not listed in that component's `allowedEntries` contract;
- missing component manifests, documentation, or canonical Rust entrypoints;
- adapter identities, protocol metadata, workers, or exact toolchain pins that drift from the contract;
- Cargo dependencies that differ from the registered direct dependency graph;
- forbidden dependency direction or cycles;
- symlinks and non-regular filesystem entries inside controlled components;
- incomplete scans caused by read errors or deterministic safety budgets.

There is no skip flag. Official build and release automation must run the same engine before any compiler phase.

Each member and adapter declares its allowed immediate files and directories. Generated `.git`, `.zry`, `dist`, `node_modules`, and `target` directories are excluded from source inspection; every other inspected repository entry is bounded, readable, regular, and non-symlinked. A structural addition therefore requires a deliberate contract edit instead of becoming architecture by accident.

## Stable architecture diagnostics

```text
ZRYNA-A1001  missing or unsupported workspace contract
ZRYNA-A1003  unsafe, duplicate, or colliding path or identity
ZRYNA-A1004  forbidden repository-root entry
ZRYNA-A1005  undeclared or missing workspace member
ZRYNA-A1006  invalid member shape, name, or entrypoint
ZRYNA-A1007  entry outside a component's declared layout
ZRYNA-A1010  invalid adapter shape or protocol metadata
ZRYNA-A1011  adapter toolchain pin mismatch
ZRYNA-A1101  Cargo and contract dependencies disagree
ZRYNA-A1102  forbidden dependency direction
ZRYNA-A1103  internal dependency cycle
ZRYNA-A1201  symlink or non-regular filesystem entry in inspected repository content
ZRYNA-A1202  canonical path escapes the workspace root
ZRYNA-A1203  invalid UTF-8 or unstable file read
ZRYNA-A1204  deterministic scan budget exceeded
ZRYNA-A1205  incomplete scan
```

## Controlled mutation

Future create and move commands will use a transactional planner:

1. derive the complete canonical change plan;
2. validate every target and ancestor;
3. refuse overwrites;
4. record expected hashes for edited files;
5. stage all writes;
6. commit atomically where the platform permits;
7. roll back on partial failure;
8. run the full architecture check again.

The future editor extension will display JSON diagnostics from this engine. It will not duplicate or redefine architecture rules.
