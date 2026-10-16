# Private real-UI qualification transport

This is a distinct trusted operator test tool. It is not part of the source-launcher
24-file closure and never grants a network or execution capability to a guest.

The root-reviewed design is recorded in the task artifact
`/tmp/M6_410_REAL_UI_STDIO_QUALIFICATION_PROPOSAL_20261015.md` (initial approval
SHA256 `41760ed35edf89d0dde3fa52d8f5db780f45bc59b303e15af4a0be0b7a473d77`).
The implementation currently covers the **initial abort/static/AX probe only**.
Corpus, Unicode diagnostic navigation, arbitrary inert diagnostic markup,
full keyboard/contrast/live-region cases and actual daemon cancellation remain
separate real-browser gates. Audible assistive-technology output is unrun.

`inner.mjs` must be installed as the fixed `/app/ui-driver.mjs`; copy its relative
module closure into `/app` unchanged: `browser-probe.mjs`, `browser-intercept.mjs`,
`framing.mjs`, `routes.mjs`, `wire.mjs`, `plan.mjs`. The sealed runtime supplies only
the independently approved `/materials/runtime/node/bin/node`, complete pinned
Playwright/Chrome closure, and required DSOs. `planBytes()` emits the exact compact
`/resources/ui-plan.json` bytes (root-owned ordinary 0444; resources directory0555).
The driver verifies this fixed image plan before loading browser tools and requires
the external operator start record's plan digest to agree. No argv, path or command
selector is supported. Separate image identity/config/kernel/supervisor review and
complete material audit must finish **before** transmitting the capability.

The parent operator owns the actual launcher and a distinct fixed-300-second UI
container. The production compiler/evaluator supervisor is unchanged. Guest active
budgets remain10/5seconds plus2seconds cleanup. The trusted UI object's external
removal/reap allowance is2seconds; unknown ownership/cleanup must poison the run.
The transport modules do not provision engine objects. The separate `owner.mjs`
trusted operator entry owns one creation-scoped container through the fixed local
Engine socket; it exposes no command, URL, socket or mount selector.

Use `createHostSession({configuration,send,resourcePins})` around the actual launch
origin/capability and independently reviewed fixed resource byte/SHA pins. The
operator obtains the launch URL once; do not log it. `begin()` sends one private
start record. Decode inbound UI stdout with `createFrameDecoder(session.accept)`;
with engine Tty=false first demultiplex its8-byte stdout/stderr headers using
`createEngineDemux`, bounding stderr separately. The synchronous callback must
handle a thrown protocol failure by stopping admission and executing owned cleanup.
Do not treat malformed/truncated engine or application EOF as a result; call
decoder.end(), demux.end(), and session.eof(). `send` must be a bounded serialized
frame writer to attached container stdin, attached before start. Both directions
must retain their aggregate budgets. Do not use a TTY, socket/tcp tunnel, HTTP
proxy, host network, engine socket mount, or generic command transport.

The broker forwards actual browser request headers only, without inserting missing
Authorization/Origin/Sec-Fetch-Site. It forwards actual response statuses, security
headers and bytes and never invents a compiler/session/evaluation response.
It has a fixed origin and closed route enum, bounded queue/inflight/total requests,
and finite byte/time budgets. Unknown routes, redirects, headers, workers or
noncanonical targets fail. Capability/header/job-token values never enter its
audit records. The caller must preserve this rule when storing raw protocol data;
**do not persist raw stdio, Playwright traces, HAR, console output or launch URLs**.

The first compile interception is deliberately held **before HTTP dispatch**.
Real UI keyboard compilation and Cancel must produce a genuine Chromium
requestfailed event while page.route is waiting. Only that event sends abort;
the host's matching transport acknowledgement releases the route. This proves
interception cancellation fidelity without starting a guest, delaying its budget,
or supplying a fake response. If Chrome does not produce the event within1second,
the initial probe fails with abort unproved; do not expand acceptance or claim
real daemon cancellation. A separately reviewed CDP Fetch adapter or a capable
native browser host is then required. The actual UI /cancel request still goes
to the real launcher/service. Response-versus-abort races acknowledge only a known
completed ID once and cannot acknowledge foreign/live ownership.

