# JavaScript numeric lowering

The JavaScript backend preserves exact Zryna operations rather than erasing every numeric type to unconstrained JavaScript `number` behavior.

Initial mapping:

```text
I32Add(lhs, rhs) → (lhs + rhs) | 0
```

Planned mappings require dedicated conformance suites:

```text
u32 → unsigned 32-bit number coercions
f32 → Math.fround-based operations
f64 → JavaScript number operations with specified edge cases
i64 → BigInt with signed 64-bit normalization
u64 → BigInt with unsigned 64-bit normalization
```

Every operation must define overflow, `NaN`, negative zero, conversion, equality, and serialization behavior where applicable.
