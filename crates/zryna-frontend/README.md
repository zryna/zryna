# Zryna frontend contract

Versioned, provider-neutral boundary for replaceable TypeScript and native Zryna frontends.

Provider output is untrusted. Protocol-v1 adapters return their declaration snapshot under the
legacy contract; protocol-v2 providers return serialized bytes so the core applies the transport
limit before decoding. The core then verifies fixed item budgets and the exact canonical
file-id/path set against `SourceMap`, and converts every raw UTF-8 range into an opaque, map-bound
`Span`. Only the resulting verified project may enter later compiler phases.

Protocol v1 intentionally carries declarations and diagnostics only. Protocol v2 is a separate
executable-syntax contract owned by `zryna-syntax`; it does not change v1 semantics in place. The
TypeScript 6 adapter remains on v1 until its dedicated conformance change is complete.
