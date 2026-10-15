# Rust library C-ABI source fixture

This #417 fixture supplies independent Rust-library source data and Linux x86-64
source-issuer tests, a distinct bounded data-only Rust ELF capture profile, and
nine ordinary test-owned execution observers. The source-issuer tests authenticate
documents and emit Zryna objects without compiling the foreign program. The
ordinary observers instead require a fresh trusted Rust compile and actual
separate C client/runtime/link execution. These remain distinct evidence and
supply no qualified execution recipe, public guest capability or production grant.

library.rs is a safe data module containing FOREIGN_RUST_SOURCE. Its exact
1696-byte raw-string payload retains the independently authored foreign program
and SHA256 26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b.
The workspace compiles only the safe constant and source tests; it does not
activate the foreign unsafe program as repository implementation. Later ordinary
measurement must explicitly stage the constant bytes in a private create-only
external directory and compile that retained source, never this data wrapper.

The library identity is fixture-rust-c-v0@0. The two foreign functions use only C
int32, byte-pointer, size_t and out-pointer carriers. Rust String/Vec, callbacks,
allocation, retention and unwinding are excluded. add wraps modulo 2^32.
sum_bytes reads at most 4096 borrowed bytes and commits its out slot only on 0;
1 is raw over-limit with unchanged caller storage. Defensive 2 rejects invalid
null callers outside the admitted domain; it is never a recoverable status.
These extern C declarations do not admit the Rust native ABI.

The independently authored header and operation policy are digest-bound in the
canonical declaration document. Source bindings and primitive spans refer to
these exact two Zryna source files, not the old seven-operation C fixture.
The capture helper calls genuine syntax, declaration, body, private-boundary,
IR and MIR issuers, then the scalar-import, byte-import and reverse-export
backend emitters. Both add and sum_bytes remain original foreign definitions
in the complete source authority even when only the scalar entry is selected.

Two registered source tests cover these boundaries. The positive test checks
the exact foreign-source data digest and that actual Zryna backend relocatables
retain the original Rust-specific source
issuer and exact selected imports. The independent equal-document recapture
must not replace that source issuer. These are source/compiler-boundary tests,
not a Rust object compile, Rust-library execution or the normative Rust C-shim
gate.

raw-client.c is the independent raw oracle for wrapping, null-zero, 4096/4097,
input preservation, failure sentinel and invalid-null status controls.
reverse-client.c must be separately compiled against the actual emitted
reverse-exports.h; no hand-authored generated header is supplied. The reverse
client exercises a pure Zryna export, not a Rust-library import.

The implemented Rust-specific data profile retains the genuine no_std object
sections, symbols and source expectations separately from the unchanged GCC
capture. Ordinary observers record the fresh pinned Rust1.97.1 invocation,
original producer snapshots, actual final function bytes and real shim-call
targets before execution. This does not authenticate a complete compiler input
closure or qualified recipe. Unknown ELF must not be rewritten or accepted by
loosening either audit. Rust data is never borrowed as a GCC-origin CompiledObject
or a qualified material inventory.
The actual 157,802,496-byte rustc_driver exceeds unchanged 64 MiB tool and
128 MiB total retained-material limits. Complete Rust tool inventory and
isolated recipe qualification remain absent; no file is omitted or relabelled.

The six original ordinary observers implement independent C entry counting and its live
missing-increment sensitivity, 4096/4097 Vec<i32> element packing and pre-entry
rejection, bad elements, selected private allocation cleanup, original-issuer
and producer-record substitution refusal, retained foreign snapshots and pristine
recovery. Counts prove their selected calls/allocations, not global leak absence
or pointer non-retention. The standalone raw-client4097 status1/sentinel4242
case is distinct from the wrapper4097 no-entry test.

The three separate result-fault observers freshly compile Rust-source status7
and status1/output-write controls. Independent C assertions require HostAbiFailure,
no exposed output, selected cleanup and poisoned-context refusal. A separately
compiled nonfaulting control must fail the same live oracle, followed by fresh
pristine recovery. These observers passed the Linux compiler-contract and M0
execution gates on the reviewed source integrated by PR #555. Modified control
sources have separate measurements and cannot inherit the pristine1696-byte
source/function certificate.

C-client/private-runtime sanitizer diagnostics and the full normative
resource/fault matrix remain separate. The independent SQLite allocator proof is
documented in `tests/native-c-sqlite-v0/README.md`; its results do not qualify this
Rust fixture or grant production recipe admission.
This fixture does not close #417 or #405.
