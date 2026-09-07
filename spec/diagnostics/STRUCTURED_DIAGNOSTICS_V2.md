# Structured diagnostics v2

Status: opt-in library transport contract for Issue #169. This does not activate a CLI
flag, language server, formatter, editor extension or playground integration. Existing
`render_text`, `render_structured`, `render_json` and `STRUCTURED_DIAGNOSTICS_VERSION = 1`
remain unchanged. V2 is explicitly selected through `zryna_diagnostics::protocol_v2`.

## Authority and closed shape

The [JSON Schema](../../schemas/zryna-diagnostics-v2.schema.json) describes the closed
wire shape. The [Rust boundary](../../crates/zryna-diagnostics/src/protocol_v2.rs)
additionally enforces byte budgets, canonical ordering, terminal records and source
binding. Schema acceptance alone is insufficient. No new component or dependency edge
is introduced: diagnostics still depends only on the source foundation and existing
serialization dependencies.

One UTF-8 JSON object contains exactly `schema_version: 2` and `diagnostics`, an array.
Every diagnostic contains exactly these fields, in canonical serialization order:

| Field | Contract |
| --- | --- |
| `code` | ASCII `ZRYNA-[A-Z][0-9]{4}`; stable code owned by the producing compiler phase |
| `severity` | Exactly `error` or `warning` |
| `location` | One closed variant described below |
| `message` | Unicode string, displayed as inert text |
| `guidance` | Unicode string, displayed as inert text; no executable fix or command authority |

Location variants are mutually exclusive. Their fields are serialized in table order:

| `kind` | Remaining fields | Meaning |
| --- | --- | --- |
| `global` | None | Locationless failure or warning |
| `workspace-path` | `path` | Verbatim workspace diagnostic display label |
| `source` | `path`, `byte_start`, `byte_end` | Exact canonical source path and half-open UTF-8 byte range |

Source offsets are unsigned 32-bit integers. Zero-width ranges and EOF are valid.
The issuing immutable `SourceMap` must resolve every producer span; a span from any
other map is rejected even if its text and paths match. Decoding looks up the source
path, checks its exact stored spelling (including case), and calls `SourceMap::span`
to prove ordered endpoints, bounds and UTF-8 character boundaries. Portable source
path rules and limits belong to `zryna-source`; this protocol does not normalize paths,
text, line endings or Unicode. It transmits no snapshot-local numeric file IDs.

A workspace label may describe an invalid path that caused architecture rejection;
it is deliberately not normalized or treated as a source file. It can be empty or
contain controls. Consumers must never resolve it as a URI, open it automatically,
or interpret message/guidance/label text as HTML, terminal escapes or commands.

`validate_json` only validates transport against the caller-supplied snapshot. It
returns no compiler `Diagnostic`, `Span`, type, semantic result or permission to compile.
The enclosing request must bind responses to the exact compilation/source revision;
the document alone cannot authenticate the producer or detect a stale same-length
source with otherwise valid offsets. Consumers must discard stale results before
displaying them and must not use this validator to authorize compiler input.

## Deterministic ordering and encoding

Sort ascending by `(path, byte_start, byte_end, severity, code, message, guidance)`.
Absent path/offsets sort before present values. Thus global records sort first;
workspace labels sort before source records at the same path. Offsets compare
numerically, severity is `error` before `warning`, and strings compare by unsigned
UTF-8 bytes (not locale, UTF-16 code units, case folding or Unicode normalization).
Equal records are retained, including duplicates. Permuting a complete input multiset
does not change emitted bytes. A decoder rejects unordered records instead of repairing
them. There are no display-coordinate tie breakers because coordinates are not on the wire.

The producer emits compact JSON in the declared field order, no BOM or trailing newline.
Strings use the pinned `serde_json` escaping: quotes/backslashes and control characters
are escaped, other Unicode remains UTF-8. Decoders accept JSON whitespace and field
reordering, but reject duplicate keys, unknown/missing fields, nulls, unknown variants,
invalid UTF-8, lone surrogates, truncated JSON and trailing documents. Canonical producer
bytes are covered by the [golden fixture](../../crates/zryna-diagnostics/src/protocol_v2/fixtures/golden.json).
The fixture file's final LF is storage formatting, outside the rendered document.

## Inclusive budgets and terminal exhaustion

| Resource | Inclusive limit |
| --- | ---: |
| Complete encoded input/output document, including input whitespace | 65,536 UTF-8 bytes |
| Diagnostic records | 256 |
| Each decoded message and guidance | 4,096 UTF-8 bytes |
| Each decoded source path or workspace label | 1,024 UTF-8 bytes |
| Code | Exact 11-byte grammar above |

