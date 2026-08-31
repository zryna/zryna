# zryna-layout

Compiler-owned verification authority for canonical M3 type identities, aggregate layouts, and
sealed layout fingerprints. Raw type graphs are untrusted. Only this crate can construct the
opaque immutable views consumed by later compiler and runtime layers.

This crate does not lower syntax, allocate memory, expose aggregate ABI, emit target code, or
activate the public `DataOwnershipV1` profile.
