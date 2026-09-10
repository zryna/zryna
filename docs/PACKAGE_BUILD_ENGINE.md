# Authenticated package build engine

Status: implemented internal pure-source driver boundary for Issue #404. This does not add a
package registry, source acquisition, package-import syntax, native recipes, a public CLI selector,
or release publication.

## Authority boundary

`zryna-package` and the driver-owned source resolver authenticate the exact `zryna.lock.v1` graph,
package-instance pairs, source inventories, paths, sizes, and bytes. The resolver now retains those
already-verified source bytes in immutable memory. The build engine consumes only that sealed
resolution result; it never reopens a package path selected by source code.

The caller separately supplies the compiler, profile, execution-policy, host, environment, target,
runtime, composition, host-tool, and output identities required by
[`zryna.resolved-build-plan.v0`](../spec/package/RESOLVED_BUILD_PLAN_V0.md). The driver validates the
closed source-only relationships before constructing canonical bytes. The exact authenticated
package graph remains authoritative; caller input cannot add a package, dependency, source, or
unsupported target. The caller must declare the complete selected target set and target-qualified
output inventory before compilation.

Package source has no process, executable, hook, environment, filesystem, or network interface.
Compilation is an explicit in-process `PureSourceCompiler` call owned by the driver caller. Each
package is presented once in deterministic dependencies-first order with immutable source bytes
and only the already-completed direct dependency results. Dependency packages cannot publish root
outputs. The root output inventory must exactly equal the plan's target-qualified output list.

Package manifests do not yet declare dependency entrypoints or package-import syntax. This engine
therefore does not infer either one. A concrete project/CLI integration must admit its explicit
root entrypoint separately and implement only the language profiles it can authenticate.

## Plan identity

The engine emits exact canonical `zryna.resolved-build-plan.v0` bytes and preserves the specified
source-only status and schema. Objects use recursively sorted JSON keys, arrays retain their
contract order, insignificant whitespace is absent, and one LF terminates the record. The plan key
is the accepted domain-separated SHA-256 of the entire record with only `cacheKey` absent. Every
package/lock/source, compiler, profile, execution-policy, environment, target, runtime,
composition grant, tool, and output identity remains in that projection.

Each target cache key uses the separate accepted target domain over the exact plan key and target
id. The published bundle contains the exact plan as `zryna-resolved-build-plan-v0.json`. Its
`zryna-package-build-manifest-v1.json` records:

- the root package instance and lock digest;
- the exact plan path, byte length, SHA-256, and plan cache key;
- each target cache key and whether it was a validated hit or a newly compiled miss; and
- the complete output path, byte length, and SHA-256 inventory.

The final create-only bundle name is `<plan-cache-key>.package-build` below the validated project
`.zryna/out` root. Existing destinations are never replaced.

## Deterministic cache

`ArtifactCacheRoot::prepare_for_project` retains only the exact project-owned `.zryna/cache`
directory after validating its real ancestor chain. Before its first mutation, each fill reopens
that path, binds the opened directory to the retained identity, and creates/opens the cache
namespace relative to that capability with no-follow semantics. Build entries live below
`build-plan-v0/<target-cache-key>`. A complete entry contains closed `entry.json` metadata and the
exact declared target outputs.

A missing target-key directory is a cache miss. A present entry is always independently checked:
its directory and complete inventory must be real and link-free; metadata must be exact canonical
bytes with the expected plan key, target key, and target; and every output path, size, and SHA-256
must match both the plan and current bytes. Because self-declared metadata hashes are not an
authentication authority, every structurally valid hit is also presented to the trusted compiler
with the exact plan, target, authenticated source graph, and output bytes. The compiler must
reproduce the outputs or verify its own plan-bound attestation before reuse. Wrong-target, partial,
extra, missing, stale, corrupt, or self-consistently forged entries reject. They are never relabeled
as misses or repaired during the failing request.

Misses compile from the immutable authenticated source closure. The writer creates a private
sibling entry, writes and synchronizes every output, writes metadata last, audits the complete
inventory, synchronizes the directory tree, and performs one create-only rename. A failed or
interrupted writer does not make its staging directory addressable by a target cache key. A race
that finds an already committed entry succeeds only when its bytes exactly match the current
trusted compilation result. Cleanup retains the exact stage identity and removes only a bounded
subset of paths successfully written through the retained stage capability. Files and directories
are removed through that same capability and the stage root is removed non-recursively through its
retained parent. A substituted stage or unexpected path is left untouched and reported as a
failure.

Both `offline` and `frozen` build modes expose no acquisition or network operation. `frozen`
additionally requires a resolver result produced by exact existing-lock verification; an update
resolution cannot be relabeled frozen. A clean frozen build may compile from preprovisioned,
authenticated source/tool inputs when the artifact cache is empty.

## Publication and diagnostics

After every selected target is materialized, publication writes the exact plan, outputs, and build
manifest to one private sibling of `.zryna/out`. The reopened output directory must match the
previously retained output-root identity before the stage is created. Files are create-only and
synchronized through the retained stage capability. The driver
audits all paths and hashes before and after one create-only directory rename. Any preparation,
compiler, cache, or publication failure returns no successful bundle.
Publication cleanup uses the same retained-stage and bounded-inventory rule and refuses to
recursively remove a substituted or expanded directory.

Stable failure families are:

| Code | Boundary |
| --- | --- |
| `ZRYNA-B4101` | plan identity, ordering, compatibility, bounds, or frozen admission |
| `ZRYNA-B4102` | cache root, entry identity, inventory, or bytes |
| `ZRYNA-B4103` | output staging, audit, synchronization, or create-only commit |
| `ZRYNA-B4104` | explicit pure-source compiler failure or undeclared output |

Focused tests cover dependencies-first execution, independent domain hashes, isolated-cold and
warm byte identity, every plan invalidation class, exact/first-extra output bounds, corrupt and
partial cache entries, wrong-target metadata, wrong compiler outputs, frozen relabeling, and failure
without cache or output publication. They also cover self-consistent cache forgery, substituted
stage cleanup, every reserved metadata filename, and the first file/ancestor output collision.
Root and cache-namespace substitution probes also run in the exact validation-to-capability-capture
windows and verify that no stage or output is created in the replacement directory.
Run `cargo test --locked -p zryna-driver package_build` for the focused Rust boundary, then the
repository-required preflight and M0 gates on the exact candidate revision.
