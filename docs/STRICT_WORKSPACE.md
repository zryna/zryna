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

Component paths use one host-independent identity before any operating-system lookup.
Applications are exactly `apps/<id>`, library members are exactly `crates/<id>`, and adapters are
exactly `adapters/<id>`. Path segments and controlled entry names are printable ASCII, use
case-insensitive identities, and reject normalized-away segments, Windows-reserved device stems,
reserved characters, trailing dots or spaces, and case-colliding siblings. The `apps`, `crates`,
and `adapters` containers may contain only their registered component directories.

The root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and `zryna.workspace.json` entries must
be regular files; `apps`, `crates`, and `adapters` must be real directories. Registered package
names, manifest locations, component ids, and Cargo workspace package ids must describe the same
physical roots.

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
| Cargo metadata stdout | 16 MiB |
| Cargo metadata stderr | 64 KiB |
| Cargo metadata packages | 4,096 |
| Cargo metadata dependency edges | 65,536 |
| Cargo metadata execution | 30 seconds |

Entry traversal and later structural directory checks are sorted before diagnostics are selected. Exhausting any validation budget emits `ZRYNA-A1204`, halts all remaining traversal, and prevents later architecture validators from reading the incomplete workspace. A structural addition therefore requires a deliberate contract edit instead of becoming architecture by accident.

The implementation detects persistent links/reparse points, final-component replacement, and ordinary modification/replacement races. It is not an operating-system sandbox against a hostile process concurrently replacing an ancestor directory: Rust's standard filesystem API does not provide one portable atomic directory-handle walker. That stronger property requires a future capability-based, handle-relative walker on each supported operating system. Builds must therefore validate a workspace not being concurrently mutated by an untrusted process.

## Complete Cargo graph proof

After bounded filesystem inspection, the architecture engine invokes the pinned Cargo CLI directly
without a shell using metadata format version 1, all features, `--frozen`, and no platform filter.
The subprocess has concurrent bounded output readers, package and edge budgets, and a 30-second
deadline. The validator stops waiting at the same deadline even if a descendant retains an output
pipe. Network access and lockfile updates remain disabled. Root and member manifests, `Cargo.lock`,
`rust-toolchain.toml`, and both present and absent repository-local Cargo configuration paths are
recorded before the subprocess and compared again afterward.

The pinned command guarantee assumes a trusted build environment: `CARGO`, `PATH`, and rustup
selection variables must not be controlled by untrusted workspace content. Official CI pins Rust
1.97.1 before running the gate.

The engine combines both Cargo metadata views instead of trusting manifest key spelling:

- `workspace_members` and package ids prove the exact registered package set;
- every package dependency declaration covers aliases, inactive optional dependencies, normal,
  dev, build, and target-specific sections;
- `resolve.nodes[].deps[].pkg` proves the opaque package id reached after patches and resolution;
- every local package and local dependency path must match one registered physical member root.

The v1 contract stores component adjacency as target-id strings. One declared source-to-target edge
therefore authorizes that component relationship across all Cargo dependency kinds and target
predicates; every observed kind and target still contributes to the union graph. Adding an edge to
an undeclared component fails even when hidden behind an alias, dev/build section, optional feature,
target predicate, workspace inheritance, or source patch. Exact kind-specific authorization would
require a future structured-edge contract version and is not claimed by v1. Layer-direction and
cycle checks run over the observed Cargo union graph, not only the claimed JSON graph.

The executable-syntax phase graph is registered explicitly:

```text
zryna-source ───────────────┐
zryna-diagnostics ──────────┼→ zryna-syntax (foundation)

zryna-source ───────────────┐
zryna-diagnostics ──────────┼→ zryna-frontend (frontend)
zryna-syntax ───────────────┘

zryna-source ───────────────┐
zryna-diagnostics ──────────┼→ zryna-semantics (compiler)
zryna-syntax ───────────────┤
zryna-ir ───────────────────┤
zryna-abi ──────────────────┘
```

This graph lets providers construct untrusted DTOs while preventing semantic lowering from
depending on provider code. Compiler-to-frontend and backend-to-frontend edges are rejected by
the architecture validator.

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

## Compiler output transactions

The architecture scanner excludes only the declared real `.zryna/out` directory; that exclusion
does not authorize arbitrary output paths. Public `build` and `run` derive an output capability
for that exact directory and accept no caller-selected output root. They validate the portable
entrypoint and artifact stem, reject persistent links and Windows reparse points, and place all
selected output below one new sibling transaction directory. Unix sets the transaction and target
directories to mode `0700`. Windows inherits ACLs from the validated compiler-owned output root;
the workspace and `.zryna/out` must therefore already be private to the invoking principal.

The transaction synchronizes every artifact and `zryna-manifest-v1.json`, revalidates containment,
and performs one create-only same-filesystem directory rename to exactly
`.zryna/out/<stem>.build` or `.zryna/out/<stem>.run`. Only selected target subdirectories are
created. A final bundle exists only when it is complete; failure removes the known staged entries,
does not advertise a partial build, and never modifies a pre-existing destination. Build and run
bundles may share a stem because their final names are distinct. Cleanup that cannot be confirmed
is a separate fail-closed exit category. The create-only rename is the commit point; directory-
entry crash durability afterward is not claimed. The exact layout is specified in the
[CLI reference](CLI.md).

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
