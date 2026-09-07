# Package and release contract v1

Status: contract-only foundation for [#168](https://github.com/zryna/zryna/issues/168),
within M5. The checked implementation validates synthetic fixture records only. It provides no
package resolver, package-manager command, download, installation, registry publication, signing
operation, release activation, or executed clean build. Existing releases, public support, CLI,
workspace registration, output manifests v1–v3, and completed M0–M3 behavior remain unchanged.

## Ownership and extension boundaries

This document owns package manifest/lock serialization and release provenance. The existing
`zryna.workspace.json` owns repository architecture; compiler output manifests own create-only
driver bundles. None is interchangeable. Packages declare source inputs; semantics still owns
nominal types and verified IR; the driver still owns compilation, toolchain checks, linking and
atomic publication. A package declaration never authorizes execution or an architecture bypass.

[#356](https://github.com/zryna/zryna/issues/356) indexes the supplemental planning:

- [#360](https://github.com/zryna/zryna/issues/360) owns detailed package-instance/type identity,
  visibility, aliases at the language boundary, version coexistence and host/target graphs.
- [#361](https://github.com/zryna/zryna/issues/361) owns resolved source/build plans, cache identities,
  native recipes and linker inputs. The reproduction recipe here is a declaration, not that plan.
- [#362](https://github.com/zryna/zryna/issues/362) owns source namespaces, trust, execution isolation,
  optional registry protocol and trust exceptions.
- [#167](https://github.com/zryna/zryna/issues/167) owns WIT worlds and capability profiles;
  [#357](https://github.com/zryna/zryna/issues/357) owns cross-profile composition.

Those complete future implementations do not block this foundation. New features, version ranges,
pre-releases, feature selection, host tools as dependencies, profile composition and native library
acquisition require separately reviewed extensions. Unknown fields reject now; no empty extension
map can smuggle those behaviors into v1. Contract acceptance, implementation, conformance and
public activation are separate states. This document does not declare M5 complete.

## Wire authority and bounded fixture profile

The closed [JSON Schema](../../schemas/zryna-package-release-v1.schema.json) defines required
fields, primitive types, vocabularies and structural bounds. Its named `$defs` specify separate
`zryna.package.v1`, `zryna.lock.v1`, `zryna.sbom.v1`, `zryna.provenance.v1` and
`zryna.release.v1` records. The `zryna.package-release.fixture.v1` envelope joins manifests,
lock, release and inline materials for independent validation. It is not a compiler input format.
The schema uses JSON Schema draft-07; it makes no SPDX, CycloneDX, SLSA or signature-conformance claim.

Every object has exactly its required keys. Null, missing/unknown fields, duplicate keys, unsupported
versions, coercions and default insertion reject. Wire bytes are UTF-8 without BOM. All admitted
strings are ASCII; material content additionally permits LF. Consequently string lengths equal
UTF-8 byte lengths. Source material is never normalized. Fixtures intentionally use tiny textual
materials instead of archives or executable binaries; production acquisition/transport and larger
resource profiles require reviewed versioning before use.

Canonical bytes are compact JSON with object keys sorted by unsigned ASCII bytes, recursively,
no spaces outside strings, JSON string quoting with only required escapes, and one terminal LF.
Slash and printable ASCII characters are not escaped; LF inside a string is `\n`. Integers use
ordinary unsigned decimal without leading zeros, exponent, fractional part or negative zero.
Arrays retain the ordering below. Consumers reject noncanonical input rather than normalize it.
Byte comparison after bounded parsing rejects duplicate keys and alternate spellings. Maximum
container nesting is six, counting the root object as one; brackets within strings do not count.
The validator checks byte length and nesting before parsing, then schema before semantic graphs.

Hashes use lowercase hexadecimal SHA-256. Material hashes cover exact content bytes without
a domain prefix or added LF. Structured record hashes cover:

```text
UTF8("ZRYNA-PACKAGE-RELEASE-V1\0" + kind + "\0") || canonical_record_bytes
```

Here `\0` is one NUL byte and canonical record bytes include their terminal LF.
Kinds are `manifest`, `selection`, `source-files`, `lock`, `recipe`, `environment`,
`release-subject` and `release-note`. Domains are not interchangeable. No host path, insertion
order, mtime, wall-clock timestamp or random value enters canonical serialization.

| Collection | Strict ascending unique key |
| --- | --- |
| Manifests | domain-separated manifest digest |
| Lock packages, SBOM packages | package ID (manifest digest) |
| Source files, artifacts, expected/observed outputs | portable path |
| Manifest dependencies, package lock edges | alias |
| Compatibility targets | literal target name |
| Inline materials, provenance materials | material SHA-256 |
| SBOM edges | source package ID, then alias |
| Environment tools and variables | name |
| Signature declarations | key ID |

Recipe arguments retain invocation order, including repetitions. A collection that is a projection
of another authority must equal that projection exactly; sorting a forged subset is insufficient.
Root `lock.root` is explicit and is not inferred from collection order.

| Fixture resource | Inclusive limit |
| --- | ---: |
| Entire canonical envelope, including LF | 65,536 bytes |
| Container nesting | 6 |
| Manifests / lock packages / SBOM packages | 16 each |
| Dependencies per package / lock edges per package | 8 each |
| SBOM edges | 128 |
| Source files per manifest / artifacts / expected outputs / observed outputs | 16 each |
| Materials / provenance material references | 64 each |
| Bytes per source/artifact material, checksum size | 1,024 |
| Targets | 3 |
| Tools / environment variables | 8 each |
| Recipe arguments | 16 |
| Signature declarations | 4 |
| Name, license declaration, environment variable name | 64 ASCII bytes |
| Path, source locator, argument, environment value | 96 ASCII bytes |
| Release note text / rollback reason | 1,024 ASCII bytes |
| SHA-256 / Git commit / Ed25519 signature hex | 64 / 40 / 128 characters |
| Version / tagged version | 14 / 15 characters |
| Source epoch (seconds) | 4,294,967,295 |

Versions contain three decimal components, each 0–9999 with no leading zeros; the maximum string
is `9999.9999.9999`. Empty dependencies, recipe arguments and material content are allowed.
At least one package, file per package, artifact, target, material, pinned tool and signature
declaration is required. Other lower bounds follow the closed schema; semantic rules additionally
require the three environment variables below.

The budget suite independently pins every schema bound and tests exact/first-extra structural
values. Some structural maxima are intentionally looser than semantic graph constraints:
128 duplicate SBOM edges pass only the isolated array shape, never a complete graph. Full fixtures
separately prove achievable graph, wire, material, file, artifact, tool, argument, signature and
epoch boundaries. A first-extra rejection never authorizes a partial result.

## Package manifest, lock and deterministic resolution

A manifest contains format, lowercase portable package name, exact version, explicit source,
compatibility, complete file inventory and alias-ordered dependencies. Each dependency gives its
alias, package name, exact version and complete source tuple; ambient namespace lookup is absent.

Local sources carry a lowercase relative locator and an empty revision. Git sources carry one
canonical lowercase HTTPS `host/path.git` locator and an exact 40-character lowercase commit,
never a branch/tag. The admitted URL subset excludes userinfo, query, fragment, port, percent escapes
and alternate transports. The Git tuple is a recorded identity, not a claim that Git was fetched.
Local locators are relative to the declared clean reproduction root, never the verifier process cwd.
Moving that root does not alter identity. Moving a relative package location does alter identity.

Paths use lowercase ASCII letters/digits, slash, dot, underscore and hyphen, begin with an
alphanumeric character, and contain no empty, dot or parent segments, trailing segment dot or
Windows device stems (CON, PRN, AUX, NUL, COM0–9, LPT0–9, including extension forms).
Absolute paths, backslashes, case variants and file/directory prefix collisions reject.
A future material reader must obtain regular non-symlink files within the declared root and reject
undeclared files or unstable bytes before issuing this inventory. This in-memory checker proves
no filesystem race or sandbox property. Source-only v1 has no executable mode or link entry.

The package ID is the manifest digest. `sourceSha256` is the `source-files` digest of its complete
path-ordered checksum inventory. Each checksum's size and hash must match an independently supplied
material. This transport identity is not a language TypeId.

Deterministic resolution requirements, independently checked against a claimed lock:

1. Start with the explicit root and supplied manifest catalog; validate all records and source tuples.
2. Traverse declared aliases in ascending order. Select exactly one manifest whose name, version
   and complete source tuple equal the declaration. Zero matches or ambiguous matches reject.
3. Reject duplicate selection tuples even when their differing contents yield distinct manifest
   IDs. Multiple aliases may intentionally select the same exact record; different versions/sources
   stay distinct records. Their nominal-type compatibility is pending #360.
4. Every package must have exactly the lock's compiler version and profile, and cover every selected
   target. Version names alone never imply compatibility. Target lists are exact declared
   `javascript`, `native-linux-x86_64`, and/or `webassembly` vocabulary; this does not add support.
5. Reject cycles and unreachable catalog entries; emit all packages by ID and edges by alias.
   No installation order or producer traversal order becomes serialization authority.
6. A frozen request requires this exact lock, full matching manifest/source material set, and all
   checksums. Stale/missing entries, mismatched compatibility or tampered cached bytes reject.
   Offline/frozen operation cannot download, resolve a newer version, repair a lock or silently
   consult a registry. An explicit future update operation must produce a new fully verified lock.

There is no highest-version selection, semver range approximation, implicit feature unification,
cross-profile fallback or dependency deduplication by name. This conservative exact model provides
determinism now without preempting #360's broader compatibility decisions.

## Release, SBOM, provenance and signed-note declarations

The release record binds exact `v<root-version>` tag, immutable source commit, domain-separated
lock digest and complete artifact checksums. A tag is not enough: a future release consumer must
authenticate its immutable tag-to-commit relation externally. Changing that relation rejects.
The fixture checks format and root-version equality; it does not contact Git or attest that tag exists.

The custom SBOM contains exactly every locked package with its source digest and a bounded license
declaration, plus exactly every lock edge. Missing, duplicated, reordered or substituted components
reject independently of producer output. `NOASSERTION` is permitted; license text is a declaration,
not an audit or an external standard identifier claim.

Provenance binds the complete material digest set, builder identity, recipe/environment record
digests and fixed source epoch. Inline bytes independently rehash to each material ID. Pinned
compiler and toolchain hashes must name supplied material bytes. Extra material records are allowed
only as explicitly listed provenance inputs; omission or substitution rejects. Hash integrity
alone does not establish source/builder trust; #362 owns that policy.

The release subject is the `release-subject` digest of the release object with only `notes`
and `rollback` removed. This binds tag, commit, lock, artifacts, SBOM, provenance and reproduction.
The `release-note` payload is canonical `{subjectSha256, text}`; its digest is recorded by every
signature declaration. Key IDs are opaque SHA-256 fingerprints, algorithm is exactly `ed25519`,
and signature is exactly 64 raw bytes represented by 128 lowercase hex characters.

The fixture status is necessarily `unverified-fixture`, including well-shaped all-zero signatures.
The validator checks key ordering, payload binding, algorithm and width; it never verifies
cryptographic validity, key ownership, revocation, authorization or transparency evidence. A future
signed-release consumer must authenticate trusted keys out of band, verify signatures over the
exact canonical payload bytes, apply revocation/threshold policy and reject missing/invalid evidence
before trusting the release. A self-declared key or colocated checksum is not a trust root.
These are pending execution gates, not a signing service implemented here.

## Rollback records

Rollback is an immutable `proposal-only` record: `from` must match this release subject,
`to` must name a distinct prior release subject, `reason` records the motivation and
`authorizationSha256` references independently authenticated approval evidence. The checker
does not authenticate that evidence or prove the prior release exists.

Before any future rollback activation, a consumer must verify the prior release, its own signed
notes, artifact integrity, current-active subject and authorization for the exact from/to/reason
proposal. A stale active subject, absent prior material or missing/invalid authorization rejects
before mutation. Use a create-only audit record for the outcome and preserve the previous active
state on failed preparation; do not rewrite historical release records. Activation transaction,
revocation policy and executed rollback outcome formats require a separately reviewed implementation.
Accepting this proposal never changes an active release.

## Clean-environment reproduction

The declared input closure consists of tag plus commit, exact manifests/lock, every source/artifact
material, compiler digest/version, named tool versions/digests, OS, selected targets/profile,
ordered executable/argument recipe, complete environment variables and fixed source epoch.
The environment is explicitly empty-workspace, empty-cache and network-disabled. Recipe executable
must name a pinned tool; arguments are literal ordered strings, never interpolated shell text.

Required variables are `LC_ALL=C`, `TZ=UTC` and `SOURCE_DATE_EPOCH=<provenance epoch>`.
Additional variables must be explicitly declared and sorted; no ambient environment is implied.
Absolute host paths and secrets are not suitable portable release inputs. The current fixture
validator does not run the recipe or enforce OS process/network isolation.
Native Linux target declarations reject a Windows reproduction environment.

For a future independently executed reproduction, materialize only the authenticated closure into
a fresh private root, prove workspace architecture with the pinned compiler before build/package
work, use admitted pinned tools without network/ambient cache, and run the complete required gates.
Reproduce twice in distinct clean roots. Compare every artifact's path, raw byte length and SHA-256,
and the canonical metadata; timestamps, absolute root paths, acquisition order and host locale must
not change output. Missing, extra or differing output rejects without publication or cache repair.
#361 supplies the resolved plan and #362 supplies enforceable execution/trust policy.

The fixture's `expected` and `observed` inventories must both equal release checksums exactly.
Its `verification` is always `comparison-only`: observed bytes here are synthetic supplied
materials, not evidence of a performed build. Successful checker output contains exact lock/subject
digests and `contract-fixture-valid`, `not-verified` signatures, `not-executed` reproduction
and `not-activated` release. Failure returns a stable rejection category and no accepted record.
Real reproduction evidence must retain exact revision, tool/platform inputs, commands, exit
statuses and independently measured outputs; this fixture result cannot substitute for that proof.

## Validation and acceptance evidence

The byte entrypoint is
[`validateFixture`](../../scripts/package-release/validate.mjs); it accepts only bounded wire bytes.
The [wire parser](../../scripts/package-release/canonical.mjs) and closed schema precede graph,
integrity, compatibility and cross-record checks. It returns only a frozen summary, never executable
authority or a mutable verified package object. Tests construct/rebind records with a separate
serializer and hashing implementation and include a committed fixed input/output oracle.

Stable rejection categories: `P168-BUDGET`, `P168-WIRE`, `P168-SCHEMA`, `P168-ORDER`,
`P168-PATH`, `P168-SOURCE`, `P168-LOCK`, `P168-RESOLUTION`, `P168-GRAPH`,
`P168-COMPATIBILITY`, `P168-INTEGRITY`, `P168-RELEASE`, `P168-SBOM`,
`P168-PROVENANCE`, `P168-REPRODUCTION` and `P168-ROLLBACK`.
The first failure in the documented phase order is returned. These fixture categories are not
new public compiler diagnostics.

| #168 criterion | Deliverable / evidence |
| --- | --- |
| Canonical manifest, lock, deterministic resolution and compatibility | Named schema records; exact graph, forged edges, cycles, source and transitive compatibility tests |
| Checksums, SBOM, provenance, signed notes and rollback | Cross-record validator; independent tampering, omissions, payload, tool and rollback tests |
| Canonical serialization and ordering explicit | Wire section, fixed `valid.json`/`expected.json`, malformed/duplicate/alternate-byte and collection-order tests |
| Exact and first-extra limits fixture-tested | `boundaries.test.mjs`: independent schema budget inventory and full graph/wire/material/environment boundary fixtures |
| Clean reproduction inputs and verification outputs specified | Clean-environment section, recipe/environment/material binding and expected/observed mismatch tests |
| Existing releases and support unchanged | Additive contract-only files; no compiler, runtime, public CLI, support registry or historical inventory edits |
| Focused tests, docs, preflight and M0 | Commands below; observed revision/platform results belong in the PR and hosted CI, not an invented pass in this specification |

Run from the repository root with its pinned Node/pnpm/Rust toolchains:

```bash
pnpm package:contract
pnpm docs:check
pnpm structure:check
pnpm install --frozen-lockfile
pnpm preflight
pnpm m0:check
```

An additive package-contract workflow runs the focused tests on Linux and Windows. Existing
documentation commands, digest-pinned preflight, M0–M3 and documentation inventories are unchanged.
This specification remains repository-only; it is not implicitly added to the website export
whitelist. Its M5 roadmap reference is additive. Required hosted checks and maintainer review
govern integration; fixture success alone does not authorize merge, closure or release.
