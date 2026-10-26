# Private Windows Job metrics calibration

All eight fixed controls passed on Windows Server 2022 x64 at source
`0129cb9adfebef6f225d50b8369a53c2cff8e81c` in
[run 40381078171](https://github.com/zryna/zryna/actions/runs/40381078171).
Both explicit C17 builds passed `/W4 /WX` with compiler file version
`19.44.35229.0`, linker `14.44.35229.0`, MSVC tools directory version
`14.44.35207`, Windows SDK `10.0.26100.0` and Node `22.22.1`.

The retained artifact is `windows-job-metrics-40381078171-1`, ID `12677583631`,
with archive SHA-256
`a67cb685e17ff7468d7a1a40e0a35417fc00aab7f66b3b214ed235c89ff577f2`.
It contains source/tool observations, build commands, executables, raw streams,
original-handle ownership records and all eight case results.

The controls cover UTF16 argv, successful completion, nonzero exit, timeout,
exact 16 MiB combined output, its first extra byte, descendant-held EOF,
descendants with stripped stdhandles, and concurrent committed memory. The
success control also owns the argv check, giving eight cases in total.
Five cases produce valid measurements; the three deliberate failures retain
invalid results and no memory sample. Every outer supervisor observed its Job
empty before cleanup and required no forced termination. Original process
handles and creation timestamps bind the exit observations. The overlapping
32 MiB and 64 MiB allocations distinguish aggregate Job commitment from each
individual process peak.

The helper reports peak committed bytes charged to one Job and QPC wall time
covering launch, execution and owned drain. These finite trusted controls qualify
that instrumentation path. Actual workload collection, exact build/toolchain
admission, supported-host release baselines, reviewed variance thresholds and
held-out regression checks remain separate #418 acceptance work. Baseline and
threshold authority remain unset.
