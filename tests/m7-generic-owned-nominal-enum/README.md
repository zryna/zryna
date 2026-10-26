# Private nominal enum execution proof

This packet exercises the same source-authenticated owned program on JavaScript,
core WebAssembly, and Linux x86-64 native objects. Both the single-module and
cross-module forms retain the same fixed observations and allocation/cleanup
oracles. Each target runs 43 observations, including controlled failures and
successful retries; the native harness additionally checks 183 literal event lines.

The covered private domain includes original enum identity, ordered generic
arguments, every substituted variant payload, nested standard Option/Result
payloads, shared/exclusive borrows, and owned cleanup. Phantom String arguments
remain Copy when no stored member owns a String; fixed String members remain
owned even for the empty active variant. Both target layouts, typed ownership,
and the final native relocation inventory are independently checked.

`run.py OUTPUT_DIRECTORY` requires the repository's pinned Rust and Node versions,
`CARGO_TARGET_DIR`, and a supported Linux GCC. It builds with the frozen lockfile
and compares results with the checked-in oracles. Expected results must never be
regenerated from compiler output.

This is private qualification for issue #416. Struct/container execution, nested
nominal arguments, structural Clone and additional ownership transport still
need separate verification. Public profiles, CLI admission and full M7 acceptance
remain unchanged.
