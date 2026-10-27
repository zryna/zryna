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

`replayHostedPerformanceArchive(selection, artifactId, bytes)` joins a primary
archive to an opaque, live platform selection. It checks the selected archive's
length and digest, independently frozen methodology, source/round/platform
bindings, and all retained raw process records against session samples. Linux
uses GNU time wait4 maximum RSS in KiB; Windows uses native QPC timing and Job
COMMIT bytes. Each cell includes one warmup and five measured attempts, with
medians, maxima and spreads recomputed from the five measured samples.

The replay verifies all 24 Linux or 20 Windows cells per round and their exact
paired output bytes. JSON and log entry sizes are bounded before returned copies.
This establishes raw consistency with the selected platform artifact. Source
execution, host classification, paired-oracle provenance, baseline policy and
held-out admission still require their own verification; all qualification flags
remain false and thresholds remain null. Both compact receipt families retain
exact original-byte reconstruction under their unchanged historical pins.
