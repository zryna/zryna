# Zryna diagnostics

`zryna-diagnostics` owns stable diagnostics shared by every compiler phase. Its dependency on
`zryna-source` is intentional: a source diagnostic carries one authoritative primary `Span`, not
an independently supplied path and range. Repository architecture failures retain a mutually
exclusive workspace-path location, and truly locationless failures use a global location.

Source diagnostics must be rendered with the matching `SourceMap`. Rendering fails closed for an
unknown file or invalid range, sorts by resolved location and content, and emits deterministic LF
text or compact versioned JSON. The structured contract includes the normalized path, half-open
UTF-8 byte range, and one-based Unicode-scalar line/column display coordinates.

The opt-in `protocol_v2::{render_json, validate_json}` library boundary adds a closed,
bounded tooling transport with deterministic terminal exhaustion. See the normative
[v2 contract](../../spec/diagnostics/STRUCTURED_DIAGNOSTICS_V2.md) and
[schema](../../schemas/zryna-diagnostics-v2.schema.json). Existing text and JSON v1 stay
unchanged. The separate `zryna-language-server` application carries exact v2 reports inside
revision-bound notifications; this component still owns their codes, shape, ordering and limits.

Run `cargo test --locked -p zryna-diagnostics` and `pnpm diagnostics:contract` for focused
source-binding, malformed-input, golden, ordering and exact/first-extra limit evidence.
