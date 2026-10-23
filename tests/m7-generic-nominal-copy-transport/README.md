This private source-first packet qualifies concrete nominal enum Copy transport through one
owned program seal and the existing JavaScript, core WebAssembly and Linux x86-64 emitters.
The expected results and allocation traces were fixed before nominal transport production.
These files grant no public profile, driver, ABI or runtime capability.

`Box<Option<Result<bool,i32>>>` crosses branch/loop machinery as one exact nominal type.
Two such values exchange positions across zero, one and two iterations using an explicit
saved source value. These observations check that source behavior; independent IR edge-swap
controls cover parallel edge transfer. `Marker<String>` stores only
a bool; its unused String argument must not make the value affine. Separate semantic controls
cover direct bool/i32 payloads, a nongeneric enum, inactive String-bearing variants, opaque T
payloads and shared/exclusive loans of mutable destinations.

Literal arithmetic gives `zero(0)=11+7+5=23`, `one(0)=7+31+3=41`, and
`two(0)=11+7+3=21`. Nominal Copy operations allocate no owners. An incoming owned argument,
a retained shared guard loan and per-iteration `step` strings retain independent cleanup
obligations. `oracle.json` freezes three successes, all nine reachable allocation failures and
a pristine retry after each failure. Per module form this requires 21 JS and 21 Wasm
observations, and 12 native successes plus nine cleanup-before-SIGILL failures. The native
stdout oracle fixes all 126 lines. The literal values and events intentionally match the older
standard-family Copy packet; the nominal declarations and source are separate new fixtures.

Both joined and imported source forms authenticate independently. Generate their complete
reference DTOs without invoking ownership discovery:

```sh
ZRYNA_GENERIC_NOMINAL_COPY_FIXTURE_DIR="$PWD/tests/m7-generic-nominal-copy-transport" cargo test --locked -p zryna-semantics --lib nominal_copy_target_packet_authenticates_both_independent_dtos_before_transport -- --nocapture
```

The source-first semantic admission tests are selected with `nominal_copy_transport`.
Before implementation, the positive source admission tests must fail at the existing nominal
Copy gate. Original body typing and full source authentication must succeed first.

With the existing bounded toolchain environment:

```sh
python3 tests/m7-generic-nominal-copy-transport/run.py /tmp/m416-nominal-copy-target-evidence
```

The emitter independently authenticates the frozen fixture DTO and the actual native
lexer/parser candidate against the same immutable source, then requires complete raw-file
facts to agree. Semantics, complete instantiation discovery, both layouts, ownership/runtime
ABI and IR sealing precede all three backends. The runner preserves existing deadlines,
reproducibility checks, unchanged runtime header and actual allocation identities. Its presence
is a test obligation until the exact source revision passes; target execution is not claimed.
