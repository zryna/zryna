# zryna-abi

`zryna-abi` owns the versioned scalar boundary shared by every Zryna target. It verifies logical
exports and scalar signatures before a backend sees them, produces typed target-name views, and
defines typed host invocation and observation values.

Scalar ABI v1 specifies `i32` and `bool`, but specifying a representation does not enable a type in
Universal IR or any backend. Each compiler profile must independently prove complete support before
it admits that type.

The normative contract and shared conformance cases live under [`spec/abi`](../../spec/abi).
