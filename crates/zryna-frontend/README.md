# Zryna frontend contract

Versioned, provider-neutral boundary for replaceable TypeScript and native Zryna frontends.

Provider output is untrusted. Adapters return `RawProjectSyntaxSnapshot`; the core decodes it under
fixed byte and item budgets, verifies the exact canonical file-id/path set against `SourceMap`, and
converts every raw UTF-8 range into an opaque, map-bound `Span`. Only the resulting
`ProjectSyntaxSnapshot` may enter later compiler phases.

Protocol v1 intentionally carries declarations and diagnostics only. Protocol v2 will add executable
syntax without weakening this fail-closed boundary or changing v1 semantics in place.
