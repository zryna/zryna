# Downloadable prerelease installation and removal

Status: observed for the immutable `v0.2.1` beta and `v0.2.2` Developer Preview by the #423
clean-host matrices. The immutable `v0.2.2` release's public upgrade jobs failed because it
rejected the exact `v0.2.1` project compiler identity. The `v0.2.3` corrective release remains a
source candidate until its own public post-publication jobs produce canonical receipts. This page
does not broaden the supported hosts, create a system-wide installer, or make a prerelease stable.

## Supported clean hosts

| Archive target | Tested host | Portable targets | Additional native prerequisite |
| --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 24.04 x86-64, ordinary non-root user | JavaScript and core WebAssembly | canonical `/usr/bin/gcc`, x86_64-linux-gnu GCC 12–15, and GNU ld 2.38–2.46 |
| `x86_64-pc-windows-msvc` | Windows Server 2022 x64, standard non-administrator user, OS UCRT | JavaScript and core WebAssembly | none; native execution rejects with `ZRYNA-N4002` |

The archives bundle Node.js 22.22.1 and the TypeScript provider. JavaScript and WebAssembly use
neither a source checkout nor a separately installed Node, Rust, Cargo, or pnpm. Linux native
linking trusts the named system compiler, assembler, linker, CRT, libc, and loader; Zryna does not
download or substitute them.

## Authenticate before extraction

Download every asset attached to the immutable
[`v0.2.1` prerelease](https://github.com/zryna/zryna/releases/tag/v0.2.1), not only an archive.
Confirm that the GitHub release reports `immutable: true` and has exactly the asset allowlist in
`zryna-release-envelope-v1.json`. Verify the envelope, `SHA256SUMS`, release notes, both provenance
attestations, both SBOMs, and both archive inventories with the exact certificate identity and OIDC
issuer recorded in the envelope. The archive digests are:

```text
2d38dd9c09b7abcbef777fa115365f5f0b2a5670c4e39dcf9e1560f314c27aa3  zryna-0.2.1-x86_64-pc-windows-msvc.zip
f38ab0bed4d8874f80ebd7c5c39d403f2914100ceff37dc0651af8adc5b219cc  zryna-0.2.1-x86_64-unknown-linux-gnu.tar.gz
```

The checked verifier is `scripts/distribution-release/verify-signed-release.mjs`. It requires a
trusted Cosign and Node.js 22.22.1 with the archive recipe's exact zlib identity. Do not use an
executable from the unverified archive to verify that same archive. A checksum alone does not
authenticate who published it.

Tampered, truncated, wrong-version, and wrong-platform subjects are rejected before extraction or
execution. Extraction additionally rejects traversal, links, special files, case collisions,
Windows device names, unexpected modes, an existing destination, and any file not authenticated by
the signed inventory. Never repair, combine, or copy individual files between installations.

## User-local installation and PATH

Extract the authenticated archive into a new, empty, user-owned directory. Keep the single
`zryna-0.2.1-<target>` root intact and keep projects outside it. Do not extract over an existing
installation. The installation is relocatable as a whole.

Add only its `bin` directory to the current shell's `PATH`, or invoke the executable by absolute
path. For example:

```sh
export PATH="/opt/zryna-0.2.1-x86_64-unknown-linux-gnu/bin:$PATH"
zryna --version
```

```powershell
$install = 'C:\Tools\zryna-0.2.1-x86_64-pc-windows-msvc'
$env:PATH = "$install\bin;$env:PATH"
zryna.exe --version
```

Both commands must print exactly `zryna 0.2.1`. The clean-host matrix also repeats this check under
`tr_TR.UTF-8` locale variables and a sanitized environment that contains no compiler-checkout,
external Node, Cargo, or Zryna override.

## First external project

From a directory outside the installation, use the same project for every portable target:

```sh
zryna new hello
zryna build src/main.zry --project-root hello --target javascript --name js-build
zryna run src/main.zry --project-root hello --target javascript --name js-run --export main
zryna build src/main.zry --project-root hello --target webassembly --name wasm-build
zryna run src/main.zry --project-root hello --target webassembly --name wasm-run --export main
```

The run commands print `javascript: i32 42` and `webassembly: i32 42`. On the supported Linux host,
repeat with `--target native` and fresh `--name` values; the native run prints `native: i32 42`.
Output stays below `hello/.zryna/out`. Publication is create-only, so a repeated bundle name is an
error rather than an overwrite.

## Removal and recovery

The beta is a portable directory, not a privileged or system-wide installation. Remove only files
whose paths and bytes still match the authenticated archive inventory, deepest files first, and
then remove only empty owned directories. The #423 removal harness follows that rule and proves
that project source, project outputs, a sibling file, and an unrelated file placed in the
installation root remain unchanged. It fails before deleting anything if an owned file changed,
and it never recursively deletes an unresolved or nonempty root.

Remove the session PATH entry after removal. If installation admission reports changed or missing
bytes, stop using the installation, preserve user projects, and obtain and authenticate the whole
archive again. Do not replace the provider, bundled Node runtime, metadata, or CLI individually.

## Upgrade proof boundary

`v0.1.0` is source-only and `v0.2.0` has no published release, so `v0.2.1` is the first eligible
binary release. Candidate and fixture lifecycle results are not immutable upgrade evidence.

The `v0.2.2` release workflow downloaded the exact public `v0.2.1` and `v0.2.2` states and complete
signed asset allowlists on Ubuntu 24.04 and Windows Server 2022. Acquisition and authentication
succeeded, but both hosts then observed `ZRYNA-P4009` when `v0.2.2` tried to rebuild the unchanged
project created by `v0.2.1`. No canonical upgrade receipt was produced, so `v0.2.2` is not evidence
of a supported project upgrade.

Current `0.2.3` source admits only two reviewed predecessor transitions: `0.2.1` to `0.2.3` and
`0.2.2` to `0.2.3`. Exact-current identity remains accepted; every other compiler pair and every
profile, target, manifest, frozen-lock, and source-inventory mismatch remains rejected. The future
immutable `v0.2.3` release must run both transitions on both public hosts before either upgrade
claim is accepted. Always install a newer release into a separate empty directory, switch the
session PATH only after success, and retain the old installation for rollback. Never extract over
the old root.
