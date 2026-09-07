# Tooling query examples and future conformance fixtures

These are specified observations for [semantic queries v1](SEMANTIC_QUERIES_V1.md), Issue
[#363](https://github.com/zryna/zryna/issues/363). They are not output from an implemented query
service. JSON blocks are representative logical records; no new public wire protocol is active.
The later independent fixture runner must check both producer output and hostile consumer input,
not derive its oracle from the implementation under test.

## Golden source and scope

`src/main.zry` contains exactly this JSON string's decoded UTF-8 bytes (52 bytes, one final LF):

```json
"export function identity(x: i32): i32 { return x; }\n"
```

The host has authenticated this one-file closure under the existing `data-ownership-v1` profile,
syntax protocol 4 and a fixed compiler/semantic revision. Symbol `identity` occupies `[16,24)`;
parameter `x` occupies `[25,26)` and its bound use `[47,48)`. The parameter is function-local;
renaming it does not rename the exported function or its ABI. All examples below assume this
complete verified analysis, no other declarations and a host-issued `s1`, revision `1`.

### Hover

```json
{"query_version":1,"request_id":"q1","snapshot":"s1","revision":1,"method":"hover","params":{"path":"src/main.zry","byte_offset":47},"limits":{"work":100000,"results":10000}}
```

```json
{"query_version":1,"request_id":"q1","snapshot":"s1","revision":1,"status":"ok","result":{"symbol":{"path":"src/main.zry","byte_start":25,"byte_end":26,"kind":"parameter"},"type":["scalar","i32"],"range":{"path":"src/main.zry","byte_start":47,"byte_end":48},"display":"x: i32"}}
```

### Definition and references

```json
{"query_version":1,"request_id":"q2","snapshot":"s1","revision":1,"method":"definition","params":{"path":"src/main.zry","byte_offset":47},"limits":{"work":100000,"results":10000}}
```

```json
{"query_version":1,"request_id":"q2","snapshot":"s1","revision":1,"status":"ok","result":{"locations":[{"path":"src/main.zry","byte_start":25,"byte_end":26}]}}
```

```json
{"query_version":1,"request_id":"q3","snapshot":"s1","revision":1,"method":"references","params":{"symbol":{"path":"src/main.zry","byte_start":25,"byte_end":26,"kind":"parameter"},"include_declaration":true},"limits":{"work":100000,"results":10000}}
```

```json
{"query_version":1,"request_id":"q3","snapshot":"s1","revision":1,"status":"ok","result":{"locations":[{"path":"src/main.zry","byte_start":25,"byte_end":26},{"path":"src/main.zry","byte_start":47,"byte_end":48}]}}
```

With `include_declaration: false`, only `[47,48)` remains. Reversing those two locations in a
claimed response must fail the later consumer validator; do not silently sort hostile output.
Cross-file results sort by unsigned UTF-8 path bytes first, then numeric start/end, regardless
of provider enumeration or request traversal order. Exact duplicate locations are removed.

### Read-only rename plan

```json
{"query_version":1,"request_id":"q4","snapshot":"s1","revision":1,"method":"prepare_rename","params":{"symbol":{"path":"src/main.zry","byte_start":25,"byte_end":26,"kind":"parameter"},"new_name":"value"},"limits":{"work":100000,"results":10000}}
```

```json
{"query_version":1,"request_id":"q4","snapshot":"s1","revision":1,"status":"ok","result":{"edits":[{"path":"src/main.zry","byte_start":25,"byte_end":26,"old_text":"x","new_text":"value"},{"path":"src/main.zry","byte_start":47,"byte_end":48,"old_text":"x","new_text":"value"}]}}
```

Only after all rename preconditions and revision checks pass may a consumer apply the second
edit, then the first, as one transaction. Expected complete bytes are:

```json
"export function identity(value: i32): i32 { return value; }\n"
```

The new revision has new symbol ranges and invalidates `s1`; the old plan cannot be replayed.
Renaming the exported `identity` function is `conflict` / `rename` in v1 because its public
spelling would change. Renaming `x` to `return` has the same outcome (reserved identifier).

### Diagnostic reuse

```json
{"query_version":1,"request_id":"q5","snapshot":"s1","revision":1,"method":"diagnostics","params":{},"limits":{"work":100000,"results":10000}}
```

```json
{"query_version":1,"request_id":"q5","snapshot":"s1","revision":1,"status":"ok","result":{"report":{"schema_version":2,"diagnostics":[]}}}
```

The empty report only means no diagnostic records; it does not prove query, compiler, runtime or
target support. Also transport the existing #169
[golden](../../crates/zryna-diagnostics/src/protocol_v2/fixtures/golden.json) and
[exhausted](../../crates/zryna-diagnostics/src/protocol_v2/fixtures/exhausted.json) reports unchanged,
bound to their own exact fixture sources. `ZRYNA-D2001` remains incomplete even inside `ok` transport.

## Negative requests and races

Each mutation below independently starts from the indicated golden request. Unless stated,
the active pair remains `s1`/`1`. Each negative response has only the correlation fields,
`status` and `reason`, as in this stale example (the active pair is now `s2`/`2`):

```json
{"query_version":1,"request_id":"q1","snapshot":"s1","revision":1,"status":"stale","reason":"snapshot"}
```

| Fixture ID | Independent input or schedule | Exact expected status / reason |
| --- | --- | --- |
| stale-same-length | Replace both `x` tokens with `y`, retain request q1 | `stale` / `snapshot`, despite valid old offsets |
| stale-undo | Edit then undo to identical bytes; replay q4 | `stale` / `snapshot`, no edits |
| stale-race | q3 begins on s1; publish s2 before q3 completes | `stale` / `snapshot`, no location prefix |
| stale-apply | q4 succeeds; another document changes before application | Reject entire application as stale; no file modified |
| foreign-session | A second session sends the same handle spelling | `stale` / `snapshot` unless it independently owns that handle; never reuse the first session's authority |
| stale-malformed-precedence | q1 has unknown envelope field and old revision | `malformed` / `shape` before stale lookup |
| malformed-shape | Duplicate `revision` key, missing limits, fractional revision, null, lone surrogate, trailing object, unknown field | `malformed` / `shape` |
| malformed-path | q1 path becomes `../src/main.zry`, `SRC/main.zry`, an absolute path or a missing file | `malformed` / `source` |
| malformed-position | q1 byte_offset is 53 (one past EOF) | `malformed` / `source` |
| malformed-symbol | q3 claims `[25,26)` has kind `function` | `malformed` / `source` |
| unsupported-version | q1 query_version becomes 2 | `unsupported` / `version` |
| unsupported-method | q1 method becomes `completion` | `unsupported` / `method` |
| absent-token-end | q1 byte_offset becomes 48 or 52 | `absent` / `symbol` |
| rename-reserved | q4 new_name becomes `return` | `conflict` / `rename` |
| rename-export | q4 symbol is function `identity`, new_name `other` | `conflict` / `rename` |
| reference-limit | q3 results becomes 1 (two locations required) | `over_budget` / `results`, no prefix |
| work-limit | q1 work becomes 1 (more than one examined record/byte needed) | `over_budget` / `work`, no hover |
| request-byte-limit | Pad a valid JSON request with whitespace to 65,537 bytes | `over_budget` / `request_bytes` before parse |
| request-depth-limit | A bounded request has JSON nesting depth 65 | `over_budget` / `request_depth` before shape materialization |
| cancel-recovery | Cancel q3 during enumeration; then query fresh valid q3 under a new ID | `cancelled` / `request`; subsequent output equals fresh invocation |
| analysis-pending | q1 targets a current source-only record awaiting verified semantic facts | `unavailable` / `analysis` |
| invalidation-limit | Editing a valid dependency closure visits edge 100,001 | `over_budget` / `invalidation`; new revision unavailable until bounded full rebuild |

Byte/depth exhaustion must also be tested with malformed input: admission wins. Unsupported
versions are integers with valid envelope shape. Limit values outside the admitted range are
`malformed` / `shape`; an admitted limit exhausted during evaluation is `over_budget`.
Budget hostiles must use small encoded descriptors to drive large logical work, not preallocate
unbounded fixture arrays. Independently test exact inclusive ceilings and first-extra values.

## Unicode and source-map oracle

This separate `src/unicode.zry` source has 66 UTF-8 bytes. The comment prefix is 14 bytes:

```json
"// é😀e\u0301\r\nexport function identity(x: i32): i32 { return x; }\n"
```

All line and column numbers below are zero-based. UTF-8 columns count bytes; UTF-32 columns count
scalars. Combining marks are not composed. These are source-coordinate fixtures, not permission
for non-ASCII identifiers in the current grammar.

| Boundary | Byte offset | Line | UTF-8 column | UTF-16 column | UTF-32 column |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before é | 3 | 0 | 3 | 3 | 3 |
| Before astral scalar | 5 | 0 | 5 | 4 | 4 |
| After astral scalar | 9 | 0 | 9 | 6 | 5 |
| Before combining mark | 10 | 0 | 10 | 7 | 6 |
| Before CR | 12 | 0 | 12 | 8 | 7 |
| Next line | 14 | 1 | 0 | 0 | 0 |
| Parameter x | 39 | 1 | 25 | 25 | 25 |
| Reference x | 61 | 1 | 47 | 47 | 47 |
| EOF after final LF | 66 | 2 | 0 | 0 | 0 |

Bytes 4, 6–8 and 11 split scalars and reject. UTF-16 column 5 splits the astral surrogate pair
and rejects. Byte 13 is a valid source boundary inside CRLF but must not be rounded to another
editor position; reject an edit if the negotiated line model cannot round-trip it. Also require
lone CR, lone LF, tabs, empty files, no final newline, EOF ranges, embedded NUL and ill-formed
UTF-8 hostiles. Source admission owns any rejected source bytes; the query layer cannot repair them.

## Required later fixture families and gates

| Family | Independent positive and hostile evidence | Measurable gate |
| --- | --- | --- |
| Closed records | Goldens above; unknown/missing/duplicate fields, bad versions, reordered results, forged type/symbol authority, escaped strings and overflow integers | Every accepted record round-trips; every hostile rejected at the specified boundary |
| Symbols/types | Scalar and every admitted descriptor; same-named shadowed locals, nominal declarations in different modules, function parameter order, borrowed/container/array distinctions | Exact compiler-derived identity, no provider IDs, no unknown type masquerading as `i32` |
| Cross-file queries | Definition through imports, local alias references versus exported-declaration references, complete closure in permuted provider order | Fixed path/span oracle; missing/extra/reordered/duplicate locations fail consumer validation |
| Stale/incremental | Same-length edit, undo, overlay close, dependency/signature edit, package/config/profile change, eviction, session restart, racing completion | Zero stale publications or edits; incremental equals full rebuild; bounded failure then clean recovery |
| Rename | Golden private parameter; capture, duplicate declaration, alias collision, readonly dependency, unresolved external uses, overlapping plan, old-text mismatch, partial write failure | All-or-nothing application; compiler binding/type/ownership equality modulo rename; no public export rename |
| Bounds | Every table ceiling exactly and first-extra, escape expansion, type graph depth, queue/cache admission, cycles, cancellation and deadline cleanup | No partial result, unbounded allocation/work or poisoned next request; cold/warm logical outcomes agree |
| Providers / #170 | Positive, malformed, unsupported, budget, recovery and ordering sessions, two runs per provider | Exact capabilities and raw per-provider determinism; equivalent verified facts; diagnostic comparison follows #170 |
| Corpus integrity | Independently delete, insert, reorder, case-collide or tamper a manifest entry or fixture | Registry rejects before comparison; a producer cannot regenerate its own expected results |
| Diagnostics / #169 | Existing v2 goldens/hostiles, foreign same-length source, terminal exhaustion, Unicode ordering, inert hostile text | Preserve v2 report bytes/meaning; exact snapshot binding; no second diagnostic authority |
| Formatter | Line/block/doc comments at file start/end and between tokens; comment markers in strings, CRLF/tabs/no-final-LF; unsupported/malformed syntax | Exact ordered comment bytes and attachment, token/parse equivalence, `F(F(s)) = F(s)`, compiler equivalence through mapped spans |
| Syntax-only test/doc | Comments attached to declaration, reordered source inventory, malformed trivia partitions and inert markup | Stable extraction without semantic service or implicit execution |
| Debugging | Unicode and multi-file maps, stale artifact hash, one-to-many mappings and unmapped optimized code | Exact source/artifact binding; deterministic mapping; no invented breakpoint/variable |
| Playground / #362 | Query-only session, attempted network/process/credential access, native recipe, dependency substitution, runaway execution and failed cleanup | Execution disabled until package/runtime/isolation policy passes; each denied action independently rejected |

The formatter fixture sources must include `// keep\n` before a declaration, `/* keep */`
between parameter tokens, a documentation comment attached to a function, and `"// text"` inside
an admitted string literal. An unchanged comment multiset is insufficient: assert original order,
attachment and exact bytes. For invalid semantic programs with valid syntax, compare compiler
diagnostic codes/severities and mapped token ranges before/after, rather than demanding execution.

Later fixtures pin source bytes/digests, semantic/profile versions, requests, full expected
responses, rejection stage and registry order. Recovery uses independent fresh state as its oracle.
Each implementation slice reports exact revision, platform, executed and ignored counts; a fixture
inventory is not a passed suite. Query conformance cannot substitute for consumer package, runtime,
isolation or publication gates in the [activation table](SEMANTIC_QUERIES_V1.md#consumer-slices-and-measurable-activation-gates).
