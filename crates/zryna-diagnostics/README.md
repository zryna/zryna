# Zryna diagnostics

`zryna-diagnostics` owns stable diagnostics shared by every compiler phase. Its dependency on
`zryna-source` is intentional: a source diagnostic carries one authoritative primary `Span`, not
an independently supplied path and range. Repository architecture failures retain a mutually
exclusive workspace-path location, and truly locationless failures use a global location.

Source diagnostics must be rendered with the matching `SourceMap`. Rendering fails closed for an
unknown file or invalid range, sorts by resolved location and content, and emits deterministic LF
text or compact versioned JSON. The structured contract includes the normalized path, half-open
UTF-8 byte range, and one-based Unicode-scalar line/column display coordinates.
