This private source-first packet checks recursive concrete Copy branch and loop transport through
one owned seal and the existing JavaScript, core Wasm and Linux x86-64 emitters. It adds no wire
opcode, runtime operation, ABI carrier, host permission or public profile selection.

`values.zry` replaces an `Option<bool>` across a branch and transports two
`Option<Result<bool,i32>>` values simultaneously across zero, one and two loop iterations.
The two nested observations distinguish a correct simultaneous swap from sequential replacement.
An incoming owned argument, a retained shared guard loan and iteration-local String allocations
check unchanged owner cleanup alongside the Copy carriers.

The literal source-first results are `zero(0)=23`, `one(0)=41`, `two(0)=21`. `oracle.json`
fixes all nine reachable allocation failures and cleanup events before execution. Each failure
has a pristine retry. Each single/imported fixture therefore defines 12 native successes and
nine cleanup-before-SIGILL failures; JS and Wasm each define 21 observations. The native stdout
oracle fixes all 126 lines. These are requirements until the exact-revision runner succeeds.

Generate the complete authenticated reference DTOs through the semantic fixture test:

```sh
ZRYNA_GENERIC_CONCRETE_COPY_FIXTURE_DIR="$PWD/tests/m7-generic-owned-concrete-copy" cargo test --locked -p zryna-semantics --lib source_first_recursive_copy_target_packet_authenticates_both_module_forms -- --nocapture
```

With the existing bounded Rust/Node/GCC toolchain environment, execute:

```sh
python3 tests/m7-generic-owned-concrete-copy/run.py /tmp/m416-concrete-copy-target-evidence
```

The runner preserves the existing toolchain assertions and deadlines; the caller selects
Cargo build parallelism using its normal environment.
It checks deterministic artifacts, fixed scalar results, actual allocation identity/release
traces and the exact unchanged runtime header on all three targets, for both module forms.
Independent raw typed/owner witnesses live in `zryna-ir`'s `plan::concrete_copy_transport` tests;
genuine source, hostile source/edge/identity and exact/first-extra key/parameter resource tests
live in semantics' `owned_v2::concrete_copy_transport` module.

Full #416 acceptance remains open, including other container/nominal ownership, non-Copy
transport, broader operations, provider parity, diagnostics, platform qualification and explicit
driver/profile admission.
