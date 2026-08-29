# Zryna source model

`zryna-source` is the lowest compiler foundation and owns the authoritative source text,
portable paths, snapshot-local file identities, and half-open UTF-8 byte spans. It has no
dependency on diagnostics or a frontend provider.

`SourceMap::build` validates the complete bounded file set atomically. Paths use a strict
portable ASCII grammar with `/` separators; absolute paths, traversal, Windows-reserved names,
exact duplicates, and ASCII-case collisions fail. Dense `FileId` values are assigned after path
sorting, so input enumeration order cannot change them.

Source text is preserved byte-for-byte. `SourceMap::span` validates file identity, ordering,
file bounds, and UTF-8 character boundaries. `span_from_utf16` is the explicit adapter boundary
for JavaScript/TypeScript coordinates and rejects offsets inside surrogate pairs. Resolved lines
are zero-based internally; CRLF, LF, and lone CR are recognized without rewriting source bytes.

Providers deserialize only `UntrustedSpan`. Frontend verification converts it through the exact
`SourceMap` into an opaque `Span`; both `FileId` and `Span` retain the issuing map identity so an
equally numbered file from another map cannot be substituted. Semantic IR verification resolves
every span again before constructing a backend-accepted program.