JSON Schema `maxLength` counts Unicode scalars, not UTF-8 bytes. Its limits are
necessary shape bounds; Rust enforces the stricter byte limits. The byte bound applies
before parsing or materializing wire DTOs. No allocation proportional to unbounded
wire input occurs. Producer inputs already belong to the caller; count and decoded
string bounds are checked before cloning or resolving diagnostics, and encoded size
is checked after bounded serialization (including JSON escape expansion).

On any producer count, text/path byte or encoded document exhaustion, discard **all**
ordinary records and emit exactly the [terminal fixture](../../crates/zryna-diagnostics/src/protocol_v2/fixtures/exhausted.json):
one global error `ZRYNA-D2001`, message `structured diagnostic limit exceeded`, guidance
`reduce the diagnostic input and retry; this report is incomplete`. There is no partial
prefix, dropped-count estimate, clipping or second terminal. The terminal fits every
budget and is permutation independent. Its presence means incomplete diagnostics,
never successful compilation; even an empty ordinary report is not a compilation-success
or profile-support claim. Earlier phase-specific exhaustion codes remain owned by
their phases and are carried unchanged as ordinary records.

The producer reserves `ZRYNA-D2001` and rejects callers attempting to supply it.
A decoder accepts it only as that exact singleton document; it never repairs or
truncates an oversized/malformed report. After any rejection or exhaustion, a valid
subsequent request produces the same bytes as a fresh invocation; there is no state.

Producer precedence is count/text/path limits, then input-order code/source validation,
then sorting and encoded bytes. A count/string exhaustion deliberately prevents later
resolution and emits only the locationless terminal. Invalid codes and mismatched
spans return an error with no output. For several invalid producer records, the caller's
input order selects code/source failure; permutation invariance covers valid inputs and
resource exhaustion, not arbitrary mixtures of unrelated invalid producer records.

Decoder precedence is encoded bytes (`Limit`), serde JSON shape (`Shape`), version
(`Version`), decoded collection/string bounds (`Limit`), code/terminal validity (`Record`),
source binding (`Source`), then ordering (`Order`). Parsing failures share one stable
category; parser-specific prose is not protocol output. Unsupported versions are never
silently downgraded, and limits include the exact boundary with the first extra rejected.

## Compatibility and consumer rules

V1 remains available unchanged; it is not reinterpreted as bounded v2. V2's explicit
version is independent of syntax protocol v2/v3/v4, compiler version, manifest version,
and language profile. Adding fields, variants or severities, changing location meaning,
ordering, budget, terminal shape or serialization requires a new transport version.
Consumers must negotiate/select a supported version explicitly and fail closed on others.

Compiler phases retain diagnostic-code and source authority. New ordinary codes matching
the grammar may be displayed without a transport revision; consumers must tolerate
unrecognized ordinary codes and show the original severity/message/guidance. Message prose
is not a stable identifier and may change. A code must not be silently repurposed to
mean another failure. Tooling never duplicates name resolution, type/ownership checking,
ABI, architecture, or support-profile decisions to reinterpret compiler diagnostics.

| Consumer | Required adaptation; implementation remains out of scope |
| --- | --- |
| Formatter | Preserve compiler diagnostics; do not infer semantic validity or apply guidance as edits |
| Language server | Bind request/document revision; convert byte ranges against that exact text to negotiated positions |
| Editor | Display inert text, preserve code/severity, distinguish source ranges from labels, expose incomplete-report state |
| Playground | Bind results to the submitted source revision; render inert text and never grant workspace/host authority |

Display coordinates are deliberately derived from the retained source snapshot, not
accepted as independent claims. CRLF, CR, LF, multibyte scalars and EOF must be handled
by the source authority; a client requiring UTF-16 positions converts against the same
text instead of treating byte offsets as character positions. V1's one-based Unicode-scalar
display fields are preserved only in v1.

## Verification

`pnpm diagnostics:contract` runs schema tests; `cargo test --locked -p zryna-diagnostics`
runs source-bound producer/decoder tests and the unchanged v1/text goldens. The shared
[hostile fixture](../../crates/zryna-diagnostics/src/protocol_v2/fixtures/hostile.json) declares whether rejection
belongs to schema shape or Rust authority, using independent mutations of the golden.
Rust also tests duplicate keys, malformed bytes, string/count/document exact and first-extra
limits, escape expansion, Unicode byte ordering, terminal forgery and deterministic recovery.

Run `pnpm docs:check`, `pnpm structure:check`, `pnpm preflight` and `pnpm m0:check` with
the repository-pinned toolchains before submission. The additive diagnostic workflow
runs schema and focused Rust checks on Linux and Windows; existing required CI gates
and timeout policy remain unchanged and must pass independently before integration.
