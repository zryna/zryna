# CLI performance collection draft

The frozen `tests/cli-performance-methodology-v4.json` defines four original
families, 24 Linux and 20 Windows build/run selectors. Its single reader pins the
complete canonical document, 27 selected source/material files and original import
graphs. Historical methodology versions remain review evidence; the v1 registry,
reader, runner and null thresholds retain their existing contracts.

Expected Run values are M1 -2147483648, M2 42, M3 29 and M7 97. Arguments,
profiles, source bytes, deterministic outcomes, one warmup and five measured
attempts are fixed. Run includes CLI compilation plus execution. M3 stages its
original main.zry/math.zry copies in a create-only ignored directory while --root
remains the compiler checkout; source rewriting is forbidden.

The Linux adapter `captureLinuxDraft({document,capture,oracles,root,output})` uses
the [portable provider](CLI_PERFORMANCE_PORTABLE_PROVIDER_PROPOSAL.md), whose root,
fixed recipe and actual receipt hashes bind the selected GNU executable. It uses
no caller-selected replacement or installation. Private absolute-origin v1
provider packets are historical IO qualification, not portable CI admission.

Hosted collection builds an actual locked offline release CLI, checks architecture
and measures its independently copied/rehashed single-link image. Debug builds
can qualify IO only. Explicit source, tools, runtime providers and environment are
rechecked; private output/cache leaves start absent. Raw statuses, argv, streams
and timing records precede semantic validation. Invalid sessions retain bounded
diagnostics and are not retried automatically.

Each selector first completes two untimed correctness/artifact replays with exact
owned cleanup between them. Typed original outcomes and existing artifact/source
audits fix both streams and complete sorted artifact inventories before timed
observations exist. Timed outputs cannot supply their own goldens. Original M3
staging bytes and import closure are checked before and after commands.

Linux uses hrtime.bigint around owned execution, including bounded EOF/descendant
cleanup before artifact validation/deletion. Nanoseconds imply neither CPU time
nor a precision guarantee. Memory is GNU wait4 maximum RSS KiB, not simultaneous
summed tree RSS. [Windows](CLI_PERFORMANCE_WINDOWS_DRAFT.md) instead uses native
QPC and Job committed bytes. Generic host containment is at most 900000 ms;
cleanup is 10000 ms and combined streams 16777216 bytes. The hosted operator
record fixes 15000 ms. Containment caps are not performance thresholds.

The four #576 jobs collected 528 attempts: 440 measured and 88 warmups, alongside
176 untimed paired invocations. Both host pairs reproduced their complete release
image byte-for-byte. These are unqualified observations, not baseline admission.
The [retained-evidence readers](CLI_PERFORMANCE_ARCHIVE.md) and
[live platform link](CLI_PERFORMANCE_PLATFORM_LINK.md) provide separate checks.

Draft reports retain sourceAuthenticated=false, performanceQualified=false,
baselineAuthority=null, thresholds=null and closureStatus=blocked. The declaration's
collectorImplemented=false describes absent admitted capability. Source/build/host
and oracle provenance, reviewed variance policy, externally frozen baseline and
threshold authority, genuine held-out verification, required resource/security
proofs and the final #418 report remain necessary. No label or hash grants them.
