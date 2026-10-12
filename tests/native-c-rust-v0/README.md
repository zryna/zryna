# Rust library C-ABI source fixture

This #417 slice supplies independent Rust-library materials and Linux x86-64
source-issuer tests. The workspace tests authenticate source documents and emit
Zryna objects; they do not compile or execute the foreign Rust program. It adds
no accepted Rust ELF profile, execution recipe, public guest capability or
production native grant.

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

Next, after source validation, retain the exact pinned Rust 1.97.1 ordinary
compile invocation and inspect the genuine no_std object sections, symbols,
dependencies and relocations. Unknown ELF output must not be rewritten or
accepted by loosening the GCC audit. A Rust-specific object capture/profile
and all link/runtime consumers are still unimplemented. The GCC-origin
CompiledObject and qualified material inventory must not be borrowed.
The actual 157,802,496-byte rustc_driver exceeds unchanged 64 MiB tool and
128 MiB total retained-material limits. Complete Rust tool inventory and
isolated recipe qualification remain absent; no file is omitted or relabelled.

Actual wrapper controls still require implementation: independent C entry-count
wrapping, 4096 versus 4097 pre-entry rejection, bad element/count and private
allocation cleanup, undeclared-status and write-on-error Rust mutants, original
issuer substitution, retained foreign-object snapshots, failure cleanup and
recovery. This source fixture does not close #417 or #405.
