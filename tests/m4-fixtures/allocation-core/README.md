# Private allocation core conformance

Issue #377 develops the S2 fixture slice of
`spec/libraries/MINIMAL_CORE_HOST_V0.md`. These are private source fixtures for
A1–A5 under DataOwnershipV1. They add no library package, owned entry ABI, host
operation, public selector, or public support claim. Only `score` is an entry;
String and compiler-known `Vec<i32>` values remain inside source functions.
String scalar continuations use the existing relative dependency-export shape
from the M3 String corpus. Those dependency exports are private to the source
module graph; they do not become public entry exports.

`cases.json` records independent typed results and cleanup places, rather than
using one backend as another backend's oracle. Q4/Q5 pin literal UTF-8 bytes;
Q6 observes the copied and mutated vectors in separate invocations; Q7 covers
the last valid, negative, first-extra, and empty indices; Q8 covers an append
across the initial two-element capacity and deterministic failed growth.
The three invalid-index cases also arm the next allocation failure: the bounds
trap must win before the later String initializer is evaluated.

The first-allocation and operation-failure cases use the existing bounded
private fault command. A distant allocation ordinal enables normal cleanup
tracing without changing these small successful executions. Failed String and
Vec clone/concat rows inspect the retained input payload and absence of a
completed result, then run the matching fixed success twice in the same imported
JavaScript module or WebAssembly instance. Replacement cases observe both
preparation failure and successful commit. Prefix cases fail the second or third
argument preparation before entering `consume`: only completed String results
are released, in reverse order, followed by the retained source. This exercises
A1/A2 result preparation without introducing `Vec<String>` or aggregate APIs.
Copy `i32` elements have no fallible owned-member clone or drop.

The driver tests run each selected target independently through authenticated
source, semantics, IR, target emission, typed execution, and the existing
cleanup channel. N7/N10 negatives require the fixed source diagnostic and a
source span before any target dispatch or artifact publication. Unsupported
native execution on Windows requires `ZRYNA-N4002` with an empty output root.

`inspect.mjs` supplies additional private JavaScript and core WebAssembly
observations through the driver's existing bounded Node capability. JavaScript
appends inspection to a test-only copy of emitted source and checks exact
released values and distinct owners. WebAssembly first audits the original
module's imports/exports, then prepends one argument-recording call to the
selected drop helper and adds a memory export only in an in-memory test copy.
The recorded cleanup handles must be distinct and must own the exact expected
bytes; this rejects an orphan copy combined with reuse of the source handle.
Bounded post-invocation snapshots separately retain allocation-history evidence,
while the independent unmodified driver execution proves logical cleanup.
Retained arena bytes are not proof of live ownership or physical deallocation.
The inspected copies are never published as compiler artifacts.

On Linux x86-64, the same source/fault suite runs native code. A separate
`native-storage.c` harness exercises the actual private C runtime, inspecting
UTF-8 bytes, disjoint allocations, zero failed outputs, retained old Vec storage,
successful retry after String clone/concat failure, successful Vec growth, and
an empty allocation registry after cleanup. Its compiler and executable run
through the established bounded native process-group helper. Exact and
first-extra Vec/allocation limits use injected allocation failure at the exact
limit so the test never exhausts the host. This helper test complements the
source cleanup suite; it does not replace compiler execution evidence.

`capacity-cases.json` keeps resource cases separate from the small fixture loop.
Two source fixtures push up to the 1,048,576-element maximum, test the first
extra push, and read the last/first-extra element of a maximum-length clone.
They check the fixed scalar/trap result with tracing disabled: each loop
iteration has derived temporary cleanup, so a maximum-boundary trace inherently
exceeds the fixed 4,096-word observation frame. The small fixed corpus
separately proves exact owner count and reverse cleanup order without weakening
that bound.

The private allocator probes distinguish the fixed WebAssembly arena from the
universal byte maximum. `native-capacity.c` is a standalone bounded probe of
the actual C runtime; `capacity-inspect.mjs` exposes existing allocator/global
definitions only in an in-memory copy of an emitted module. Neither builds a
large payload. The previous 64 MiB first-extra `CAPACITY` oracle was incorrect;
the integrated [#384](https://github.com/zryna/zryna/issues/384) prerequisite
classifies target exhaustion within the universal limit as `ALLOCATION`.
`native-capacity.c` remains owned by the #384 runtime regression rather than a
duplicate #377 driver test. These prepared fixtures are not passing evidence
for this revision and do not establish String maximum-length source execution.
See [capacity evidence and reproduction](CAPACITY.md) before claiming completion.

Focused commands:

```sh
node --test tests/m4-allocation-core.test.mjs
cargo test --locked -p zryna-driver --lib allocation_core -- --test-threads=1
```

Use the repository-pinned Node and Rust toolchains. The Node test validates
the fixture inventory and fixed oracles; it is not target execution evidence.
Linux native execution and the complete existing String/Vec exact-limit gates
remain required before claiming conformance on all targets. Full preflight,
M0, and hosted Linux/Windows verification remain separate merge requirements.
