# Private Windows Job metrics calibration v1

This private calibration is Windows x64 only. The first eight live controls passed
with both MSVC /W4 /WX builds; the exact source, toolchain and retained evidence
are recorded in [the qualification note](../../../../docs/WINDOWS_JOB_METRICS_CALIBRATION.md).
The current source inventory binds the reviewed helper and fixture, including
their measured integration fixes. Rust unsafe policy, Cargo, compiler behavior,
the helper900s/16MiB/10s envelope and public support remain unchanged. Workload
collection requires separate toolchain admission and review. Baseline and
threshold authority remain unset.

## Actual build/run required later

Record reviewed C source/tool/linker/SDK identities and full commands/logs, then build the original helper and this fixture into DISTINCT fresh private output directories using their respective build-msvc.cmd recipes. The fixture reuses only ../portable.c and portable.h for exact UTF16 quoting/checked primitives; its explicit eightC-source list plus that portable source has no additional dependency. Kernel32 supplies the PSAPIv2 K32GetProcessMemoryInfo call; no Rust/C#/PowerShell FFI, downloaded native dependency or helper mutation is introduced. /Brepro is a recipe option, not proof of reproducibility.

With pinned Node22.22.1 on actual Windows x64:

```
node scripts/stability-gates/windows-job-metrics-v1/calibration/calibrate.mjs --helper C:\private\helper\windows-job-metrics-v1.exe --fixture C:\private\fixture\windows-job-calibration-v1.exe --out C:\private\fresh-calibration
```

Use trusted existing private ancestors and a fresh output OUTSIDE the checkout. All generated files/directories are create-only and retained, including failure. Existing output is refused; there is no recursive cleanup/retry or output fitting. Executable admission has a private32MiB/file ceiling and source reads2MiB/file; these are evidence-tool bounds, not source/compiler quotas. The source manifest captures all13reviewed helper files and every calibration source except the manifest itself, which is separately hashed before/after. Tool/source byte and filesystem identities identify observations, not a compiler build signature or complete input-use attestation. Capture the actual compiler/SDK/build closure separately.

## Eight frozen controls

1. success0 also checks empty/space/quote/trailing-backslash/Unicode argv through actual Windows CRT; stdout/stderr exact fixed bytes.
2. exit17 must be invalid with exact original main17; raw prefix only, never a successful sample.
3. timeout5s requires both original processes observed and READY before the deadline, then invalid timeout/forced inner-Job termination and BOTH original handles gone BEFORE outer cleanup. No expired-before-entry or later fixture/outer cleanup credit.
4. exactly8MiB stdout+8MiB stderr is a valid combined16MiB boundary.
5. one additional stdout byte must be invalid overflow; retained raw files never exceed combined16MiB.
6. main exits while an acknowledged descendant owns pipe writes; helper must wait until explicit release, actual EOF and Job drain.
7. descendant inherits only owned NUL stdhandles. Pipe EOF alone cannot suffice while that acknowledged child remains alive/held.
8. two children commit/touch32+64MiB then READY and HOLD until explicit RELEASE. All three processes are alive during real per-process peak-COMMIT snapshots. Helper Job peak must≥96MiB and exceed every observed process peak; those peaks must be below96MiB so a process-peak substitution is distinguishable. Otherwise this host's calibration fails/inconclusive, never silently changes a bound. PeakPagefileUsage is a comparison witness only, never the reported Job metric/RSS.

Every process publishes PID+creationFILETIME and waits for observation-ack. Csupervise opens and binds the original process handle before ack; it never infers absence from a PID lookup after termination. For intentional parent-exit cases lifetime ownership transfers to supervisor+helperJob before main exits. READY QPC values enforce actual entry before helper expiry. The250ms held observation does not assume memory overlap: both memory allocations are explicitly held throughout all snapshots. Release/exit-marker/helper-end QPC order adds an independent finite timing witness. No finite calibration is claimed to prove every scheduler/kernel behavior.

## Ownership, errors and strict replay

The Node parent invokes only the same C fixture's supervise mode. Cowns one fresh noninherited outer kill-on-close Job and starts helper via JOB_LIST. Helper owns its reviewed separate inner measured Job. Descendants inherit both; no breakaway flags. Every launch uses an exact stdhandle HANDLE_LIST, and control mapping/events are opened noninheritable. An unexpected owner error or Node timeout closes outer ownership and terminates its tree; no detached helper is introduced. Original-handle before-cleanup flags remain immutable even when cleanup later kills a process.

Helper deadline is fixed5000/10000ms per frozen case. Cowner awaits helper within that+10000ms and owns a separate10000ms cleanup reserve. Node bounds the entire Cowner invocation to helperDeadline+20000ms, including bootstrap/report; an unfinished/error case is retained as failed, not retried into pass. Synchronous kernel/file calls are not magically preemptible. Root must qualify that containment on Windows; no universal hostile-kernel/no-hang guarantee is inferred.

The parent retains actual Cstdout/stderr/status BEFORE assertions. Flat JSON replay rejects duplicates, unknown/truncated/nested/escaped or wrong-type data, preserves exact64bit decimal strings, and requires helperexit0 plus complete fields for a valid measurement. Expected invalid helper samples never become measurements. Original main DWORD is compared with the retained original process handle; there is no SIGILL/source-trap or error-message inference. Raw files are read with fixed64KiB buffers and bounded identity/size/hash checks, with exact text/prefix/repeated-byte oracles. No base64/whole16MiB buffer expansion. CRT diagnostic LF/CRLF alternatives are allowed only as fixed diagnostic prefixes, never as authority or failure classification.

The strict source/expected/tool records and final eight-case results remain local observations: sourceAuthenticated=false, proofQualified=false, baselineApproved=false, thresholds=null, closureStatus=blocked. Build qualification, same-Job API/cancellation failure controls not in this finite packet, real selected workload methodology/repeats, host provenance, reviewed baselines and independent thresholds remain separate. Do not label this packet or prior Linux portable units an actual Windows result before root runs it.
