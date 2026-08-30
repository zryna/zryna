# Linux x86-64 native executable v1

Status: implemented as a driver library boundary for one typed `I32V1` invocation. A public CLI is
not part of this contract.

## Inputs and authority

`compile_native_invocation` accepts authenticated source, an exact logical export, typed scalar
arguments, the explicit `x86_64-unknown-linux-gnu` target, a validated artifact-output capability,
an artifact stem, a previously discovered toolchain capability, and process limits. Universal IR's
embedded scalar ABI authority validates the export, arity, and argument types before native object
emission, staging, or a linker process. The driver does not reconstruct target symbols or coerce
arguments.

The current executable profile admits only verified `i32` source and returns one typed
`Returned(I32(n))` outcome. The ABI authority still accounts for native Boolean carriers, but
source-level `bool` remains rejected by `I32V1` with `ZRYNA-I1006`; this contract does not claim
Boolean execution.

## Toolchain capability

Discovery is explicit and deterministic. On Linux x86-64, the driver probes only canonical
`/usr/bin/gcc`; it does not search `PATH` or honor `CC`, linker, shell, locale, or temporary-directory
environment overrides. The capability proves:

- target identity exactly `x86_64-linux-gnu`;
- GCC major version 12 through 15;
- the compiler-reported linker is either absolute or one plain leaf name resolved beside the
  canonical compiler driver, then resolves to a canonical regular executable;
- GNU ld version 2.38 through 2.46;
- canonical compiler and linker file identities remain unchanged before linking.

An unsupported host, missing tool, unexpected target/vendor/version, malformed probe, timeout,
output overflow, or changed tool identity fails closed. GCC's installed `cc1`, assembler,
`collect2`, CRT, libc, and loader are trusted parts of this system capability; this boundary is not
a sandbox for a hostile installation. Discovery is not installation: Zryna never downloads or
silently substitutes a toolchain.

## Sealed harness and link

The driver generates one bounded C11 harness for exactly one already-validated invocation. It
declares only the ABI-sealed `zryna_v1_e_<logical>` symbol, bakes typed `int32_t` carriers as decimal
constants, calls the function once, and writes exactly four little-endian result bytes to standard
output. It has no source parser, dynamic symbol lookup, user-supplied C, arbitrary entry point, or
runtime service.

Linking uses direct argument-vector process creation, never a shell, with a cleared and fixed
environment. Inputs live in a new private `.zryna/out/.zryna-link-<stem>-<pid>-<sequence>` staging
directory. The linker command uses non-PIE output, no build ID, fatal warnings, no undefined
symbols, a non-executable stack, RELRO, and immediate binding. Known staged files are removed on
both success and failure; cleanup never recursively removes an unresolved path.

The candidate must be a bounded ELF64, little-endian, x86-64 executable with a nonzero entry point,
no writable-and-executable section, and the expected nonempty global text symbol. Only then is it
made mode `0755` and create-only published as `.zryna/out/<stem>.elf` through a hard link. An
existing file, directory, or link is never replaced.

## Execution and observation

`run_native_invocation` executes only the opaque published-artifact capability. That capability
retains the audited bytes and validated output-root authority; each run copies those bytes into a
new private executable stage. Replacing, truncating, relinking, or changing permissions on the
public `.elf` therefore cannot substitute the code that the capability executes.

The child receives no stdin and a cleared environment containing only `LANG=C`, `LC_ALL=C`,
`TZ=UTC`, `SOURCE_DATE_EPOCH=0`, `PATH=/usr/bin:/bin`, and a private `TMPDIR`. It runs in its own
process group. Limits may tighten but not exceed five seconds for probes, 30 seconds for linking,
five seconds for running, 64 KiB per tool stream, 16 KiB runner stderr, and 32 MiB executable
bytes; process timeouts have a 100 ms minimum. The stdout capture budget is five bytes so a fifth
byte is observable as an overflow sentinel, while a valid frame is exactly four bytes. Timeout,
live overflow, leader exit, or other process failure terminates and confirms disappearance of the
group under a separate bounded cleanup deadline.

Success requires exit status zero, empty standard error, and exactly four standard-output bytes.
Those bytes are decoded as a little-endian `i32` and normalized by the scalar ABI authority into a
typed outcome. Process exit status reports harness health; it is never the Zryna result.

The published executable is dynamically linked against the validated host's startup objects,
dynamic loader, and libc because the harness uses C standard I/O. It is neither static nor promised
portable across Linux distributions. It must be rebuilt under the target deployment toolchain.

Assigned stable native diagnostics are limits/host/tool discovery and identity (`N4001`–`N4005`),
process I/O/deadline/output/cleanup (`N4006`–`N4009`), harness generation (`N4010`), destination
inspection (`N4014`), staging/cleanup/link/audit/publication (`N4015`–`N4019`), and abnormal
exit/result framing (`N4021`–`N4022`). Invocation name, arity, and type mismatches retain the scalar
ABI's `B2101`–`B2103` codes, Boolean source remains `I1006`, and an existing create-only destination
remains `D2007`. Unlisted native codes in these ranges are reserved. External tool output never
becomes a stable diagnostic message.

## Non-goals

This profile is not a static executable, cross-toolchain byte-reproducibility promise, general C
ABI/FFI, arbitrary native program runner, bundled native runtime, Windows/macOS output, Boolean
source execution, public CLI, sandbox, or freestanding systems profile. Those require separate
contracts and gates.
