# SQLite allocator-backed byte fixture

This fixture defines only `fixture-sqlite-c-v0@0/sqlite_fixture_copy_bytes` and
`fixture-sqlite-c-v0@0/sqlite_fixture_release_bytes`, with byte kind
`fixture-sqlite-c-v0@0/owned_bytes`. The adapter includes its own `library.h` and
an independently supplied upstream `sqlite3.h`. Its positive-length allocation
calls `sqlite3_malloc(int)` and release calls `sqlite3_free(void *)` on the exact
returned allocation. It does not substitute libc allocation or query a database.

The `copied` source function follows the reviewed native-C byte wrapper: borrow
input through the call, check status before taking output, copy validated bytes
into private Zryna storage, then release the foreign allocation once. Byte values
are packed `Vec<i32>` input under the existing wrapper contract, not the private
vector layout. Both the wrapper and policy retain the 4096-byte maximum.

For valid callers, `(NULL, 0)` succeeds with `(NULL, 0)` and no allocation. A
positive admitted length succeeds with a fresh SQLite allocation and exact count.
Raw length above 4096 and allocation failure return 1 without output writes.
After the raw over-limit check, null input with positive length or null output
slots return defensive 2 before allocation or writes; 2 is outside the declared
valid-input status domain.
Output slots must be live, aligned, separate, and disjoint from stable input.
Non-null outputs are actual matching live SQLite allocations, including the
reviewed conditional release guarantee for malformed metadata. Arbitrary forged
pointers are not made safely releasable by this in-process adapter contract.

The policy is the existing reviewed copy/release schema with a separate library,
allocator, kind and release identity; it imports no other fixture operations.
`declarations.ffi.json` binds the exact local header/policy/source hashes and
actual UTF-8 byte spans of the copied function's Ffi sites. The header checks the
LP64 carriers, and the adapter checks upstream allocator function types at C
compile time. The driver tests in `foreign_library/tests/sqlite.rs` verify the
fixed materials through syntax, declarations, body checking, IR, MIR and object
emission, including independent issuer and hostile material substitutions.

`conformance.c` is a separate direct-adapter oracle. It checks empty, ordinary,
maximum and first-extra lengths, unchanged inputs and failure outputs, injected
allocation failure, fresh recovery and exactly-once same-library release. Its
allocation wrapper delegates successful allocations to real SQLite. Injected
failure is distinct from an observed real SQLite out-of-memory condition. This
client does not execute the generated Zryna entry or prove private-copy cleanup.

Run the source-bound Rust tests from the repository root:

```sh
cargo test --locked -p zryna-driver --lib sqlite_ -- --nocapture
```

For the direct C oracle, supply independently acquired SQLite 3.54.0 source and
header in `SQLITE_AMALGAMATION`. Their SHA-256 values are
`2da161e310ef0aba3465f22a4d12b8a43723236f0942fa76e8aac523c3317243`
(`sqlite3.c`) and
`cd20a2877bc82b94e36c1093b727dd87cfa3c1d581aac6e240925535cbd90166`
(`sqlite3.h`). Verify these before compiling. From the repository root:

```sh
sqlite_test_dir=$(mktemp -d)
cc -std=c11 -O0 -g0 -DSQLITE_THREADSAFE=0 -DSQLITE_OMIT_LOAD_EXTENSION \
  -c "$SQLITE_AMALGAMATION/sqlite3.c" -o "$sqlite_test_dir/sqlite.o"
cc -std=c11 -O0 -g0 -Wall -Wextra -Werror -pedantic \
  -I tests/native-c-sqlite-v0 -I "$SQLITE_AMALGAMATION" \
  tests/native-c-sqlite-v0/library.c tests/native-c-sqlite-v0/conformance.c \
  "$sqlite_test_dir/sqlite.o" -Wl,--wrap=sqlite3_malloc -Wl,--wrap=sqlite3_free \
  -lm -o "$sqlite_test_dir/conformance"
"$sqlite_test_dir/conformance"
```

For ASan/UBSan, compile both the provider and the final executable with
`-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer`.
An independent missing-release mutant must fail the live-allocation assertion;
restore the original adapter and rerun the oracle to check fresh recovery.

Upstream amalgamation/header acquisition and checksum provenance are separate
inputs. No upstream source archive or amalgamation is vendored here. These files
alone establish no artifact provenance, accepted compiler/link/runtime policy,
public source grant, SQLite execution result, sanitizer result or #417/#405
closure. Owning source-bound observations, generated-wrapper execution and the
production linking/host prerequisites remain separate acceptance gates.
