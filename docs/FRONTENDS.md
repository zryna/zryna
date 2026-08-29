# Replaceable frontend providers

## Why TypeScript 6 is temporary

The bootstrap adapter uses the public TypeScript 6 API to avoid reimplementing parsing, source locations, basic declarations, and module semantics before the first backend slice exists. It is selected for available API access, not runtime speed.

The adapter is pinned exactly and isolated behind protocol version 1. It may return only UTS-owned normalized data. UTS semantics remain authoritative.

## TypeScript 7 readiness

TypeScript 7.0 does not ship a supported programmatic compiler API. Its CLI can run as a shadow compatibility check, but it is not a frontend provider. A TypeScript 7 adapter will be added only after an upstream API exposes the capabilities required by the provider conformance suite.

Provider selection is capability-based:

```text
handshake
├── provider identity and exact version
├── UTS protocol version
├── module-resolution capability
└── semantic-diagnostics capability
```

A provider version mismatch, crash, malformed message, incomplete snapshot, or unsupported syntax fails closed.

## Migration to a native UTS frontend

The permanent path is:

```text
native UTS lexer → parser → resolver → ProjectSyntaxSnapshot v1
```

Because all later phases depend on the UTS-owned snapshot and IR, replacing the bootstrap provider must not modify either backend.
