# Semantic queries and tooling snapshots v1

Status: proposed contract for [#363](https://github.com/zryna/zryna/issues/363), native
[M6 — Developer Tooling](https://github.com/zryna/zryna/milestone/7). Review accepts a
specification, not an implemented service. No transport endpoint, public selector, compiler or
runtime implementation, extension release, or executable capability is introduced here.

The [examples and future fixtures](QUERY_EXAMPLES_V1.md) are normative observations for later
implementation, not recorded executions. Existing M0–M3 acceptance gates, including the order
of #89 then #90 closure, and historical digest-pinned inventories remain unchanged.

## Ownership and dependencies

| Authority | Responsibility |
| --- | --- |
| `zryna-source` | Immutable exact UTF-8 text, portable paths, source identity and coordinate validation |
| `zryna-syntax` | Independently verified provider-neutral syntax and any future authenticated trivia view |
| `zryna-semantics` | Name resolution, scopes, exact types and references; owns query facts and rename proofs |
| `zryna-diagnostics` | Existing compiler diagnostic codes and explicitly selected structured transport |
| Driver and future tooling host | Compose these authorities, retain sessions, bound scheduling and invalidate results |
| Consumers | Convert coordinates, render inert text, present plans and apply only revision-checked edits |

These are responsibilities, not new component registrations or dependency edges. A later service
must follow the [architecture](../../docs/ARCHITECTURE.md) and
[strict workspace](../../docs/STRICT_WORKSPACE.md): semantics never depends on a frontend
provider; syntax-only consumers need not depend on a semantic service. No consumer may recreate
resolution, type/ownership checking, architecture rules, ABI rules or diagnostic authority.

- [#169](https://github.com/zryna/zryna/issues/169) is the diagnostic prerequisite, merged at
  `1e8674f3a8288d71933e2dbbd6408a113b580cd8`. Reuse
  [structured diagnostics v2](../diagnostics/STRUCTURED_DIAGNOSTICS_V2.md) unchanged. Its opt-in
  library transport does not already provide editor integration.
- [#170](https://github.com/zryna/zryna/issues/170), a secondary relationship to
  [M7](https://github.com/zryna/zryna/milestone/8), owns provider conformance. Query expectations
  align with that corpus; neither this review nor query implementation depends on a native lexer,
  parser, resolver, provider selector or replacement of the bootstrap TypeScript 6 adapter.
- [#362](https://github.com/zryna/zryna/issues/362), a secondary relationship to
  [M5](https://github.com/zryna/zryna/milestone/6), owns package/build trust and execution policy.
  Only the playground execution appendix depends on that decision. Core query review is independent.

## Snapshot identity and lifecycle

A host retains an immutable record for each admitted revision, containing:

1. a session-local, opaque `snapshot` handle and monotonic nonzero `revision`;
2. the exact source-map authority, canonical entrypoint and complete path-sorted source closure;
3. each file's exact bytes and content digest, including in-memory overlays and their editor versions;
4. the verified module graph, syntax protocol, language profile and semantic contract identity;
5. compiler revision, resolution/configuration inputs, and any authenticated package/lock identities;
6. completed syntax/semantic/diagnostic views and their independently bounded cache records.

The host issues the handle only after source authentication. A failed parse may retain a source-only
snapshot for diagnostics; it does not acquire verified syntax or semantic facts. Missing readiness
is explicit. A client-supplied handle, digest, numeric file ID or deserialized DTO is never proof of
source ownership. Every query resolves its handle through the session's retained record. Within
the process, views retain the exact issuing source-map identity, including for empty source sets.

For reproducible comparisons, the source fingerprint is SHA-256 of UTF-8 compact JSON for
`[[path, sha256(exact_file_bytes)], ...]`, in unsigned UTF-8 path order, without a BOM or final
newline. Paths obey the existing source contract; neither text, Unicode nor line endings are
normalized. Object/property order is absent from this array encoding; JSON string escaping uses
the structured-diagnostics convention. This fingerprint alone does not bind entrypoint, graph,
profile or configuration. Cache keys must additionally bind all inputs in items 4 and 5 using
versioned canonical records, and compare the retained authorities before reuse. An implementation
must freeze those cache records in its own reviewed boundary; they are not public query IDs.

Content equality enables comparison, never handle substitution. Reopening a file, edit-and-undo,
closing an overlay, changing dependencies, switching profile/configuration, or restarting the host
invalidates applicable handles even if paths and byte lengths match. Revisions never wrap or reset
inside a session; exhaustion requires a new session. Handles are not reused. Outstanding requests
are tagged with the full handle/revision pair and cancelled on replacement. A retained old snapshot
may finish work internally, but its result is rejected at publication and edit application.

Publication and edit application both compare the expected active revision atomically with the
operation. Notifications and asynchronous responses carry that same pair. A client discards any
result whose document versions or active pair differ; arrival order cannot override revision order.
An expired, evicted, foreign-session or unknown handle returns `stale`, with no facts or edits.

## Coordinates and source maps

All query positions are zero-based unsigned 32-bit UTF-8 byte offsets. Locations contain exactly `path`,
`byte_start`, `byte_end`; ranges are half-open. The exact retained source map proves canonical
path spelling, file membership, `0 <= start <= end <= byte_length`, and scalar-boundary endpoints.
EOF and zero-width ranges are valid. A position at a token's end does not select that token;
there is no implicit left bias. Whitespace, comments and EOF yield `absent` for symbol queries.
The smallest containing identifier token is selected; semantic ownership resolves its declaration.

Editor coordinates are derived against the same bytes. LSP-style adapters explicitly negotiate
UTF-8, UTF-16 or UTF-32 units and use zero-based line/column pairs; absent negotiation uses the
consumer protocol's documented default, not a guess from the offset. UTF-16 surrogate splits,
UTF-8 scalar splits, out-of-range columns and ambiguous line-terminator positions reject as
`malformed`; never clamp. CRLF is one break after LF, lone CR and LF are each one break. The
source authority can represent the byte boundary between CR and LF; editor edits at that boundary
are rejected when the negotiated line model cannot round-trip it exactly. Tabs count as encoded
characters, not visual tab stops; combining scalars and astral scalars retain their exact units.

Debugging uses a separate, later backend-owned mapping from authenticated artifact digest and
generated range to this source handle/fingerprint and range. Queries do not invent executable
addresses, expand optimized-away variables or imply complete mappings. A missing mapping is
explicitly unmapped; an artifact/source mismatch rejects. Reverse mapping may be one-to-many and
must be deterministically ordered. Backend mapping/version and debugger runtime gates are required
before breakpoints or stepping are supported.

## Symbol and type scope

A `symbol` is the canonical declaration-name location plus `kind`: `function`, `parameter`,
`local`, `struct`, `field`, `enum`, `variant` or `import`. It is unique only inside the enclosing
snapshot and semantic contract. Distinct shadowed declarations remain distinct; source spelling
alone is never identity. Import bindings have their own local identity; definition follows their
verified target. A reference query on an import targets that local binding, whereas one on the
exported declaration includes verified uses through imports in the complete admitted closure.
Any shape that cannot establish this distinction returns `unsupported`, not guessed references.

A `type` is a bounded canonical descriptor, not a provider ID or display string. Its closed forms
are arrays: `["scalar", name]` for an admitted exact scalar, `["owned", "String"]`,
`["nominal", symbol]`, `["container", name, type]` for `Vec`, `Shared`, `Weak`, `Borrow` or
`BorrowMut`, `["array", length, type]`, and `["function", [parameter_types...], result_type]`.
Nominal members are not recursively expanded. Scalars and forms are admitted only by the selected
compiler profile; this vocabulary grants no new language support. Type equality is structural
equality of the canonical descriptor under one snapshot, with nominal equality by declaration
identity. Borrow mode, container kind and array length are part of identity. There is no structural
equating of separate nominal declarations, host layout identity, provider type number or inferred
`any`. Unknown/error/incomplete types produce `unavailable`, never a fabricated descriptor.

Compiler-derived descriptors and symbols may be compared across equivalent snapshots after
checking every semantic input. They cannot authorize reuse of a handle across revisions. Rename
or text motion changes location-based identity; no durable database identifier is promised.
Presentation text may change with compiler revision and is not used as a lookup key.

## Request and response observations

The JSON examples define logical closed records for a future internal contract, not an installed
wire protocol. A later transport must freeze duplicate-key rejection, canonical serialization and
independent schema/runtime validation before a prototype claims conformance. Version `1` below
is independent of syntax v4, diagnostic v2, manifest version and language profile; unknown versions
never downgrade. Changing these records, ordering, units, failure semantics or limits requires a
new query contract version.

Requests have exactly `query_version`, `request_id`, `snapshot`, `revision`, `method`, `params`,
`limits`. IDs are nonempty ASCII strings of at most 128 bytes, revisions unsigned integers in
`1..2^53-1`, and `query_version` is `1`. `limits` contains exactly `work` and `results`, positive
integers at most the host ceilings below. Duplicate in-flight request IDs reject as `malformed`.
Methods and their closed `params` are:

| Method | Params | Successful `result` |
| --- | --- | --- |
| `hover` | `path`, `byte_offset` | `symbol`, `type`, `range`, inert compiler `display` string |
| `definition` | `path`, `byte_offset` | `locations` array of declaration-name ranges |
| `references` | `symbol`, `include_declaration` Boolean | `locations` array of bound uses, optionally declarations |
| `prepare_rename` | `symbol`, `new_name` | `edits` array of exact locations with `old_text` and `new_text` |
| `diagnostics` | Empty object | `report`, exactly the existing structured-diagnostics-v2 object |

Successful responses contain exactly `query_version`, `request_id`, `snapshot`, `revision`,
`status: "ok"`, `result`. Negative responses replace `result` with `reason` and a non-`ok` status
from the next section; no null, partial facts, guessed type, partial reference list or edit prefix.
If a malformed envelope lacks safely decoded correlation fields, the transport reports an
uncorrelated request rejection; it must not copy attacker-selected malformed data into an echo.
Transport details of that rejection require the later framing contract, not a compiler diagnostic.

Locations sort by unsigned UTF-8 `(path, byte_start, byte_end)` with numeric offsets. Exact
duplicate locations are removed before publication. Edits use that same ascending order, are
nonoverlapping, and contain no duplicate ranges. Application uses descending offsets within each
file after validating the whole plan. Symbol object fields are `path`, `byte_start`, `byte_end`,
`kind` in that order. Type arrays preserve parameter order. Hover is a single fact. Diagnostic
order and duplicates remain exactly #169's rules, not the location-list rule.

Hover display and documentation are inert text; no HTML, terminal escapes, commands, URI opens or
executable guidance. Diagnostics remain owned by their producing phases. The enclosing snapshot
pair supplies the revision binding absent from diagnostic v2 itself; a syntactically valid report
from another same-length source is still stale. Preserve ordinary codes, severity, messages and
guidance. A `ZRYNA-D2001` terminal report remains an incomplete report, never successful analysis.
Query failures below are service outcomes, not new `ZRYNA-*` language diagnostic codes.

Before consuming any response, validate its closed shape/version, exact correlation pair and ID,
method-specific result shape, source membership and bounds, symbol/descriptor shape, ordering,
uniqueness, edit nonoverlap and all output limits. Reject a malformed or mismatched response as a
whole; do not repair it. Shape validation authenticates neither a compiler nor semantic truth: the
host must deliver facts from the retained compiler authorities over its trusted session boundary.
For `diagnostics`, readiness means that the requested diagnostic pass has completed, even if it
reports parse errors. Hover/definition/references/rename require their complete semantic views;
absence of those views never prevents displaying an available authoritative diagnostic report.

## Failure precedence and bounded work

Evaluate these checks in order; an earlier failure wins. Within a stage use request field order
above, then path/span order. An unknown method is tested before interpreting its method-specific
params, after bounded JSON shape and snapshot correlation have been checked.

| Stage | Status / reason | Required effect |
| --- | --- | --- |
| Encoded admission bound | `over_budget` / `request_bytes` | Reject before unbounded parsing/allocation |
| JSON/closed envelope/number/limit shape | `malformed` / `shape` | Duplicate/unknown/missing fields, invalid UTF-8, lone surrogates, trailing JSON reject |
| Query version | `unsupported` / `version` | No silent fallback |
| Active handle and revision | `stale` / `snapshot` | No resolution through current text for an old request |
| Method and method-specific shape | `unsupported` / `method`, or `malformed` / `params` | No method fallback |
| Exact paths, spans, symbol structure | `malformed` / `source` | Reject traversal, case aliases, foreign files, invalid offsets and forged declaration kinds |
| Cancellation observed | `cancelled` / `request` | Discard all pending output |
| Required compiler view | `unavailable` / `analysis` | Parsing/semantic failure, pending recomputation or incomplete facts never become empty success |
| Query capability for selected profile/construct | `unsupported` / `construct` | Reject unimplemented reference/rename/type coverage explicitly |
| No selected or bound symbol | `absent` / `symbol` | Valid position with no semantic target; known symbol with zero uses instead returns an empty list |
| Rename proof | `conflict` / `rename` | No edit if any precondition below fails |
| Evaluation or output bound | `over_budget` / `work`, `results` or `response_bytes` | Discard every partial result; no continuation token |
| Final active-revision check | `stale` / `snapshot` | Suppress a result made stale while work was running |

The JSON envelope depth ceiling is 64; exceeding it is `over_budget` / `request_depth` during
admission, after the byte check and before materializing nested values. Limits are inclusive:

| Resource | Proposed v1 ceiling |
| --- | ---: |
| Encoded request / response | 65,536 / 1,048,576 UTF-8 bytes |
| Per-query work units | 100,000 |
| Published locations or edits | 10,000 |
| Type descriptor nodes / depth; hover display | 4,096 / 64; 4,096 UTF-8 bytes |
| Retained revisions / total cache bytes per session | 2 / 64 MiB |
| Queued plus running requests per session | 32 |
| Query deadline, including bounded cancellation/cleanup | 30 seconds |
| Incremental dependency edges visited / files invalidated per revision | 100,000 / 4,096 |

A work unit is one visited syntax node, semantic record, dependency edge, or decoded source byte
examined by the query; each visit charges separately, including cache validation and sorting
comparisons (one unit per compared record pair). Hashing/scanning charges each byte. Charge before
the operation with checked arithmetic. Source authentication and compiler analysis retain their
existing independent bounds; query budgets cannot expand those limits. Descriptor/display limits
count toward output admission and return `over_budget` / `response_bytes`. Session admission or
cache exhaustion returns `over_budget` / `session`; invalidation excess uses `invalidation`.

No cache hit may turn an over-budget logical query into a passing one: cache entries retain their
canonical evaluation cost, including mandatory authority validation, and replay is charged that
same logical cost. Actual cache maintenance has its own session bound and cannot expand a query's
admitted limits. Deterministic query observations compare cold and warm caches at identical limits.
A separate monotonic
deadline and explicit cancellation bound real scheduling/process work; a deadline produces
`cancelled` / `deadline`, never fabricated compiler results. Timing-dependent cancellations are
tested separately from deterministic language observations. Queue saturation rejects before work;
the host never recursively expands an unbounded type, reference graph or JSON structure.

## Incremental invalidation and rename transactions

Edits are submitted with expected revision and exact old text/hash. Apply a bounded batch to a
new immutable source record; reject overlapping ranges, split characters, file-set/path aliases
and version mismatch before mutation. A local edit invalidates its syntax, trivia, diagnostics and
semantic facts. Conservatively invalidate reverse dependencies, including resolution, imported
types, signatures and uses; changing file membership, entrypoint, package/lock/configuration or
profile invalidates the whole closure. Traverse a sorted worklist, visit each dependency once and
charge the invalidation bounds above. When a bound is exceeded, publish no partial updated view:
mark the new revision unavailable, return `over_budget` / `invalidation`, and require a separately
bounded full rebuild. Do not silently reuse the old revision or exceed budgets to finish.

Unaffected cached facts may be reused only with proof that their exact source and transitive
semantic inputs match. Rebind through the new source authority, never transplant old spans or
numeric IDs. Full recomputation and incremental recomputation must have identical canonical
observations. Eviction releases authority only after bounded outstanding users are cancelled;
unknown/evicted handles remain stale. Failed or cancelled work must leave a subsequent valid
request observationally equal to a fresh request, with no leaked partial plans or poisoned caches.

`prepare_rename` is a read-only plan, not permission to write. Semantics must prove all of:

- the symbol denotes exactly one writable source declaration in the authenticated complete closure;
- `new_name` is a valid nonreserved identifier in the selected Zryna grammar, at most 128 ASCII bytes;
- every affected reference and import binding is known, writable and source-bound; no unresolved,
  ambiguous, unsupported or externally unenumerable use can be omitted;
- the new name introduces no duplicate, shadow capture, changed resolution or incompatible public
  export/ABI spelling; v1 refuses externally visible exports and API-affecting renames;
- a bounded hypothetical reanalysis preserves types, ownership and reference binding modulo this
  exact rename, with no new compiler errors; all old token text matches and edits do not overlap.

A name equal to the current name returns `ok` with an empty edit list after the same proof. An
implementation unable to prove a supported case returns `unsupported` / `construct`; a proved
collision or unsafe rename returns `conflict` / `rename`. Budget failure remains `over_budget`.
Before applying, the consumer rechecks every affected document version, source hash, old token
text and active snapshot. Apply all edits as one transaction or fail without edits. Consumers
unable to guarantee that property must refuse application. Any resulting revision requires fresh
compiler validation; an old plan never transfers to it. Code actions need their own reviewed
contract and cannot interpret diagnostic guidance as replacement text.

## Formatter and syntax-only consumers

Current executable syntax snapshots do not promise a lossless trivia inventory. A formatter first
needs a source-owned token/trivia representation validated against exact bytes, either a reviewed
new syntax contract or a separate verified lossless view. It must not extend protocol v4 silently
or infer all comments from AST gaps. Trivia entries carry exact ranges, kind and token attachment;
their partition must cover all non-token bytes exactly once without overlap, omission or invention.
This view preserves line/block/doc comments, order, exact comment bytes and attachment to the same
declaration/token boundary. Comment-like text inside strings remains string content.

Versioned formatting options explicitly choose indentation and newline policy. Defaults preserve
existing line endings and final-newline presence. Whitespace edits must not merge tokens, move a
line comment across a newline, change literal bytes or reattach documentation. Malformed or
unsupported syntax, unclassified trivia, unverifiable attachment or budget exhaustion returns no
edits with the corresponding query-style outcome; v1 does not format a guessed partial parse.

For every accepted input `s`, options `o`, and formatter version `v`, require byte idempotence
`F(F(s,o,v),o,v) = F(s,o,v)`, identical parsed token/declaration structure modulo whitespace spans,
and equivalent compiler semantic observations modulo the verified source-range mapping. Invalid
semantic programs may still format when syntax is complete: preserve their compiler diagnostic
codes/severities and corresponding token locations instead of claiming semantic success. The
formatter does not implement semantic checks; conformance invokes the compiler independently.
Syntax-only formatting, comment extraction and test discovery need no live semantic service.

## Provider-independent observations

For identical exact source inventories/bytes, entrypoint, graph, profile, semantic contract,
compiler revision and admitted limits, equivalent verified snapshots must produce the same
symbols, types, ranges, reference order, rename plans and compiler-owned diagnostic observations.
Session handles, revisions and request IDs are correlation metadata and may differ between runs;
the comparison binds them to the corresponding fixture rather than comparing their spelling.
No provider AST identity, type number, symbol number, host path or traversal order may escape.

The #170 comparison boundary remains distinct: exact protocol-v4 handshake/capabilities, UTF-8
spans, module specifiers, literal spellings and canonical source order; complete deterministic raw
output across two runs of each provider; provider-equivalent canonical snapshots or stable
diagnostic codes. Provider diagnostic prose is deterministic within a provider and may differ
between providers under #170; tooling preserves it unchanged. Compiler-semantic diagnostics on
equivalent verified input are compared under the same compiler revision. This does not turn
provider-specific prose into a semantic identity or permit normalizing away a different span,
missing declaration, literal or result order.

The pinned TypeScript 6 adapter remains the bootstrap authority. Source completeness inside a
file remains the provider conformance obligation described in [FRONTENDS.md](../../docs/FRONTENDS.md);
DTO acceptance alone cannot prove a provider omitted nothing. Later query fixtures must consume
#170's positive, malformed, unsupported, budget, recovery and ordering classes and independently
reject missing, extra, reordered, case-colliding and tampered corpus entries. Provider selection
and that registry/checker belong to #170; this document adds no alternate implementation.

## Consumer slices and measurable activation gates

| Slice | Prerequisites | Conformance and separate activation |
| --- | --- | --- |
| Source/session and diagnostics | Source authority; merged #169 | All coordinate, stale and diagnostic fixtures; session transport/security review before editor publication |
| Hover and definition | Verified syntax and complete semantic facts | Golden and hostile facts through each admitted provider; no native provider required |
| References and rename | Complete closure, binding graph, hypothetical reanalysis and atomic client edits | Shadow/import/collision/readonly/stale multi-file fixtures all pass before edits are enabled |
| Formatter | Verified lossless syntax/trivia and options | Comment preservation, parse equivalence and byte idempotence on every accepted fixture; independent compiler equivalence checks |
| Test discovery and documentation extraction | Syntax, spans and authenticated comments | Deterministic extraction fixtures; types/cross-links optionally use queries; no execution on discovery |
| Test runner and runnable documentation | Their own package identity/build plan, selected runtime and target support | Independent package, isolation, invocation, resource and target-result gates; discovery does not authorize running code |
| Debugger | Backend artifact-bound source maps and supported debug runtime | Unicode/multifile/unmapped/optimized and stale-artifact fixtures before breakpoints or stepping |
| Playground/editor distribution | Revision binding, inert rendering and consumer packaging | Separate package provenance, runtime/isolation and release gates; marketplace/browser availability never follows from query conformance |

Implement in bounded stages: source/session plus diagnostic binding; read-only semantic queries;
complete references and rename planning; independent syntax/trivia and formatter work; then each
consumer's own activation. Each stage first freezes a closed schema and independent rejection
boundary, implements its subset, and passes every applicable fixture below at exact revisions on
Linux and Windows. Unsupported subsets remain explicit; none of these stages changes M3 scope.

Track four states per stage: **specified** after focused review; **prototype** after a separately
reviewed internal implementation; **conformance-passed** only with the named complete fixture and
platform evidence; **publicly-supported** only after that consumer's activation/release gates and
documentation are accepted. This document proposes the first state only. No dates, reviewers,
runtime support or later evidence are invented.

### Playground execution appendix — conditional on #362 / M5

Submitting text for queries is not permission to build, fetch dependencies or execute it. The
eventual playground must consume #362's accepted package/source and build-execution policy,
including its source-only/native distinction; it cannot define an alternative policy here.
Until that policy and separate package/runtime/isolation gates pass, execution stays disabled.

The later activation proof must demonstrate: no ambient filesystem, credentials, environment,
network or process access; authenticated pinned dependency inputs; bounded CPU, memory, wall time,
output and storage; containment-wide cancellation and confirmed cleanup; isolated sessions and
no cross-user cache/source leakage. Any granted capability requires the owning profile's explicit
reviewed declaration. A manifest permission or WebAssembly label alone is not an operating-system
sandbox. Native recipes cannot run in the untrusted playground merely because a local trusted
build permits them. Failed isolation, stale execution input or incomplete cleanup fails closed.
These are required proofs for later activation, not newly specified allowed tools or an isolation
implementation. Core query/formatter review remains independent of this appendix's dependency.

## Verification of this specification

[QUERY_EXAMPLES_V1.md](QUERY_EXAMPLES_V1.md) fixes representative source bytes, records, negative
outcomes and a later fixture matrix. Those requirements are not executed query tests. This change
must run documentation/link/example consistency checks, `pnpm docs:check`, `pnpm structure:check`,
`pnpm install --frozen-lockfile`, `pnpm preflight` and `pnpm m0:check` with the pinned toolchains.
No executable query schema is introduced; existing diagnostic/schema gates remain authoritative.
Report actual commands and outcomes separately from future conformance requirements.
