# Replaceable frontend providers

## Why TypeScript 6 is temporary

The bootstrap adapter uses the public TypeScript 6 API to avoid reimplementing parsing, source locations, basic declarations, and module semantics before the first backend slice exists. It is selected for available API access, not runtime speed.

The adapter is pinned exactly and isolated behind protocol version 1. It may return only ZRYNA-owned normalized data. Zryna semantics remain authoritative.

## TypeScript 7 readiness

TypeScript 7.0 does not ship a supported programmatic compiler API. Its CLI can run as a shadow compatibility check, but it is not a frontend provider. A TypeScript 7 adapter will be added only after an upstream API exposes the capabilities required by the provider conformance suite.

Provider selection is capability-based:

```text
handshake
├── provider identity and exact version
├── Zryna protocol version
├── module-resolution capability
└── semantic-diagnostics capability
```

A provider version mismatch, crash, malformed message, incomplete snapshot, or unsupported syntax fails closed.

## Source-coordinate contract

Protocol v1 `span.start` and `span.end` are zero-based, half-open UTF-8 byte offsets. The
TypeScript adapter therefore converts its native ECMAScript UTF-16 code-unit positions before
emitting a snapshot. It rejects ill-formed strings and never rounds an offset that splits a
surrogate pair. Non-ASCII adapter fixtures are required on Linux and Windows.

The compiler constructs the authoritative bounded `SourceMap`. Protocol-v1 responses deserialize
into `RawProjectSyntaxSnapshot`, whose IDs, paths, collection bounds, diagnostics, and
`UntrustedSpan` values have no compiler authority. `verify_snapshot` requires the exact complete
file set, canonical identifier/path pairs, bounded declarations and text, and source-map-valid
UTF-8 ranges before constructing `ProjectSyntaxSnapshot`. Provider transport bytes are bounded
before JSON decoding. Protocol v2 extends this verified boundary with executable syntax; it must
not silently change the meaning of the v1 `start` and `end` fields.

## Migration to a native Zryna frontend

The permanent path is:

```text
native Zryna lexer → parser → resolver → ProjectSyntaxSnapshot v1
```

Because all later phases depend on the ZRYNA-owned snapshot and IR, replacing the bootstrap provider must not modify either backend.
