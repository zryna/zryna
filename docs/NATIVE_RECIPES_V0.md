# Explicit native compile recipes v0

The driver exposes a typed Rust library API for one independently approved Linux x86-64
C compilation. This is a compile-only interface. It adds no CLI command or profile, package
manifest execution field, native cache admission, release publication, linking or loading grant.
The [source trust policy](../spec/package/SOURCE_TRUST_V0.md) and existing source-only package
contracts remain unchanged. #417 supplies the separate retained source/ABI/object prerequisites;
it does not grant recipe or host permission.

## Caller policy and independent approval

`zryna_package::SourceTrustMode` accepts exactly these caller-selected spellings:

| Mode | Native recipe disposition |
| --- | --- |
| `pure-source` | Forbidden; sources remain data for the trusted compiler. |
| `trusted-native` | Requires exact independent operator approval and actual host isolation. |
| `untrusted-playground` | Forbidden; no recipe or publication authority. |

Package content cannot select or upgrade the mode. Unknown spellings reject. Trusted-native
classification alone starts no process and proves no host enforcement.

The `zryna_driver` entrypoints separate planning, approval and actual execution:

1. `prepare_native_recipe(NativeCompileRecipeRequest)` retains an effect-free
   `NativeCompilePlan`. It binds the authenticated package resolution and source plan, frozen
   lock behavior, compiler/host/target/runtime/policy identities, declared C source and read-only
   material, one output, and genuine `NativeRecipeRequirements` from the existing verified FFI
   pipeline. A digest or raw JSON cannot construct that original source/ABI issuer.
2. `approve_native_recipe(plan, mode, NativeRecipeOperatorApproval)` returns an opaque
   `ApprovedNativeRecipe` only after checking independently supplied exact expectations.
   `prepare_and_approve_native_recipe` additionally rejects forbidden modes before plan work.
   Approval performs no host operation, cache access or publication.
3. On Linux x86-64, `execute_native_recipe(approval, NativeToolRootApproval)` owns the real
   session and returns `NativeRecipeSuccess` only after object audit and confirmed cleanup.
   Other hosts have no execution entrypoint. The planning and approval interface grants no
   cross-platform execution claim.

The trusted caller must obtain expected source-plan and recipe identities, complete tool-root
and image digests, exact tools and policy, output path/size/SHA-256 and allowed undefined symbols
through its operator channel. `NativeToolRootApproval` is a closed, deserializable data record;
`native_tool_root_identity` computes its canonical identity without authenticating its origin.
Package JSON, a colocated checksum, the new output or a caller's `isolated: true` claim cannot
supply independent approval. Raw request records are data; opaque plans, approvals, audited
objects and session success retain their separate owning checks.

## Fixed host mechanism

Execution requires Linux x86-64 and the trusted local Docker daemon at `/var/run/docker.sock`.
The operator's `NativeToolRootApproval.engine_uid` independently identifies the trusted daemon
owner. The fixed Unix socket must have that protected owner, and each connection must report the
same UID through peer credentials. A package or daemon response cannot supply this expectation.
There is no ambient endpoint selection, image tag resolution, network acquisition or host tool
discovery. The operator approves the entire bounded root inventory and the exact GCC, cc1,
assembler, loader and trusted supervisor executable bytes. Actual root archive members, modes,
ownership, aliases and complete input/output inventories must match those expectations.

The fixed layout compiles retained `library.c` with exactly the declared `library.h` and
`sqlite3.h` read-only inputs, producing the sole audited `adapter.o` at logical
`native/adapter.o`. It accepts no arbitrary shell command or user-selected host path. The image
root is read-only; the fixed seccomp policy denies operations by default, drops all Linux
capabilities, requires no-new-privileges and denies networking and namespace escape.

Admission counts root and compile inputs together: at most 64 nonempty regular material files,
128 MiB total bytes and 32 MiB of read-only data, including the retained C source and headers.
Each executable is at most 64 MiB. The complete fixed-layout input archive must also fit its
separate 2 MiB bound; satisfying the larger aggregate ceilings does not waive that limit.

The approved image root permits only two exact symbolic aliases, each with mode `0777`:
`/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2` to `/lib64/ld-linux-x86-64.so.2`, and
`/etc/mtab` to `/proc/mounts`. Other aliases or changed targets reject. This allowance grants
no symbolic-link authority to source, compile-input or output archives.

Both roles mask `/etc/hostname`, `/etc/hosts` and `/etc/resolv.conf` at runtime. The production
supervisor checks root-owned character-device endpoints matching `/dev/null` and reads EOF
from each. This hides daemon-generated hostname, hosts and DNS text; opening those paths is
allowed. The separately checked pre-start root inventory permits only the declared empty daemon
placeholders. Separate local trusted controls passed executable-mapping and execution denials
through original descriptors, `/proc/self/fd` and `/proc/self/root` aliases, with an approved GCC
mapping as a positive control. Identical library copies failed to map at the tested input,
temporary and output paths while the approved root library preload succeeded. These specific
mapping failures expose no loader errno and establish no general loader or package authority.
The scratch hostile archive auditor refused the extra ELF output and extra control inputs;
that exact archive was not passed to the production Rust parser. Production-parser inventory
rules have separate unit coverage, and the production API separately refused the wrong output
expectation in its five-session run. The hostile controls compiled the genuine adapter, waited
for recipe termination before the holder's read-only archive read, and confirmed owned cleanup.

