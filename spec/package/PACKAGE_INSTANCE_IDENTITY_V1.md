# Package instance identity and dependency compatibility v1

Contract identity: `zryna.package-instance-identity.v1`. State: **draft, dependency-aligned, specified-only**.
This document records the language-facing M5 decisions tracked by
[#360](https://github.com/zryna/zryna/issues/360). It specifies future compiler inputs and
rejection rules. It does not implement a resolver, add package or type-import syntax, activate a
registry, fetch a Git repository, execute a build dependency, add a CLI selector, or change any
public M0-M3 profile.

## Authorities and dependency alignment

The package and release contract owned by
[#168](https://github.com/zryna/zryna/issues/168) is the sole authority for package-manifest,
lockfile, checksum, canonical-serialization, and release-provenance bytes. This document consumes
its exact `source`, `dependency`, `manifest`, `edge`, `package`, `lock`, and `compatibility`
records. In the accepted v1 schema, a lock package has the exact fields `id`, `sourceSha256`, and
`dependencies`; a lock edge has `alias` and `package`. This document does not define competing JSON
fields, key order, hash domains, wire bytes, or fixture limits. Any future #168 schema revision
requires this semantic mapping to be reviewed again.

The [cross-target profile composition v1](../language/CROSS_TARGET_PROFILES_V1.md) contract owned by
[#357](https://github.com/zryna/zryna/issues/357) is the sole authority for language-profile,
target, deployment-host, interface, and transitive capability acceptance. This document decides
which package instances and graph roles are compared; #357 decides whether the resulting target
closure is admissible. Its specified-only contract requires exact language-profile compatibility,
independently verified claims when source supports both profiles, no relabeling of precompiled
authority, and rejection of unknown compatibility. Its future composition bounds apply after
package resolution and do not replace #168's fixture envelope.

[#356](https://github.com/zryna/zryna/issues/356) remains the planning index. The package build-plan
and trust tasks [#361](https://github.com/zryna/zryna/issues/361) and
[#362](https://github.com/zryna/zryna/issues/362) own executable build inputs, cache keys, source
acquisition, and build-tool isolation. They do not redefine source-package nominal identity.

Existing contracts remain authoritative:

- `zryna.workspace.json` describes this compiler repository, not a user package;
- output manifests v1-v3 describe one compiler transaction, not dependency resolution;
- current M2/M3 module closure accepts only explicit relative `.zry` named imports;
- Zryna semantics owns names, types, visibility, and nominal compatibility after authenticated
  source closure; and
- backend target names, filesystem paths, and emitted symbols never decide source type equality.

Acceptance of this draft must preserve those boundaries. A schema or profile disagreement is a
dependency-alignment blocker, not permission to duplicate or reinterpret the owning contract.

## Identity layers

The following identities are intentionally distinct. A consumer must not substitute one for
another because two displayed names happen to match.

| Identity | Normative components | Purpose | Explicit exclusions |
| --- | --- | --- | --- |
| source tuple | #168 source kind, canonical locator, and exact revision | selects one declared local or Git origin | alias, checkout path, redirect, branch, tag |
| manifest identity | exact #168 lock-package `id`, the domain-separated digest of canonical manifest bytes | authenticates name, version, `source`, files, dependencies, and compatibility | source material trust or semantic `TypeId` |
| source identity | exact #168 lock-package `sourceSha256`, the `source-files` digest of the complete ordered source-file inventory | authenticates exact declared source bytes | mtime, host root, discovery order |
| package instance | exact ordered pair `(id, sourceSha256)` from one validated `zryna.lock.v1` `packages[]` record | stable resolved content instance | package name, version, alias, target, host path |
| module identity | `(package instance, normalized package-relative module path)` | stable source-module owner across discovery order | dense `ModuleId`, importer spelling, absolute path |
| semantic domain | exact graph role and exact selected language-profile authority | prevents host/target and cross-profile authority mixing | backend target and deployment host |
| nominal declaration | `(semantic domain, module identity, declaration ordinal)` | compiler-owned nominal type equality | declaration spelling, structure, layout, ABI name |

The declaration ordinal is the existing zero-based source order among nominal declarations in the
authenticated module. A compiler may assign dense package, module, or type IDs after sorting the
complete closure, but those IDs are private indexes into the sealed identities above. They are not
persistent package identities and cannot be serialized as substitutes for them.

`package instance` and `source identity` are semantic terms, not #168 field names. In particular,
there is no `packageInstance` or `sourceIdentity` wire field. The second component is serialized
only as `sourceSha256`. A lock edge's `package` is the selected package record's `id`; its
`sourceSha256` is recovered and authenticated from that record. Canonical bytes, digest domains,
record ordering, and the mapping between manifests and lock packages remain wholly #168-owned.

`javascript`, `webassembly`, and `native` are not part of nominal identity. One universal target
closure verified under one exact language authority therefore has one nominal universe consumed by
all selected backends. A target-specific source selection would be a different resolved closure and
is unsupported by this v1 contract.

The semantic domain includes graph role, not compiler-host location. The same locked source used
once as a host build tool and once as a target library is authenticated by the same package-instance
pair but is checked in two incomparable semantic domains. No value or nominal type crosses between
those domains without a separately specified interface bridge.

## Local and exact-commit Git instances

Local and Git source syntax, normalization, digesting, and canonical bytes remain #168-owned. The
language-facing consequences are fixed here:

- A local locator is interpreted relative to the declared reproduction/source root, never the
  resolver process current directory. Relocating that complete root preserves the locator and
  instance identity. Moving the package to a different relative locator changes the manifest and
  therefore the package instance.
- A Git source names one canonical locator and one full exact commit. A branch, tag, abbreviated
  hash, symbolic ref, redirect destination, credential-bearing URL, or local checkout path is not
  an instance identity.
- Equal commits reached through different canonical Git locators remain distinct sources and
  package instances. No content-based origin substitution is inferred.
- Equal name, version, and source tuple with competing manifest or source identities is ambiguous
  or tampered input and rejects during package resolution. It does not create two selectable
  variants of one coordinate.
- Frozen/offline resolution uses only the exact validated lock plus already available declared
  materials. It cannot fetch, follow a ref, update, repair, choose a newer version, or consult a
  registry. Missing material rejects without a partial graph.

A source digest authenticates bytes but grants no trust, execution, capability, or compatibility.
Those decisions remain with #362, #357, and compiler semantics.

## Aliases, duplicates, and coexistence

A dependency alias belongs only to its importing package edge. It selects one exact locked package
instance and supplies a local package namespace for future package import syntax. It never enters
the selected instance's identity or the nominal identity of declarations inside it.

Two aliases from the same parent, or two paths through a diamond, may select the same exact package
instance. They share one module and nominal universe in the target graph. Importing the same
exported nominal declaration through both aliases therefore yields the same type.

Same-name packages may coexist only when every dependency edge resolves unambiguously by its alias
and #168 exact selection fields. The following instances are distinct and nominally incompatible:

- the same package name at different versions;
- the same name and version from different local locators;
- the same name and version from different Git locators or commits; and
- any pair with different authenticated manifest or source identities.

Names, versions, source spelling, equal declaration names, equal field/variant shapes, equal layout
fingerprints, and equal emitted ABI shapes cannot make distinct nominal declarations compatible.
There is no structural fallback, semver type equivalence, source substitution, package deduplication
by name, or "closest" version selection.

The resolver may intern repeated occurrences of the exact same package-instance pair within one
graph role. It must not intern across semantic domains or merge distinct instances merely to reduce
the closure. Because v1 has no feature-selection field, one package instance has one dependency
set. An unknown feature field rejects under #168 even when its requested set is empty; there is no
separate empty-feature spelling or implicit default-feature expansion. Future features that change
source, dependencies, declarations, profile requirements, or visibility must become an
authenticated part of a versioned instance identity before contextual instances are allowed;
implicit feature unification and every feature request reject now.

## Module and declaration visibility

Modules and declarations are private by default at a package boundary.

1. A relative import inside one package may resolve only within that package's retained source-root
   capability. It follows the current explicit `.zry` rules and may name any declared source module
   allowed by the active language profile. `..` cannot escape the package root.
2. A cross-package import may resolve only through a dependency alias declared by the importing
   package and only to an explicitly exported module entry in the selected dependency instance.
   The accepted #168 schema does not yet carry that module-export table, so cross-package import
   activation waits for a reviewed #168 schema extension that authenticates an explicitly owned
   module-visibility document. A detached language document or local convention cannot add an
   export to an already authenticated package instance.
3. The referenced declaration must also carry the existing explicit source `export`. Exporting a
   module does not export all its declarations, and exporting a declaration from a private module
   does not make the module externally reachable.
4. Dependency exports remain compiler-internal to the complete build. Only the root entry module's
   permitted exports enter the existing public target ABI. A package export is not automatically a
   JavaScript, WebAssembly, native, WIT, or C export.
5. Deep-import fallback, undeclared dependency lookup, ambient namespace search, re-export,
   wildcard, default, namespace, URL, `node_modules`, and host-path imports remain unsupported.

This contract assigns no new source spelling. In particular, a dependency alias in a manifest is
not silently accepted as a bare module specifier by protocol v2, v3, or v4. Package-module import
syntax and its provider-neutral DTO require a separate approved language/protocol change.

The same applies to nominal type imports. A future type import must authenticate the dependency
alias, exported module, exported declaration, exact package instance, and declaration ordinal
before semantics exposes the nominal identity. Until that syntax and its conformance gate are
approved, existing M3 behavior remains unchanged: relative imported function signatures may retain
foreign nominal identity, but source gains no package or type-import grammar.

## Graph roles, cycles, and compatibility

Resolution produces separate closed graphs:

| Graph role | Contains | Compatibility authority | Cannot authorize |
| --- | --- | --- | --- |
| target/runtime | source packages whose verified code enters the requested output closure | #168 exact compiler/profile/target claims, then #357 composition | build-tool execution or host permissions |
| host/build | tools and build inputs executed while producing target inputs | #361 resolved build plan and #362 trust/execution policy | target imports, target nominal equality, runtime capabilities |

An edge has exactly one role. Target resolution never follows a host/build edge, and source code
cannot import a host/build dependency. Host tool availability does not satisfy a target dependency,
and a target capability grant does not authorize build execution. If one locked package is used in
both roles, it appears once in each role-scoped closure and is validated independently.

The #168 v1 lock currently describes one source-package dependency graph and has no build-edge
kind. It must not be reinterpreted as a mixed graph. Host/build serialization waits for #361/#362.

Package dependency graphs are acyclic in v1. A self-edge or multi-package cycle rejects during
resolution before source-module discovery, profile composition, semantics, IR, or build execution.
Within each selected package instance, the existing module-import and resolved-call graphs also
remain acyclic. A package DAG does not excuse a module or call cycle, and a module-level edge cannot
hide a package dependency missing from the lock.

Cycle diagnostics use the first canonical shortest cycle: choose the fewest edges, then compare
the complete sequence of `(graph role, package instance, alias)` bytes lexicographically after
rotating each candidate to its least edge. No filesystem traversal or catalog insertion order may
change the witness.

For the initial source-only resolver, every target/runtime package must match the lock's exact
compiler and language-profile authority and must cover every requested target, as required by
#168. A version number does not imply compiler or language compatibility. No package may be
relabeled from `ControlFlowV1` to `DataOwnershipV1`, or vice versa, based on syntax or structural
inspection alone.

Cross-profile acceptance follows #357. A source package can claim both only if separately verified
under each; a precompiled authority cannot be relabeled and unknown compatibility rejects.
Because #168 `compatibility.profile` stores one exact profile, a package-backed multi-profile claim
requires a versioned #168 schema extension before resolution can represent it. With the current
schema, exact profile equality is therefore the only package-backed rule. A dependency incompatible
with any target in an `all` request rejects the complete request before backend dispatch; no target
is silently removed and no partial bundle is committed.

## Deterministic phases and diagnostics

Stable public codes are assigned only with an implementation. The following categories and phase
order are normative requirements for that later allocation:

| Category | Rejecting phase | Required evidence |
| --- | --- | --- |
| invalid-package-instance | #168 input/authentication | offending manifest, source, lock, or selection identity |
| ambiguous-selection | resolver selection | importing instance, alias, requested source tuple, zero or competing matches |
| duplicate-instance | resolver graph validation | duplicate serialized record or contradictory repeated record within one role; repeated traversal through a diamond is not an error |
| package-cycle | resolver graph validation | canonical role-scoped cycle witness |
| private-module | package module visibility | importer, alias, selected instance, requested module |
| private-declaration | language name/visibility resolution | authenticated declaration and import span |
| incompatible-nominal-type | language type checking before IR | both complete nominal identities and use span |
| incompatible-profile-target | #357 composition before IR | exact root-to-leaf package witness and rejected profile/target |
| graph-role-leak | graph validation or source import resolution | host/build and target/runtime occurrences plus forbidden edge/import |
| resolver-resource-limit | resolver validation | metric, inclusive limit, observed first-extra value |

Within a phase, order diagnostics by graph role (`target/runtime`, then `host/build`), package
instance bytes, normalized module path, authenticated source start, category, and complete stable
tie-break data. A diagnostic may print a portable locator already authenticated by #168, but not an
absolute checkout path, credential, environment value, or unauthenticated package name. Retain at
most 255 ordinary diagnostics plus one terminal resource diagnostic; once the terminal diagnostic
is emitted, stop without exposing a partial closure.

## Representative resolution and type examples

`A -x-> B` means package instance `A` has dependency alias `x` selecting exact instance `B`.
`T(P,m,d)` abbreviates the nominal identity for semantic domain `T`, package instance `P`, module
`m`, and declaration ordinal `d`. "Accept" is a specified future decision, not an executed result.

| Case | Input | Fixed outcome and phase |
| --- | --- | --- |
| alias-same-instance | `A -left-> B` and `A -right-> B`; both import exported `api.zry` declaration 0 | accept; both refer to the same `T(B,api.zry,0)` |
| diamond | `A -b-> B`, `A -c-> C`, `B -d-> D`, `C -d-> D` | accept one exact `D` instance in the target closure; canonical identities are discovery-order independent |
| version-coexistence | `A -old-> lib@1.0.0` and `A -new-> lib@2.0.0` from exact sources | resolution accepts two instances; passing `old::Token` where `new::Token` is required rejects as incompatible-nominal-type before IR |
| duplicate-name-source | two `lib@1.0.0` instances from different exact Git locators/commits | resolution accepts when aliases are unambiguous; equal `Token` spelling/layout remains nominally incompatible |
| ambiguous-coordinate | two manifests claim equal name, version, and source tuple but different authenticated content | ambiguous-selection during resolution; neither instance enters source discovery |
| private-module | `A -lib-> B` requests `internal/state.zry`, absent from B's public module table | private-module before the file is exposed to provider or semantics |
| private-declaration | B exports `api.zry` but declaration `Secret` has no source `export` | private-declaration during name/visibility resolution |
| root-escape | B's internal relative import resolves outside B's package root | existing path/containment rejection during module discovery |
| package-cycle | `A -b-> B -c-> C -a-> A` | package-cycle before module discovery, profile composition, or build execution |
| module-cycle | acyclic packages; two modules within B import each other | existing module-cycle rejection after package selection, before semantics |
| incompatible-transitive-profile | A selects one exact profile; `A -> B -> C`; C lacks that accepted profile or one requested target | incompatible-profile-target under #357 before IR; report A-to-C witness |
| host-is-not-target | build tool H and target library H use the same package-instance pair | two role-scoped occurrences; types are incomparable and host availability grants no target edge |
| host-import-leak | target source imports an alias declared only in the host/build graph | graph-role-leak before module lookup or provider invocation |
| exact-git-offline | locked full commit and all verified materials are available without network | accept source selection; checkout directory is not identity |
| missing-git-offline | same lock but exact material is absent | frozen/offline rejection; no ref lookup, fetch, repair, or partial closure |
| future-type-import | source uses unapproved package/type-import spelling | current provider/language unsupported-syntax rejection; this contract supplies no fallback |

## Bounded source-only resolver outline

A first implementation can operate on supplied local packages and a prepopulated exact-commit Git
material cache. It needs no registry service, native library, M7 FFI, component runtime, or package
publication:

1. Validate canonical #168 manifest, lock, checksum, and material records before building indexes.
2. Build an exact catalog keyed by #168's manifest dependency selection tuple and authenticate each
   `(id, sourceSha256)` package-instance pair. A lock edge's `package` selects the record by `id`.
   Reject zero/ambiguous matches and contradictory duplicate instance records.
3. Starting from the explicit root, traverse dependency aliases in canonical byte order with an
   iterative work queue. Validate reachability, role, duplicate edges, and the package DAG before
   opening source roots.
4. Validate exact compiler/profile/target compatibility, then invoke #357 composition for every
   requested target/deployment policy. Supply an authenticated, finite, canonical graph with stable
   instance IDs, exact source identities, and dependency edges. Do not prune a declared dependency
   based on reachability, backend choice, or an unsupported feature predicate.
5. Bind each selected instance to one retained package source-root capability. Verify every declared
   regular source file and checksum without links, root escape, case substitution, or undeclared
   bytes before provider analysis.
6. Resolve internal relative imports within that source root. A later package-import frontend may
   map an authenticated alias and exported module entry to another selected instance; until then,
   package imports reject.
7. Sort complete module identities, assign dense private IDs, and provide semantics with the exact
   semantic domain, package instance, module path, declaration order, sources, and edges. Semantics
   alone seals nominal identities and compatibility.
8. Bind every later IR/build/cache result to the complete role-scoped instance and module graph.
   Revalidate cached inputs; a changed manifest, source digest, role, profile, target set, edge,
   visibility table, or host policy invalidates the result.

The initial resolver consumes only envelopes already accepted by #168. These are #168's exact
schema and canonical-input bounds, repeated for implementation routing rather than redefined here:

| #168 quantity | Exact inclusive bound | Resolver consequence |
| --- | ---: | --- |
| entire canonical package-release fixture, including LF | 65,536 bytes | reject before building indexes when #168 validation rejects the envelope |
| manifests / lock `packages` / SBOM packages | 16 each | a source-only role-scoped closure cannot select a seventeenth lock package |
| manifest `dependencies` / lock-package `dependencies` | 8 per package | traverse only accepted alias and lock-edge arrays |
| SBOM `edges` | 128 | structural array bound only; #168 explicitly does not make 128 duplicate edges a valid complete graph |
| manifest `files` | 16 per manifest | authenticate only the accepted ordered source inventory |

#360 introduces no competing lower package-fixture limit. The accepted #168 graph invariants,
including acyclicity, reachability, unique identities, exact aliases, and canonical ordering, still
decide which structurally bounded records form a valid graph. The existing module-closure limit of
4,096 files and diagnostics limit of 256 remain independent downstream limits; they are not package
schema fields. #357's future limits of 256 instances including root, 4,096 edges, 32 edges in a
root-to-leaf path, and 65,536 total UTF-8 identity bytes likewise describe its later composition
input, not this #168 fixture.

Every count uses checked arithmetic. Validate #168's instance, edge, and byte limits before
materializing transitive summaries. Exact-limit and first-extra fixtures remain owned by #168;
#360 fixtures add semantic identity, graph-role, visibility, nominal-compatibility, and cycle cases
within accepted envelopes and reject atomically without exposing a truncated graph.
Independent fixtures must cover permuted catalog/discovery order, diamonds versus distinct
instances, duplicate aliases, malformed identities, dangling edges, shortest-cycle ties, local
root relocation, exact Git commit replay, private module/declaration access, graph-role leakage,
and all representative rows above. #360 must consume #168's validated values rather than fork or
reimplement its canonical serializer as an independent authority.

## Delivery and activation gates

This document is aligned to merged #168 field names, canonical ownership, and fixture limits, and
to merged #357 cross-target profile composition. It can be reviewed as a language/architecture
decision. That is **specified** state only.

A later source-only prototype must assign stable diagnostics and implement the bounded outline with
independent positive, negative, determinism, exact-limit, first-extra, race, and cache-replay tests.
Conformance additionally requires the approved package/type-import syntax and provider-neutral DTO,
cross-platform local/Git source proofs, nominal incompatibility cases, #357 profile/target cases,
and applicable repository gates. Public support separately requires explicit CLI/package-manager
activation, migration and support documentation, and authenticated release evidence.

No stage may claim package publication, registry trust, host-tool isolation, native dependency
linking, public aggregate ABI, or broader M3 syntax from this contract. Track **specified**,
**prototype**, **conformance-passed**, and **publicly-supported** as separate states.
