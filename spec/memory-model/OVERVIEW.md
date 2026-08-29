# Native memory model direction

The first executable slice contains only scalar values and requires neither heap allocation nor garbage collection.

The planned no-GC profile uses:

1. stack and value semantics by default;
2. unique ownership for heap values;
3. deterministic drop insertion;
4. compiler-checked borrowing;
5. explicit shared reference counting;
6. explicit weak references for cycles;
7. a versioned runtime ABI that never exposes Rust standard-library layouts.

No-GC does not mean no heap. It means that native lifetime management does not require a tracing collector.

JavaScript output runs inside a JavaScript engine and therefore uses that engine's memory management. Zryna source-level move and ownership rules may still be enforced to keep cross-target behavior predictable.

String encoding and indexing semantics must be specified before an owned string runtime is implemented.
