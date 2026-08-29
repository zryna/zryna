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

Each member and adapter declares its allowed immediate files and directories. The scanner inspects an entry's metadata before excluding generated content. It excludes only these exact paths:

- root `.git`, which may be the regular file used by a Git worktree or a real directory;
- root `target` and root `node_modules`;
- each registered adapter's immediate `node_modules` directory;
- the declared `.zryna/cache` and `.zryna/out` directories.

An excluded directory must still be a real directory. A symlink, Windows reparse point, socket, FIFO, device, or other special file at an excluded path fails validation. Names such as nested `target`, `dist`, and nested unregistered `node_modules` remain controlled content; for example, `crates/example/src/target` is inspected normally. Unexpected content such as `.zryna/other` is also inspected instead of inheriting an output exemption.

## Bounded stable inspection

Every controlled regular file, including the workspace contract and component manifests, is read as UTF-8 through the same stable reader. The reader opens final file components without following links, compares safe file handles before reading, bounds the read, and compares the open handle and current path again after reading. Unix uses no-follow, non-blocking file opens. Windows opens reparse points themselves, rejects every reparse attribute, denies new write/delete sharing while the handle is held, and compares volume/file identifiers through safe file handles.

The production budgets are:

| Budget | Limit |
| --- | ---: |
| Workspace contract | 1 MiB |
| Cargo or adapter manifest | 1 MiB |
| Other controlled file | 2 MiB |
| Aggregate controlled bytes | 64 MiB |
| Filesystem entries | 50,000 |
| Directory depth | 32 |
| Registered members plus adapters | 256 |
| Validation diagnostics | 256, including the terminal budget diagnostic |

Entry traversal and later structural directory checks are sorted before diagnostics are selected. Exhausting any validation budget emits `ZRYNA-A1204`, halts all remaining traversal, and prevents later architecture validators from reading the incomplete workspace. A structural addition therefore requires a deliberate contract edit instead of becoming architecture by accident.

The implementation detects persistent links/reparse points, final-component replacement, and ordinary modification/replacement races. It is not an operating-system sandbox against a hostile process concurrently replacing an ancestor directory: Rust's standard filesystem API does not provide one portable atomic directory-handle walker. That stronger property requires a future capability-based, handle-relative walker on each supported operating system. Builds must therefore validate a workspace not being concurrently mutated by an untrusted process.

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
