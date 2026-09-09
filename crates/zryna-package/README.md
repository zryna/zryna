# Zryna package resolution

`zryna-package` owns the closed `zryna.package.v1` and `zryna.lock.v1` wire
records, their canonical hashes, deterministic source-only dependency graph,
package-instance identities, and frozen-lock comparison.

The crate accepts package bytes only through a caller-supplied source provider.
It never opens a host path, invokes Git, contacts a registry, executes package
content, compiles source, or publishes a lockfile. The driver owns retained
filesystem and prepopulated exact-commit Git-cache capabilities and atomic
publication. Language semantics remains the authority for declarations and
types; the nominal identity helpers here only retain the authenticated package,
module, graph-role, profile, and declaration coordinates specified by the M5
contract.

The initial implementation supports workspace-relative local packages and
already materialized exact-commit Git packages. It deliberately rejects ranges,
branches, tags, features, build dependencies, scripts, native recipes, package
imports, and registry lookup.
