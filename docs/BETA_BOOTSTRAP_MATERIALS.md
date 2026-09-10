# Beta bootstrap material evidence

Status: material audit supporting the [distribution contract candidate](BETA_DISTRIBUTION.md).
This records observed upstream/package bytes and outstanding verification. It does not certify
an executable, finish license review or activate a supported host/version.

## Pinned package closure

The [workspace lock](../pnpm-lock.yaml) pins `@typescript/typescript6` 6.0.2 and its compatibility
dependency to TypeScript 6.0.3. Registry version metadata and downloaded package tarballs were
checked against those exact SHA-512 integrities. The two registry records declare Apache-2.0:
[@typescript/typescript6 6.0.2](https://registry.npmjs.org/@typescript/typescript6/6.0.2) and
[typescript 6.0.3](https://registry.npmjs.org/typescript/6.0.3).

The following hashes cover exact extracted bytes, without newline conversion. The four executable
package-file hashes agree with the existing tooling capture constants in
[`capture.rs`](../crates/zryna-driver/src/diagnostic_sessions/tooling_execution/capture.rs).
The package tarballs were inspected as data; no package scripts or compiler code were executed.

| Package | Member | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| wrapper 6.0.2 | `package.json` | 841 | `d9b8fb53c67c947fece83bcb73fa8512227ea6a12b110e096fdab8ecdc02b655` |
| wrapper 6.0.2 | `lib/typescript.js` | 45 | `d3f3cd2b04b7f466f4484df921b744223f7bd1f3e353ec9110bdf52695b983d5` |
| wrapper 6.0.2 | `LICENSE.txt` | 9197 | `a7d00bfd54525bc694b6e32f64c7ebcf5e6b7ae3657be5cc12767bce74654a47` |
| implementation 6.0.3 | `package.json` | 3527 | `9332e97c30d3e53ed54910b89207ed657fb444066484df6e5b6965bf130865e9` |
| implementation 6.0.3 | `lib/typescript.js` | 9144216 | `569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39` |
| implementation 6.0.3 | `LICENSE.txt` | 9197 | `a7d00bfd54525bc694b6e32f64c7ebcf5e6b7ae3657be5cc12767bce74654a47` |
| implementation 6.0.3 | `ThirdPartyNoticeText.txt` | 37824 | `1af3c68039c57e539422da82a4faada506ce6d0ea6f90e0b699d02dbcdb7a90c` |

Preserve both license files and the implementation's full third-party notice in the finite
distribution inventory. Package metadata's license label alone does not inventory incorporated
material. The exact five repository worker/limit modules must be hashed from the release source
revision, not copied from this audit's revision or rebuilt from a different checkout.

## Node material and license evidence

The exact Node version remains 22.22.1. The upstream
[release checksum list](https://nodejs.org/download/release/v22.22.1/SHASUMS256.txt) records:

| Original material | SHA-256 |
| --- | --- |
| `node-v22.22.1-linux-x64.tar.xz` | `9a6bc82f9b491279147219f6a18add1e18424dce90d41d2a5fcd69d4924ba3aa` |
| `node-v22.22.1-win-x64.zip` | `877cb93829e14fffbbc7903e7d8037336c9a79f3ea43c5d0b8c2379b79da56de` |

These are observed upstream archive hashes, not extracted executable hashes or completed upstream
signature verification. Release acquisition must authenticate the checksum/signature chain using
reviewed signing identities, verify original archives, then record exact extracted Node executable
bytes and the finite license inventory. No executable pin may be inferred from a filename or
`--version` response. Unresolved signature evidence blocks trusted material admission.

The full tagged [Node LICENSE](https://raw.githubusercontent.com/nodejs/node/v22.22.1/LICENSE)
includes upstream and incorporated third-party terms. Its observed SHA-256 is
`c738ae413cf561f174e34f6961f8ca458aae2369a73640dda6234c629b98bcc4`.
Preserve the complete license text and compare it with the actual selected archive's license
material before redistribution. A short MIT label is not a replacement for those notices.

## Host qualification proposal

Node's exact tagged [build/platform contract](https://raw.githubusercontent.com/nodejs/node/v22.22.1/BUILDING.md)
places GNU/Linux x64 at kernel 4.18 and glibc 2.28 or newer; its official x64 binaries require
libstdc++ 6.0.25 (`GLIBCXX_3.4.25`) or newer. Windows x64 is listed from Windows 10/Server 2016.
These are upstream constraints, subject to that document's vendor-support policy, not proof that
the complete Zryna archive runs at those floors.

Initial qualification should use explicitly pinned Ubuntu 24.04 x86-64 and Windows Server 2022
x64 hosts, record their actual image/kernel/libc or OS build, and inspect both produced CLI and
upstream Node loader dependencies. These are proposed test hosts within the upstream platform
families, not newly verified Zryna baselines. Do not translate `ubuntu-latest` or `windows-latest`
success into an older-OS support promise. WSL testing does not replace a clean native Linux host.

The final supported host set must be no broader than the intersection of actual compiler/runtime
imports, pinned build environment, upstream requirements and clean offline installation evidence.
If the clean compiled CLI needs a newer system dependency, raise the declared floor and rerun
qualification; never search a project-controlled directory for a substitute library. No additional
redistributed DLL/shared library is admitted without an explicit inventory/license review.

## Outstanding release evidence

- Authenticate upstream Node signatures, verify selected archives and extracted executable hashes.
- Inspect ELF/PE imports and loader search behavior on each intended host; freeze exact host images.
- Inventory all linked Rust dependencies and their source/license/notice materials from the exact
  target build, including target-specific dependencies. Neither Cargo.lock alone nor a generic
  license label proves the final linked material set.
- Check complete Node and TypeScript incorporated notices against actual redistributed bytes;
  resolve any source/notice obligations before publication.
- Record source, upstream archive, extracted-member and license relationships in the production
  material lock/SBOM, with no mutable locators or undeclared executables.
- Perform independent clean builds and offline install/project/run/tamper verification on both
  qualified hosts. No archive build or execution evidence is supplied by this material audit.
