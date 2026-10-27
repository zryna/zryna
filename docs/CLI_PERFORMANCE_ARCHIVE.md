# Retained performance evidence

The #418 ZIP32 reader checks exact archive length/SHA-256, complete layout,
CRCs, descriptors, expansion and portable regular paths without case collisions.
It extracts no files and executes no data. Closed roles have fixed limits:

| Role | Entries | Archive / expanded bytes | Per entry |
| --- | ---: | ---: | ---: |
| Primary | 4,096 | 128 MiB | 64 MiB |
| Linux provider | 256 | 1 MiB | 256 KiB |

`openPerformanceArchive` returns an opaque selection after validation;
`inspectPerformanceArchive`, `listPerformanceArchive`, and
`readPerformanceArchiveEntry` expose copies. Input, inflated entries and returned
buffers use private unpooled storage. Close selections with
`closePerformanceArchive`. Caller-defined limits are rejected; M4 schemas and
archive profiles retain their existing limits.

`replayHostedPerformanceArchive(selection, artifactId, bytes)` requires a live
opaque platform selection and its primary artifact identity/digest. It checks the
independently frozen methodology, source/round/platform bindings and exact paired
outputs. All 24 Linux or 20 Windows cells retain one warmup and five measured
attempts; medians, maxima and spreads exclude warmup. Linux uses GNU wait4 maximum
RSS KiB; Windows uses native QPC and Job COMMIT bytes. JSON/log sizes are checked
before entry copies.

`reconcileSourceCensuses` checks before/after-build and after-round inventories
against a supplied complete recursive Git tree: paths, modes, blob identities,
file sizes, bounded shards and exact manifest/inventory digests. Its caller owns
Git response and commit/tree authentication. Source-file bytes are unavailable;
per-file SHA-256 values remain retained consistency claims.

`replayReleaseImage({host, expectedSource, read})` reconciles retained release
commands/results/logs, tool/version pins, controlled environment, Cargo config,
SDK records and the observed/final profile transition. It checks the whole
retained ELF/PE image, digest, copy links, and measured-round bindings using the
selected host's path rules. It executes no retained image or tool.

These checks establish consistency, not build/source execution, host or paired
oracle provenance. Qualification flags remain false and thresholds null. Reviewed
baseline policy, held-out admission and final M7 closure remain separate.
[Receipt storage](CLI_PERFORMANCE_RECEIPT_STORAGE.md) preserves complete original
bytes and inventories under their historical pins without adding authority.

`replayOracleArchitecture` checks both untimed attempts for every selector,
original process/stream and retained artifact bytes, frozen source graphs and
staging aliases, after-material claims and architecture records. Temporal staging
and before-material evidence is not retained. The opaque base64-compressed test dataset
contains only existing M2 golden preimages, bounded to 64 KiB expanded and checked
against their original hashes; it is never executed.
