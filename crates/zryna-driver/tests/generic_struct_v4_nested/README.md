# Nested driver harness proposal

The `crates/` subtree is an additive overlay for the corresponding path under
`/tmp/m416-nominal-transport-candidate`. It introduces the auto-discovered integration
target `generic_struct_v4_nested` and its private helper directory. It does not alter
the existing `generic_struct_v4` harness, fixtures, Cargo manifest or production code.

`INPUT_FREEZE.json` hashes all inspected existing driver helpers, all 56 nested data
files, and both proposal documents before the first harness write. The harness's
`fixture-identities.json` pins the complete nested data tree. Every run verifies these
bytes, including the original independent source/expected freeze and packet identities.
`execution-oracle.json` copies all eight independent admitted runtime expectations;
Rust verifies each copy equals its original `*-expected.json` before running a target.
Only independently fixed counts 0,2,1,1,0,0,1,4 were added to describe those cases.

All thirteen exact `main.zry` literals go through the actual native lexer and v5
candidate parser. The resulting complete authenticated file facts are compared with
both authenticated frozen native and TypeScript packets bound to that same SourceMap.
The source retains its existing explicit `entry`; the harness appends no wrapper.

Each admitted original undergoes original ownership checking, contextual discovery,
both sealed layouts, source-produced v4 claiming, a wire roundtrip and independent v4
issuance. The callback receives that one retained opaque program and exact wire once.
JS, core Wasm and native emitters consume that same program without new claims. The
native object undergoes independent inventory validation against the lowered sealed
MIR; an inventory JSON retains ELF sections, relocations, symbols, MIR function names
and the audited object hash. The original source, both provider packets, issuer runtime
header, wire and all artifacts and hashes remain in a unique evidence directory.

For every target, eight initial successes plus nine controlled faults and nine pristine
retries produce 26 observations. The test requires 78 total and nine separately spawned
native SIGILL outcomes. Runtime checks use actual handle/pointer identities and exact
allocation/release events, including distinct E allocation 1 and E clone allocation 4.
The prefix trace is E,L,R,C with success C,R,L,E; fail3 drops L,E and fail4 drops R,L,E.
A phantom has no runtime allocation; Fixed's member has one; both nested Option/Box cases
release only the active allocation. Every failure must trap and return no scalar.

Portable receipts preserve every observation and the actual allocated/released identity
lists. The native C runtime retains bounded records, accepts the frozen case allocation
count up to four, and checks exact once-only releases. Each native observation runs in
its own process; exact stdout must precede SIGILL. Unique initial/fault/retry stdout
files retain every run without overwrite. Native identity JSONL files retain actual
pointers and lengths; Rust compares allocation and release addresses for every
fixed role, independently of equal-content string bytes. Existing shared spawn-gate, process-group,
64 KiB streams and 10/30 second runtime/linker budgets are copied unchanged.

Apply only the `crates/` overlay, then root may run from the candidate repository:

```
cargo test --locked --frozen -p zryna-driver --test generic_struct_v4_nested -- --nocapture
```

This proposal was authored without Cargo, provider, target-runtime or C execution.
Formatting and static consistency checks are not execution results. Genuine RED/GREEN
runs, overlay application and qualification remain root-owned.
