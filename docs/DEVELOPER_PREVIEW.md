# v0.1.0 Developer Preview policy

Zryna v0.1.0 is an experimental, source-based Developer Preview. It is not
production-ready. It provides the documented repository-local M1–M3 workflows; it does not
include standalone binaries, package installation, or stable compatibility guarantees.

This document defines the support and release contract for the proposed `v0.1.0` pre-release.
It does not publish a release, authorize a tag, or make work planned for M4 and later public.
The release exists only after the exact-revision checklist below is completed and Issue
[#421](https://github.com/zryna/zryna/issues/421) publishes the immutable tag and GitHub
pre-release.

## Preview support matrix

The supported input is a repository-local checkout of the tagged source. Users install the
pinned toolchains and invoke the compiler from that checkout with `cargo run --locked -p zryna`.
Every build and run retains the strict workspace check, verified-IR boundaries, explicit profile
and target selection, and create-only output publication documented by the
[CLI contract](CLI.md).

| Area | Advertised v0.1.0 boundary |
| --- | --- |
| Source and profiles | The bounded TypeScript-compatible syntax admitted by M1 when `--profile` is omitted, M2 with exact `--profile control-flow-v1`, and M3 with exact `--profile data-ownership-v1`. The profile contracts, rather than the TypeScript language as a whole, define accepted programs. |
| Public entry ABI | Exported entry functions accept and return only exact `i32` and `bool` where the selected profile permits them. M3 owned values and references remain internal to a program. |
| JavaScript | Deterministic ECMAScript-module build and typed run through the pinned Node.js runtime on Linux x86-64 and Windows x64. |
| WebAssembly | Deterministic core WebAssembly build and typed run through Node.js on Linux x86-64 and Windows x64. This is not browser, WASI, or Component Model support. |
| Native build | Audited Linux x86-64 ELF relocatable-object output. This output may be generated on Linux x86-64 or Windows x64, but it is not a host-native Windows object. |
| Native run | Audited Linux x86-64 executable linking and typed execution only on Linux x86-64 with the documented GNU toolchain. Windows native and `all` runs fail closed; macOS is outside the preview matrix. |
| Artifacts | Repository-local, create-only `.mjs`, `.wasm`, Linux `.o`, and invocation-specific Linux `.elf` files in complete build/run bundles. M1, M2, and M3 use manifests v1, v2, and v3 respectively. |
| Distribution | One immutable source tag, reviewed release notes, and source archives with recorded SHA-256 digests. No compiler executable, installer, package-manager channel, container image, or editor package is distributed. |

The detailed [M1](M1_CONFORMANCE.md), [M2](M2_CONFORMANCE.md), and
[M3](M3_PUBLIC_PROFILE.md) contracts remain authoritative when a summary above omits a resource,
syntax, diagnostic, or ownership limit. A summary cannot broaden those contracts.

## Required toolchains and hosts

| Requirement | Exact preview value |
| --- | --- |
| Rust | `1.97.1`, selected by `rust-toolchain.toml` |
| pnpm | `11.18.0`, selected by the root `packageManager` field |
| Node.js | direct regular executable, exact `22.22.1` |
| Portable verification hosts | Linux x86-64 and Windows x64 |
| Native link/run host | Linux x86-64 |
| Native linker toolchain | canonical `/usr/bin/gcc`, GCC 12–15, GNU ld 2.38–2.46 |

An installation that merely satisfies version output is not automatically supported: the
workspace, direct-executable, platform, filesystem, and native-toolchain checks in the compiler
contracts still apply. Repository-local setup is part of this preview contract.

## Compatibility, support, and security policy

`v0.1.0` identifies an immutable experimental snapshot, not a stable compatibility epoch. Later
preview versions may make documented breaking changes to source syntax, profiles, CLI spelling,
diagnostics, manifests, generated artifacts, or internal ABI contracts. Such changes require a
new version and release notes; the `v0.1.0` tag, archives, notes, and support matrix are never
silently replaced. Generated artifacts are supported only with the exact compiler revision and
profile that produced their manifest.

Public issue support is best effort and limited to reproductions inside the matrix above. Reports
must identify the exact source revision, host, pinned toolchains, command, expected behavior,
observed behavior, and relevant diagnostics. Unsupported configurations may still be useful bug
reports, but they are not preview portability claims. There is no service-level, uptime,
performance, production-migration, or long-term maintenance guarantee.

Potential vulnerabilities must be reported through GitHub's
[private security advisory channel](https://github.com/zryna/zryna/security/advisories/new), not a
public issue. The preview grants no ambient filesystem, network, clock, randomness, environment,
browser, WASI, Component Model, FFI, raw-pointer, thread, or custom-allocator capability. Native
execution invokes the explicitly documented local GNU toolchain and must not be described as a
sandbox. Source archives contain no prebuilt executable whose security has been independently
audited for distribution.

## Deliberate exclusions

The preview does not include standalone or system-wide installation, external project layouts,
package resolution or registries, dependency acquisition, editor or language-server tooling,
browser deployment, WASI/server applications, Windows or macOS native execution, static native
executables, tracing garbage collection, stable owned-value host ABI, general TypeScript
compatibility, production certification, performance guarantees, stable 1.0 compatibility, or
support for in-progress M4–M7 work. The [current status](STATUS.md) and individual profile
contracts list further language and resource exclusions.

## Exact-revision release checklist

The release owner records every result against one 40-character lowercase `main` commit. A result
from an ancestor, a different operating system, a test listing, or a run with zero selected tests
is not evidence for that candidate. Failed, skipped, cancelled, and unrun checks remain explicit.

1. **Freeze the candidate.** Confirm the root Cargo and npm workspace versions are `0.1.0`, the
   worktree is clean, the candidate is the current reviewed `main`, Issues #419 and #420 are
   complete, and neither `refs/tags/v0.1.0` nor a `v0.1.0` GitHub release exists.
2. **Record source identity.** Record the full commit SHA, tree SHA, `refs/heads/main`, toolchain
   files, lockfiles, and the URL of the immutable hosted workflow run. Inspect tracked files and
   proposed public text for credentials, private paths, unrelated artifacts, and unsupported
   claims.
3. **Run the local candidate gates.** From a dependency-installed checkout run `pnpm docs:check`,
   `pnpm preflight`, `pnpm m0:check`, `pnpm m1:check`, `pnpm m2:check`, and `pnpm m3:check` with
   the pinned toolchains. Record platform, command, exit status, and executed/ignored counts for
   each actual run. These commands do not substitute for the second required host.
4. **Verify hosted platforms.** On the exact candidate, require successful Linux x86-64 and
   Windows x64 preflight, Rust, adapter, M0, M2, and M3 jobs and every other branch-required check.
   Record each check name, conclusion, run URL, and candidate SHA; do not summarize a pending or
   skipped dependency as passing.
5. **Rehearse a clean source install.** In fresh Linux x86-64 and Windows x64 clones, check out the
   candidate by full SHA, install only the pinned dependencies, and execute the documented M1,
   M2, and M3 JavaScript quickstarts. Also execute WebAssembly on both hosts and native on Linux
   with the required GNU versions. Record commands, typed results, artifact inventories, and the
   expected Windows native rejection. Do not add a repository-local workaround to make the
   rehearsal pass.
6. **Freeze release material.** Prepare release notes using the exact introductory wording at the
   top of this document. Include the source/tree SHAs, support matrix, toolchains, profiles,
   artifact formats, security-reporting link, all exclusions, verification table, known issues,
   and rollback procedure. Produce only bounded source archives from the candidate and record
   each filename, byte length, and SHA-256 digest; reject unexpected executable content.
7. **Create immutable identities.** Create annotated tag `v0.1.0` at the recorded candidate, push
   it without replacement, and verify the remote peeled commit and tree before attaching the
   exact source archives. If the tag or release name already exists, stop rather than retagging,
   deleting, or overwriting it.
8. **Publish as a pre-release.** Create one GitHub pre-release whose title, notes, tag, archives,
   and hashes match the reviewed material. Export the semantic-version documentation bundle from
   the same commit with channel `0.1.0` and source ref `refs/tags/v0.1.0`; validate its manifest
   digest and publish the website announcement only after the release is immutable.
9. **Verify publication.** From an unauthenticated view, verify the tag, pre-release label, notes,
   archive hashes, security link, documentation manifest, website wording, and every public link.
   Repeat the clean-clone quickstart from the published source archive before announcing
   completion.

The release record must contain a table with the candidate commit/tree, tag target, hosted run,
each command and platform, exit/conclusion, executed and ignored counts where available, archive
hashes, documentation manifest digest, announcement URL, known issues, and anything not run.

## Rollback and supersession

An immutable release is not repaired in place. If verification fails before publication, publish
nothing and correct `main` before selecting a new candidate. If a material defect is discovered
after publication, mark the release notes and website announcement as affected, stop recommending
the preview, link the private advisory or public issue as appropriate, and preserve the tag and
archives for audit. Publish a fix only from a new reviewed revision under a new version and state
whether previously generated artifacts must be discarded. Never move or recreate `v0.1.0`, replace
its archives, or present a documentation-only rollback as a compiler rollback.
