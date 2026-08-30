# Native runtime

This directory will contain implementations behind versioned Zryna native runtime ABIs. Compiler
crates may depend on ABI contracts, but runtime implementations must not depend on compiler
phases. The current pure `i32` object has no runtime dependency, undefined symbol, startup object,
linker, executable, or product execution harness.