Two separately owned roles use distinct write authority. The trusted holder prepares inputs and
sees output read-only. The recipe sees inputs read-only and its bounded output writable. Before
output is read through the holder, the driver kills and waits for the recipe's private PID
namespace, checks the original init identity and confirms termination. The complete output
archive must then match independent metadata, size and digest expectations and pass the existing
foreign-object audit. No fresh recipe execution is admitted after this transition.

The single 30-second job deadline covers host operations and audit. One additional 5-second
deadline bounds cleanup of owned containers and volumes. Unknown or failed cleanup cannot
construct success, even after an otherwise valid object. `NativeRecipeSuccess` retains the
original approval, approved root, audited object, actual session/container identities and elapsed
time. It has no raw constructor, deserializer or native cache importer.

| Resource | Fixed bound |
| --- | --- |
| Holder / recipe memory | 16 MiB / 96 MiB; aggregate 112 MiB, no additional swap allowance |
| Holder / recipe process count | 2 / 16; aggregate 18 |
| Holder / recipe CPU quota | 0.1 / 0.5 CPU; aggregate 0.6 CPU |
| Shared input / output / control tmpfs | 2 MiB / 8 MiB / 4 KiB |
| Per-role `/tmp` / `/dev` / `/dev/shm` tmpfs | 16 MiB / 1 MiB / 1 MiB |
| Job / owned cleanup deadline | 30 seconds / 5 seconds |

`native_recipe_environment()` returns the complete eight-variable environment. Nothing is
forwarded from the caller: `PATH=/tool/bin:/tool/libexec`, `LC_ALL=C`, `LANG=C`,
`LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/lib64`, `TMPDIR=/tmp`, `HOME=/nonexistent`, `TZ=UTC`,
and `SOURCE_DATE_EPOCH=0`. `native_recipe_policy_identity()` identifies this requested fixed
policy; its hash does not prove that a host enforced it.

## Refusal and verification

`NativeRecipeError` preserves earlier source-plan and object-audit diagnostics. Recipe-specific
reason identifiers include `TRUST-OPERATION`, `TRUST-NATIVE`, `TRUST-TOOL`, `TRUST-SOURCE`,
`TRUST-PLAN`, `TRUST-INPUT`, `TRUST-ENFORCEMENT`, `TRUST-OUTPUT`, `TRUST-TIMEOUT` and
`TRUST-CLEANUP`. They are library refusal categories, not new public compiler diagnostic codes.
Missing or substituted approval, extra material/output, unsupported host enforcement, compiler
failure, unconfirmed namespace teardown or resource removal rejects without session success.

Unconfirmed cleanup returns `NativeRecipeError::Cleanup { primary, owner, detail }` with reason
`TRUST-CLEANUP`. `primary` retains the original operation error when one occurred; `owner` is the
fresh label for this attempt's resources, and `detail` is a fixed cleanup refusal description.
The trusted operator can use that identity for recovery. It is recovery data, not a cleanup
certificate or permission to reuse or publish the attempted output; no captured object or
`NativeRecipeSuccess` is returned through this error.

The [explicit production-API conformance example](../crates/zryna-driver/examples/native_recipe_conformance/README.md)
documents independently reviewed tool-root and header inputs and fresh invocations through
the ordinary driver. The driver component explicitly registers its example directory; this
architecture registration adds no CLI or profile authority. Local Linux x86-64 production-API
qualification passed all five sessions:
two fresh successful compilations of the independently expected 1,232-byte object, an exact
wrong-output rejection, a compiler-error rejection, and a separately approved nonterminating
compiler rejected by the actual 30-second deadline. The run confirmed cleanup and unchanged
project state, with its source inventory and executable unchanged throughout. Compiling the
example or executing a separate C fixture does not establish this result.

This is local exact-source session evidence. Issue acceptance additionally requires independent
review and full policy/foundation and supported-host CI for the exact candidate; local session
evidence alone does not certify those gates. It adds no new supported host, public profile,
signed release or production admission.

Reproduce all five sessions with separately reviewed normal and fault root approval data:

```sh
cargo run --locked -p zryna-driver --example native_recipe_conformance -- /absolute/normal-root.json /absolute/sqlite3.h GCC_VERSION /absolute/timeout-root.json
```

Omitting the fourth argument omits the timeout proof. Reconstructing the trusted supervisor
requires both `supervisor.c` and `supervisor_alias.h` in one fresh directory, an independently
reviewed compiler and the recorded minimal environment/flags. Preserve `-nostdinc -ffreestanding -fno-builtin -nostdlib -static`, strict warnings and an NX
stack, and audit actual opened sources
and the resulting ELF. These trusted build steps confer no package execution authority. The
checked-in example reproduces the production API sessions; no standalone hostile-control runner
is supplied by that example.

Issue acceptance requires retained exact-revision evidence for all three criteria and hostile
filesystem, environment, network, process, timeout, output, cache, substitution and cleanup
cases, together with required full policy and supported-host gates. Pure-source/playground
denial, one successful compile or approval equality cannot substitute for that complete matrix.
The earlier [JavaScript prerequisite checks](../scripts/native-recipe/README.md) remain
deliberately non-executing and cannot mint this Rust session's authority.
