# Native runtime

This directory will contain implementations behind versioned Zryna native runtime ABIs. Compiler
crates may depend on ABI contracts, but runtime implementations must not depend on compiler
phases. The current pure `i32` object has no runtime dependency or undefined symbol. The driver can
link it with a private generated one-invocation C harness under the
[executable contract](../../spec/native-semantics/EXECUTABLE.md), but that harness is not a Zryna
runtime and this directory remains empty of runtime implementation. The resulting executable is
dynamically linked and requires the compatible system CRT, libc, and dynamic loader selected by
the validated GNU toolchain; it is not a static or cross-distribution portable artifact.

`zryna build --target native` publishes the audited relocatable `.o`, while
`zryna run --target native` composes the one-invocation harness and publishes the audited `.elf`
at mode `0755` inside its atomic run bundle. Native run is supported only on Linux x86-64; the CLI
is not an arbitrary native program runner.
