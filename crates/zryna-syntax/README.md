# Zryna syntax

Owns the versioned, provider-neutral syntax contract below every replaceable frontend.

Protocol v2 represents executable syntax as bounded flat arenas. Raw wire values remain untrusted;
only source-map-backed verification can construct the opaque verified project consumed by Zryna
semantics. This crate does not parse TypeScript, resolve names, assign semantic types, or construct
IR.
