# Bounded performance archive reader

This preparatory #418 reader validates complete ZIP32 layout, CRCs, exact archive
length/SHA-256, regular portable paths, case collisions, descriptors and expansion
before returning an opaque selection. It extracts no files and executes no data.
Input snapshots, inflated entries and returned reads use private unpooled storage;
editing an inspected report or an exposed backing ArrayBuffer cannot rewrite a
validated entry. Close each selection after use to release its retained contents.

Two closed roles have fixed limits. Primary artifacts allow 4,096 entries, 128 MiB
archive/expanded bytes and 64 MiB per entry. Linux provider supplements allow 256
entries, 1 MiB archive/expanded bytes and 256 KiB per entry. Caller-defined limits
are rejected. Existing M4 archive profiles, schemas and limits remain unchanged.

The API exports `openPerformanceArchive`, `inspectPerformanceArchive`,
`listPerformanceArchive`, `readPerformanceArchiveEntry`, and `closePerformanceArchive`.
The supplied archive digest must later be joined to the live platform selection.
Archive integrity alone authenticates neither source execution nor measurements;
reports retain false source/performance qualification and null thresholds. Raw
record/source replay, host policy and threshold/held-out admission remain separate.

The accompanying receipt-storage proof preserves all 24 historical Struct Clone
observations and reconstructs their exact original bytes under the unchanged old
pin. Its JSON whitespace reduction keeps this work inside the existing complete
source census budget; it adds no new execution or support claim.
