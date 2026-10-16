# Actual CLI parity qualification

This directory compares captured CLI streams with captured restricted playground
frames and scalar observations. Comparison results always have `acceptance: false`.
Neither an expected value, a caller callback, a receipt JSON object nor a cleanup
boolean authenticates execution, containment or publication.

`plan.mjs` fixes nine sources, nine CLI builds and ten CLI scalar runs. Each CLI
operation uses a fresh contained project with only `src/main.zry` and its generated
package-v1 manifest and lock, no dependencies, target `javascript` and the old
release's default `i32-v1` profile. The old release does not have the playground
browser component profile. No arbitrary source, project, executable, CLI flag,
package, native backend, provider override or environment selector is accepted.

`archive-plan.mjs` snapshots and hashes the exact original signed envelope and
52,949,082-byte archive, then applies the complete distribution verifier. Its
fragment includes all 349 files (154,023,815 expanded bytes), retaining every
original 0644/0755 file mode. The authenticated ten-file Node/provider subset is
insufficient to execute the installed CLI. Archive-plan verification consumes
fixed bytes; the operator separately reviews the retained actual offline Sigstore
verification, signer/source identity, complete runtime closure and precise image
configuration before any installed CLI or provider executes.

`producer.mjs` requires three independently reviewed adapter objects, each with
`run(input, context)` and `cleanup(context)`. The active deadlines are 10 seconds
for CLI, 10 seconds for playground compilation and 5 seconds for evaluation. Each
call gets an abort signal. Cleanup has a separate 2-second deadline and signal and
must settle to exactly `{ confirmed: true }`. It runs after success, rejection,
abort and timeout. Unknown cleanup stops the whole producer. Callbacks must yield
to the event loop; JavaScript timers cannot preempt inline blocking code. Concrete
adapters must additionally enforce the actual kernel process/resource/watchdog
limits using creation-owned handles.

`runtime-adapter.mjs` supplies the concrete CLI adapter. Its trusted bootstrap is
the fixed system Python interpreter and reviewed `cli_main.py`, with a held
control channel whose closure cancels active Engine I/O. It accepts only one of
the hash-pinned adjacent `fixed-plan.json` operation IDs. The operator separately
approves the exact immutable 360-file image/layer and unchanged compiler seccomp.
Every operation gets a fresh nonroot, networkless, read-only container with zero
capabilities, NNP, private namespaces, 512 MiB memory, 32 tasks and two CPUs. Its
CLI-only trusted idle PID1 has an independent 12-second watchdog and reaps children.
The full installed verifier retains at least 705 descriptors, so only this CLI
qualification role uses a fixed 1024-descriptor soft/hard limit, checked against
actual kernel readback. Production compiler and evaluator limits stay at 512.
Only `/work/project` (2 MiB), `/input` (2 MiB), `/output` (2 MiB), `/tmp` (64 MiB),
`/dev` (1 MiB) and `/dev/shm` (1 MiB) are writable tmpfs mounts. There are no named
volumes, host mounts or credential variables.

The owner records actual image/configuration, namespace, network, cgroup limits,
mount and original PID1 identities before source upload and after CLI execution.
One hash-pinned trusted Node bootstrap runs inside the admitted namespace. It
proves the actual project tmpfs is empty with UID/GID 65534 and mode 0700,
creates only the three opaque source files using exclusive opens, and reads back
their complete inventory, hashes, ownership and modes before CLI execution.
Docker archive APIs observe the image backing directory rather than this live
tmpfs, so their directory metadata cannot establish writable staging. Captured stdout and
stderr are separately bounded to 64 KiB, and actual CLI exit status is retained.
The complete visible-proc scan includes zombies and rejects denied reads. The
managed cloud cgroup mapping admits only the observed mount-root/observer-PID
profile; it never normalizes arbitrary parent traversal.

Cleanup uses one shared 2-second deadline for the original named/ID-bound
container, actual stop/wait, non-force deletion, 404, original-process absence and
empty/removed owned cgroup. Unknown cleanup remains sticky and prevents further
admission. The operator must reserve create-only result files before starting
effects and retain the adapter's complete evidence alongside comparison rows.
Source controls do not approve image import or CLI execution; independent review
of the exact current source/approval/image proof must precede either operation.

The recorder binds compilation revisions 1 through 9 to their fixed requests,
verifies complete frame and receipt correlation, rejects missing/duplicate/stale
rows, and requires exact CLI/playground diagnostic text, UTF-8 spans and scalar
identities. The legacy CLI emits a raw DiagnosticVec. After validating every raw
field and span, the CLI projection orders diagnostics using the unchanged
structured-v1 contract: path, byte start/end, severity, code, message and guidance,
with Rust Option and UTF-8 lexical ordering. It preserves duplicate counts and
retains original CLI bytes; the recorder still compares the complete projected
arrays exactly. It does not import or execute generated artifacts. The final comparison
still requires actual retained execution/cleanup evidence and an independently
authenticated published playground compiler before milestone acceptance.

Run the offline controls with the repository-pinned Node:

```sh
node --test --test-concurrency=1 examples/playground/restricted/daemon/cli-parity/*.test.mjs
python3 -m unittest discover -s examples/playground/restricted/daemon/cli-parity -p 'test_cli_*.py'
```

These controls use synthetic frames and streams and fake clocks. They do not run
the installed CLI, providers, generated components, native user code or Chrome.
