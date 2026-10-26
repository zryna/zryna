# Private Struct-v4 same-seal execution test

This Linux x86-64 driver integration test authenticates actual native lexer/parser
DTOs and TypeScript-v5 packets against exact source bytes, then gives one retained
v4 IR seal to JavaScript, core Wasm and native emitters. It does not activate a
public driver/profile or establish full #416 completion.

The original 159-byte construction and both 245-byte Pair<String> sources and their
oracles remain unchanged under `tests/m7-generic-struct-v4`. Each execution source
has that exact original prefix followed by a newline, the scalar export
`export function entry(): i32 { return score(); }`, and a trailing newline.
The explicit wrapper is separately source authenticated. Its new untrusted
TypeScript packets were emitted with pinned Node 22.22.1 and the existing
TypeScript-v5 producer; fixture creation alone proves neither native parity nor
executable behavior. `fixtures/manifest.json` records the source and packet digests.

`oracle.rs` and `execution-oracle.json` fixed expected results and allocation-role
traces before target compilation. Every target must return 7 on success.
The fault sources allocate earlier-root, left, then a distinct right clone, in
declaration order despite reordered initializer spelling. Success releases roles
3, 2, 1; controlled allocation 1/2/3 failures release only the successful prefix in
reverse. Allocation roles identify actual owners, including equal-content clones.

JS and Wasm check every actual allocation/release identity exactly once and retry
success after each fault. Native uses independent malloc/free callbacks and a
separately spawned executable per observation. Rust requires SIGILL on each
controlled fault and compares the exact stdout trace emitted before that trap.
The C executable does not fork. Every child is created through `zryna_process`'s
shared spawn gate and a process group; output is limited to 64 KiB per stream.
Runtime children have 10 s deadlines and the native C linker has 30 s, matching the
existing dedicated cross-target harness budgets. No process-gate or timeout
exception is introduced.

Run the auto integration target with the repository-pinned tools:

```
cargo test --locked --frozen -p zryna-driver --test generic_struct_v4 -- --nocapture
```

The test emits separate temporary evidence directories for each source. They retain
the v4 wire, JS/Wasm/native artifacts, unchanged issuer-generated runtime header,
same-seal receipt, portable observations, native signal/status observations and
native cleanup stdout. Test authorship and syntax/format checks are not runtime
results; the root agent owns Cargo execution and qualification receipts.
