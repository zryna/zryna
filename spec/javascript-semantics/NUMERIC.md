# JavaScript numeric lowering

The JavaScript backend preserves exact Zryna operations rather than erasing every numeric type to unconstrained JavaScript `number` behavior.

Implemented `I32V1` mapping:

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

## Current scalar boundary

An emitted public function checks exact arity before executing. Each `i32` parameter and result
must be a primitive JavaScript Number, pass `Number.isInteger`, be within
`[-2147483648, 2147483647]`, and not be negative zero. Strings, BigInts, Booleans, fractions,
`NaN`, infinities, and out-of-range numbers fail instead of being coerced. Addition uses the
ECMAScript bitwise conversion only after both operands have passed boundary validation, so the
observable result is signed wrapping 32-bit arithmetic.

The generated module is deterministic UTF-8 text with LF endings, a final newline, no imports,
and exact sealed public export names. It is emitted as `.mjs`; the bootstrap TypeScript provider
does not participate in JavaScript emission.
