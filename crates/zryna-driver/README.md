# Zryna compiler driver

The only compiler component allowed to orchestrate frontend, verification, and backend phases.

`analyze_sources` accepts one authoritative `SourceMap` and a configured process frontend. It
returns only the opaque protocol-v2 snapshot that the frontend crate has authenticated, decoded,
bounded, and verified against that exact map. Raw worker bytes and untrusted syntax DTOs are not a
driver-facing contract.