`session.result` validates the initial probe against actual transport audit/resource
hashes and records `fullQualification:false,containerCleanupConfirmed:false`.
Inner `browserClosed:true` does not establish container cleanup or guest cleanup.
The outer operator must stop new requests, await session.stop(), launcher.stop(),
owned container removal/reap, and verify absent ownership before persisting any
successful result. On error retain original failure, cleanup failure and actual
owner recovery. Actual Chrome sandbox/process/namespace evidence belongs to the
reviewed owner; `chromiumSandbox:true` alone is not a passing sandbox result.

`qualifyInitialUi({tools,approval,launcher,resourcePins,signal})` is the private
operator integration entry. Review all its source and exact immutable UI image,
image layer, seccomp bytes, material nonce/name/CreatedAt/inventory and UI-plan
digest before calling it. `approval` uses the closed
`zryna.playground-ui-owner-approval.v1` schema. The image must already exist;
this entry cannot import/build images or populate materials. The only Engine
endpoint is `/var/run/docker.sock`, API1.49, with actual peer UID65534 checked.
Creation uses the closed nonroot/read-only/net-none/cap-zero policy. A pre-start
duplex attach is opened before start. Complete fixed `/supervisor audit`, volume
provenance/alias checks and actual host kernel observations precede `ready` and
the first private start/capability bytes. A fixed readonly `/ui-supervisor
proc-audit` exec also verifies the same container's proc protections before the
first full host kernel capture. Kernel observation failure is fatal;
unavailable process-tree or reap metadata is never treated as a pass. Descendant
containment uses the complete visible `/proc/*/stat` PPid inventory including
zombies, bounded to8192numeric entries,16KiB per stat and the original deadline.
The observer verifies a full proc mount without hidepid and its actual PID
namespace. Owned process start identities, all thread memberships and complete
cgroup process/thread/task counts bracket that scan. An owned escaped descendant
or unreaped zombie fails; unrelated host zombies are counted without being
misattributed to the UI owner. Missing per-task `children` files are not needed.
The cgroup coordinate reader supports a canonical full-root mount or the exact
observed managed-cloud sibling profile. The latter requires observer membership
`0::/`, read-only cgroup mount root `/..`, the actual observer PID in
`/sys/fs/cgroup/init/cgroup.procs`, and target membership exactly
`0::/../docker/<owned-container-id>`. It maps that one literal shape to
`/sys/fs/cgroup/docker/<owned-container-id>` and retains the actual observer
mount/membership readbacks. It never normalizes arbitrary parent components.
An earlier sanitized `reserved` record identifies the exact random owner name
before create, so recovery retains it even if creation or audit acknowledgement
is lost. Ready and closed records must bind to that same reservation.
The structural mount reader selects an effective stack layer through
mount-ID/parent-ID ancestry. It recognizes the retained compiler's two-layer
read-only `/proc` remount for structural fixture replay and the three fixed
Docker `/etc` files masked by the same `/dev/null` tmpfs.
Filesystem/device/root/flags and original container-file identity must agree.
Row order and mount-ID numeric order do not select the effective layer. Cycles,
disconnected parents, hidden lower-layer children, extra layers and every
protected image/material/scratch stack fail. The retained actual compiler
mountinfo fixture tests structural parsing only; its numeric tmpfs sizes fail
the UI profile. UI proc/numeric fixture changes are explicitly synthetic.
The fixed UI runtime requires a single writable proc mount with nosuid/nodev/noexec
and exactly the five requested read-only same-proc binds `/proc/bus`, `/proc/fs`,
`/proc/irq`, `/proc/sys`, `/proc/sysrq-trigger`. Each actual bind must have the
same proc device/source/superblock, exact root and direct parent, and read-only
nosuid/nodev/noexec flags. Missing, writable, replaced or overmounted protections
fail. This fixed profile permits Chromium's own user-namespace uid/gid mappings;
whole-proc read-only prevents those required writes. UID65534, capabilities,
NNP, seccomp, private namespaces and all numeric budgets remain fixed. The caller
cannot select another proc policy. Compiler/CLI whole-proc protection is separate.
When the kernel has no `/proc/sysrq-trigger` interface, the missing fifth mount
requires an original target-bound proof from the fixed native proc audit exec.
The native mode uses no-follow O_PATH descriptors and procfs/statfs checks;
it accepts only actual ENOENT or a present ordinary proc file on a read-only
mount. It checks all four always-required proc directories as read-only procfs.
EACCES, ENOTDIR, symlinks, other errors and contradictory host mount readbacks
fail. The fixed five requested ReadonlyPaths remain unchanged. The proof binds
the actual container and exec identity/zero exit through an internal object;
plain JSON, copies and foreign-container proofs cannot authorize absence. Original
PID/start identity brackets the audit. Its two-second watchdog is an inner fuse
within the existing UI lifetime, with no new permission or renewed allowance.
Docker's observed `OomKillDisable:null` is accepted only as the representation of
the explicitly requested false setting; true, numeric zero and absent fields fail.

