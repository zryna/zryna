# Native runtime

This directory contains implementations behind versioned Zryna native runtime ABIs. Compiler
crates may depend on ABI contracts, but runtime implementations do not depend on compiler phases.

`ownership_runtime_v1.c` implements the single-threaded Linux x86-64
`zryna-ownership-runtime-v1` boundary and exports exactly the 17 functions in the checked ABI
header. The candidate driver renders only the authenticated Vec element-layout cases needed by
one sealed program, compiles the result with the validated toolchain, and keeps the runtime object
private to that build. The allocator bounds every request, validates opaque pointers through its
private allocation registry, and leaves caller outputs zeroed on failure. String, Vec, and
Shared/Weak transitions are status-returning and preserve their inputs unless their documented
success transition commits. Threads, custom allocators, raw host pointers, and a public C API are
outside this runtime.

The implementation retains a 64 MiB per-allocation target budget. Exhausting that budget with a
request at or below the universal 2,147,483,647-byte allocation/String limit reports `ALLOCATION`;
only checked arithmetic or a genuine universal/profile maximum violation reports `CAPACITY`.

The existing pure `i32` object has no runtime dependency or undefined symbol. The driver can link
it with a private generated one-invocation C harness under the
[executable contract](../../spec/native-semantics/EXECUTABLE.md), but that harness is not a Zryna
runtime. The resulting executable is dynamically linked and requires the compatible system CRT,
libc, and dynamic loader selected by the validated GNU toolchain; it is not a static or
cross-distribution portable artifact.

`zryna build --target native` publishes the existing audited relocatable `.o`, while
`zryna run --target native` composes the one-invocation harness and publishes the audited `.elf`
at mode `0755` inside its atomic run bundle. Native run is supported only on Linux x86-64; the CLI
is not an arbitrary native program runner. A DataOwnershipV1 candidate route remains internal
until its public selector and release gate are completed.
