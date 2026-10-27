# Windows workload collection draft

`captureWindowsDraft({document,capture,oracles,root,output})` supplies Windows IO
to `collectWindowsRound`. The frozen v4 catalogue defines 20 cells: four original
families, three Build targets and JavaScript/WebAssembly Run targets. Each retains
one warmup and five measured attempts in catalogue order. Original entries, argv,
stems, typed outcomes and M3 staging remain fixed. The collector neither learns
goldens nor selects samples, retries or deletes outliers.

Capture uses `zryna.cli-performance-windows-capture.draft.v1`: selected source
commit/tree, catalogue/oracle/review hashes, release/IO build class, host, explicit
bounded environment, guards and exactly helper/cli/node tool pins. Paths are
normalized local drive-absolute Windows paths through trusted private regular
ancestors; the Node pin must identify the actual interpreter. No inherited
credential environment is forwarded.

The hosted owner creates a canonical
`zryna.cli-performance-windows-operator-binding.v1` record containing those tools,
source and catalogue/oracle/review hashes, plus the existing helper source-pin
identity. Admission and all 29 helper/calibration files retain their byte and
persistent identity checks around every command, including failures. Binding
records supply no approval booleans or arbitrary workload options.

The independently frozen oracle array contains all 20 ordered selectors, exact
public stdout/stderr and complete sorted owned file inventories including each
manifest. Outputs must decode to the original typed response. Every attempt
checks streams and artifacts before deleting only its verified owned bundle.
M3 staging is create-only, its original graph/copies remain checked, and removal
follows complete verification. Partial stages and evidence remain inspectable.

The invocation is `helper --timeout-ms GUARD --output FRESH_SAMPLE -- CLI FIXED_ARGV`.
The helper owns a no-breakaway Job with KILL_ON_JOB_CLOSE and retains result.json,
stdout.raw and stderr.raw in the fresh sample directory. Node preserves the outer
outcome/streams and readable original bytes before parsing or asserting validity.
Oversized original helper files remain retained. The strict helper keys/types and
flat protocol remain unchanged.

Nonzero exit, timeout, overflow, spawn failure, forced termination, unknown EOF,
missing main exit, incomplete cleanup, nonempty Job, failed peak query, invalid
metadata or correctness/material drift invalidates the round. A taskkill-only
outcome cannot certify success. Child streams remain bounded at 16 MiB, cleanup
reserve at 10 seconds and the generic positive host guard at 900000 ms. Hosted
collection fixes 15000 ms. The outer guard adds only cleanup plus a 1-second
reporting margin; an outer timeout leaves closure unknown and is invalid.

Memory is actual PeakJobMemoryUsed Job committed bytes, never RSS, working set
or summed process peaks. QPC start/end/frequency/delta stay canonical unsigned64
decimal strings, including values beyond Number precision. Exact arithmetic and
guard checks precede summaries. Five samples yield native tick medians/maxima and
frequency, plus exact numerator/denominator spread; no float conversion is claimed.

The helper's five valid and three invalid controls, and both #576 Windows workload
rounds, completed on actual hosted Windows. Each round retained all 120 attempts;
the two builds reproduced their complete release CLI bytes. Collection success
and [retained replay](CLI_PERFORMANCE_ARCHIVE.md) are distinct from source/build/
host/oracle admission. The [platform link](CLI_PERFORMANCE_PLATFORM_LINK.md) checks
same-run execution and artifacts separately.

Sessions retain sourceAuthenticated=false, performanceQualified=false,
baselineAuthority=null, thresholds=null and closureStatus=blocked. Debug class
can qualify IO only. Baseline/held-out labels grant no authority. Reviewed host
and variance policy, immutable baseline/threshold anchors, a fresh held-out
candidate and complete M7 security/resource/closure evidence remain required.

The [original packet](https://github.com/zryna/zryna/blob/c1ae34fe421e3dfe63d1182054c9753ea2128dad/docs/CLI_PERFORMANCE_WINDOWS_DRAFT.md)
preserves its pre-execution author record and initial field enumeration.