`tools` is independently selected operator input, not an authentication result.
It has `python`, the exact thirteen `helperNames` file descriptors, and
`runtime:{version:'3.12',files:[...]}`. Each runtime entry has
`{name,path,bytes,sha256,ownerUid,executable}` and a canonical private name under
`lib/python3.12/`. Select the complete reviewed standard-library/native-extension
closure required by these fixed helpers, including encodings and import
dependencies. Every byte is retained and hashed before capture; limits are1024
runtime files and128MiB including the interpreter. The ELF interpreter is captured
as `bin/python3.12`, with isolated/site-disabled flags. The helper rejects a
different prefix and fixes `sys.path` to the private helper/stdlib/lib-dynload
directories. Site packages, bytecode caches, inherited Python selectors and
ambient standard-library fallback are excluded.
The fixed empty `lib-dynload` directory is created even when every selected native
extension is built into the interpreter, so CPython derives the private exec prefix.
Host ELF loader and native DSOs remain an explicit trusted host runtime dependency and require independent
operator review; the selected inventory does not authenticate itself. An
incomplete closure fails before Engine ownership rather than searching elsewhere.
An exactly empty `.py` package initializer is retained as zero bytes through a
separate non-executable stdlib capture path. It checks no-follow ancestry and FD,
ordinary file identity, ownership, mode, single link, empty EOF and SHA256; it
does not admit empty DSOs/executables or change production material capture.

The owner uses a monotonic300-second lifetime and one memoized two-second cleanup
allowance. Transport/launcher teardown and owned container teardown run
independently. A protocol failure does not settle the true cleanup acknowledgement.
Internal start/stop envelopes carry absolute CLOCK_MONOTONIC nanosecond deadlines
from `process.hrtime.bigint()` to `time.monotonic_ns()` on this Linux host; neither
helper admission nor stop renews those fixed allowances. The parent keeps stdin
open until the actual cleanup acknowledgement so EOF cannot race a stop deadline.
Success requires the real validated closed receipt, actual helper `close` after
stdio drain, nonforced removal/404 and kernel init/cgroup reap readback. Unknown
ownership poisons the attempt, preserves the captured helper and records recovery;
late acknowledgements cannot clear it. A launcher can enter this integration only
once. EOF, abort, malformed attach/framing and audit failures all trigger cleanup.
`dockerRemoved` records the Engine404 independently of `kernelReapProven`.
A provisional original PID/start-time/membership is retained before detailed
kernel capture. If that identity cannot be observed, even successful Docker
removal leaves kernel cleanup unproved and the owner remains unknown; it cannot
produce a successful qualification cleanup acknowledgement. A never-started
container or absent create attempt is explicitly identified rather than being
reported as a reaped process.

`session.stop()` starts owned HTTP socket cleanup independently of a stalled
attached-stdin writer and has one two-second deadline shared by concurrent callers.
It retains both the primary failure and cleanup failure. The outer owner must use
independent cleanup/finally paths: always attempt launcher teardown and owned
container removal even when session.stop() rejects. An expired transport cleanup
deadline is not permission to forget the container. HTTP slots remain occupied
until the actual ClientRequest close event, including after abort/error, so rapid
cancellation cannot exceed the four-owned-socket limit.

Local source checks (no Chrome/container/compiler acceptance):

```sh
. /workspace/.cloud-setup/activate.sh
node --test examples/playground/restricted/daemon/ui-qualification/*.test.mjs
PYTHONPATH=examples/playground/restricted/daemon/preparation:examples/playground/restricted/daemon/ui-qualification python3 -m unittest discover -s examples/playground/restricted/daemon/ui-qualification -p 'test_owner*.py'
node scripts/check-repository-structure.mjs
```

The real HTTP tests use the actual product service and real local sockets with
explicit inert host stubs; host compilation/evaluation must remain uncalled.
They prove transport boundaries only. A zero-error test run is not a genuine
browser/publication/source/guest qualification result.
