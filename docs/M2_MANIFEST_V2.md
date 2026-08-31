# M2 manifest and atomic bundle contract

Status: implemented for explicit `--profile control-flow-v1` build and run requests. This contract
does not change default M1 behavior. The separate [M2 conformance gate](M2_CONFORMANCE.md)
authenticates its fixed-oracle three-target evidence.

## Profile and authority

Omitting `--profile` selects the existing protocol-v2 `I32V1` path and
`zryna-manifest-v1.json`. Exact `--profile control-flow-v1` selects protocol v3, one driver-owned
module-closure discovery, one final authenticated source map, and one semantic lowering to opaque
verifier-sealed `ControlFlowV1`. Every selected backend consumes that same verified program. A run
request prepares one scalar-ABI invocation once and supplies the same typed authority to every
selected target.

The module provider receives immutable normalized source maps but no filesystem capability and
does not resolve imports. Backends receive no source or workspace authority. The CLI parses the
closed profile/target/argument grammar and renders the driver result; it does not rediscover,
reanalyze, compare, or publish.

## Bundle and manifest

M2 uses the existing create-only bundle destinations:

```text
.zryna/out/<stem>.build/
  zryna-manifest-v2.json
  javascript/<stem>.mjs
  webassembly/<stem>.wasm
  native/<stem>.o

.zryna/out/<stem>.run/
  zryna-manifest-v2.json
  javascript/<stem>.mjs
  webassembly/<stem>.wasm
  native/<stem>.elf
```

Only selected target directories exist. A bundle contains exactly one manifest version. An M1 and
M2 request with the same stem and command therefore collide rather than coexist or replace one
another. Build and run destinations remain distinct.

`zryna-manifest-v2.json` is canonical pretty UTF-8 JSON terminated by one line feed. Its fixed
top-level field order is:

1. `version`, exactly `2`;
2. `profile`, exactly `zryna-control-flow-v1`;
3. `command`, `build` or `run`;
4. canonical portable `entrypoint`;
5. lowercase `graph_sha256` copied from the authenticated `ZRYNA-M2-GRAPH\0` version-1 graph
   identity;
6. `sources`, in normalized path-byte order, each recording its dense `id`, portable `path`, and
   lowercase SHA-256 of exact UTF-8 source bytes;
7. `edges`, in canonical importer/specifier/imported/local byte order, each recording `importer`,
   resolved `target`, source `specifier`, imported name, and local name;
8. portable `stem`;
9. selected `targets` in JavaScript, WebAssembly, native order;
10. `artifacts` in the same target order, with target, kind, bundle-relative `/` path, byte length,
    and lowercase SHA-256;
11. `invocation`, `null` for build or the exact logical export and ordered typed arguments for run;
12. ordered typed `results`, empty for build; and
13. stable ordered `diagnostics`.

The graph digest remains the module-closure authority. The manifest records it; neither the CLI nor
a backend derives a competing graph identity from JSON. Source and edge records make that identity
inspectable without exposing host paths. Typed scalar records distinguish `bool` from `i32`;
WebAssembly/native Boolean lanes are canonical `0` or `1`, never truthy integers.

The manifest contains no absolute path, host path separator, temporary transaction name,
timestamp, process ID, environment value, credential, raw external-tool output, or nondeterministic
map iteration. Sources, edges, targets, artifacts, results, and diagnostics have one stable order.
Manifest serialization is capped at 32 MiB. Discovery also rejects a conservative edge-manifest
estimate above 32 MiB before repeated binding-edge strings can be materialized. The staged file
inventory is bounded. Diagnostics are compiler diagnostics only and retain their stable order and
normalized source locations.

## Atomic publication

The driver prepares every selected sealed artifact before advertising output. One private sibling
transaction receives create-new target directories, exact artifact files, and any private runtime
harnesses. Execution happens before commit. Private harnesses are removed, artifact and manifest
length/hash plus the exact staged inventory are checked, and the manifest is written last. A final
byte/inventory audit runs immediately before publication. Files and supported directories are
synchronized before one same-filesystem, create-only directory rename relative to the retained
output-root capability exposes the final bundle. The published identity and inventory are checked
again; an identity mismatch is rolled back to the private stage before failure is returned.

The successful rename is the only commit point. Every fallible discovery, source, semantic,
backend, execution, write, audit, inventory, manifest, synchronization, collision, or cleanup step
before it reports no final manifest path. Existing files, directories, symbolic links, and Windows
reparse points at the destination are never replaced. A cleanup failure has its distinct exit
status and still does not advertise a bundle. Abrupt process or machine termination may leave an
unadvertised private transaction directory. Directory-entry crash durability after the successful
rename is not claimed.

The transaction retains and revalidates compiler-owned source/output/stage identities across
authority transitions. Source traversal is component-by-component and no-follow. Transaction
names, target names, manifest names, artifact extensions, and runtime-harness names are closed
compiler constants; callers cannot supply paths inside the bundle. These protections are
deterministic compiler containment, not an operating-system sandbox against a concurrently hostile
process with the same user authority.

## Platform behavior and remaining gate

JavaScript and import-free core WebAssembly build/run are supported by the required Linux and
Windows paths. Native object bytes target Linux x86-64. Native run and `all` run require Linux
x86-64; Windows returns `ZRYNA-N4002` before native staging or final-bundle publication, with no
fallback and no partial portable-target bundle.

Issue #55 proves explicit selection, single graph/semantic authority, per-target dispatch, typed
outcomes, deterministic manifest v2, and atomic publication. Issue #56 provides the separate
executable fixed-oracle registry and required aggregate cross-target gate. Issue #57 owns
authenticated documentation publication, website synchronization, deployment, and live closure.
