# Scalar ABI v1

Status: normative for the first three-target executable slice. For the current executable
`I32V1` profile, JavaScript and direct core WebAssembly implement sealed export mappings and the
native backend implements the Linux x86-64 `i32` symbol/calling mapping in an audited object. The
driver's sealed native invocation harness validates arguments through this authority and returns a
full-width typed `i32` through an exact four-byte channel. Boolean source and IR remain
profile-gated. Strict typed WebAssembly and native Boolean host wrappers do not implement the
complete boundary yet.

## Authority and version

`zryna-abi` is the only authority that verifies scalar ABI v1 declarations. Universal IR embeds
the resulting sealed module. Backends consume its typed views and must not sanitize names, infer
signatures, coerce host values, or define a competing mapping.

The ABI identifier is `zryna-scalar-v1`. It admits fixed-arity functions with zero to 256
parameters, exactly one result, and only `i32` or `bool`. Unit, multiple results, variadics, `i64`,
floats, strings, aggregates, heap values, async values, and implicit conversions are unsupported.

## Logical exports and target names

A logical export is 1 to 128 ASCII bytes matching `[A-Za-z_][A-Za-z0-9_]*`. The verifier rejects
ECMAScript binding keywords and the frozen defensive names `arguments`, `constructor`, `eval`,
`prototype`, `then`, and `__proto__`. Names are never normalized or case-folded. Exact duplicates
and pairs that collide under ASCII case folding fail before backend emission.

| Surface | Mapping for logical name `L` |
| --- | --- |
| ECMAScript module public export | exact `L` |
| Core WebAssembly function export | exact UTF-8 bytes of `L` |
| Linux x86-64 System V public ELF symbol | `zryna_v1_e_` followed by exact `L` |

The native `zryna_v1_e_` namespace is reserved for public exported functions. Runtime helpers and
internal symbols must use disjoint namespaces. Scalar ABI v1 does not specify Windows COFF or
macOS Mach-O symbols; an unlisted native target must fail instead of guessing a decoration.

ECMAScript allows a broader `ModuleExportName`, WebAssembly permits broader Unicode export names,
and LLVM permits escaped identifiers. Zryna deliberately uses the strict intersection above. The
ECMAScript emitter must create real named ESM exports. A WebAssembly exporter must use the external
export name, not a function index or optional name-section string. Native emission must use
external linkage and the target C calling convention without LLVM's `\\01` mangling escape.

Normative references:

- [ECMAScript module grammar and early errors](https://tc39.es/ecma262/multipage/ecmascript-language-scripts-and-modules.html#sec-module-semantics-static-semantics-early-errors)
- [WebAssembly core exports](https://webassembly.github.io/spec/core/syntax/modules.html#syntax-export)
- [WebAssembly core names](https://webassembly.github.io/spec/core/syntax/values.html#syntax-name)
- [LLVM identifiers and calling conventions](https://llvm.org/docs/LangRef.html#identifiers)

## Scalar representations

| Zryna type | JavaScript boundary | Core WebAssembly boundary | Linux x86-64 native public boundary |
| --- | --- | --- | --- |
| `i32` | primitive Number, finite integral signed 32-bit, excluding negative zero | `i32`, every bit pattern | System V 32-bit integer carrier, every bit pattern |
| `bool` | primitive Boolean only | `i32`, exactly `0` or `1` | future public 32-bit integer carrier, exactly `0` or `1` |

JavaScript strings, BigInts, truthy values, fractions, NaN, infinities, out-of-range numbers, and
negative zero are invalid arguments rather than coercion inputs. A WebAssembly or native Boolean
argument other than `0` or `1` is invalid before the function body. Boolean results must also be
canonical; another raw result is a target ABI failure and must not be normalized by truthiness.
After boundary validation, every target represents a Zryna Boolean internally as the typed values
`false` or `true`; JavaScript must not retain a truthy non-Boolean and WebAssembly must not retain an
arbitrary nonzero `i32` as a Boolean. A future native Boolean implementation may use another
internal representation, but its public wrapper must carry Boolean values as 32-bit integers and
zero-extend valid results.

The generic WebAssembly JavaScript API applies `ToInt32` to raw calls. A Zryna host wrapper must
validate first and must not use that coercion as validation:
[WebAssembly JS value conversion](https://webassembly.github.io/spec/js-api/#towebassemblyvalue).

## Invocation and observation

An invocation names one exact logical export and supplies a typed argument vector. Unknown export,
wrong arity, and type mismatch are host validation errors before target execution. A successful
result is compared as `Returned(I32(n))` or `Returned(Bool(b))`; these variants are never equal even
when their raw carrier bits match. Program traps and host errors are separate typed outcomes.

Process exit status reports harness health only. It is never a returned scalar because operating
systems may expose only a small portion of it. Native harnesses resolve the exact sealed symbol and
return the full typed value through a structured channel.

## Shared fixtures

[`scalar-v1-fixtures.json`](scalar-v1-fixtures.json) is the byte-shared mapping and observation
fixture. `zryna-abi` validates it. The executable JavaScript integration consumes the same file for
both `i32` and Boolean carrier validation; the current `i32` source execution matrix also checks
its public wrapper directly. The WebAssembly conformance integration consumes all current
`core-webassembly` carrier cases, including canonical and invalid Boolean lanes, without enabling
Boolean source or claiming a public host wrapper. Native tests account for all 11 native cases:
the three `i32` lanes belong to the current object/execution proof, while the eight Boolean lanes
remain explicitly gated. Copied target-local cases are not normative.
