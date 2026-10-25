# Representable rejected generic catalogue source witnesses

These 19 rejected catalogue fragments produced raw protocol-v5 candidates from both real producers at PR570 `e639c525c0b506a5606429a9f99c59ad3a9b7a5c` (tree `23824c5ad08a48043cfb8da38571134dff5112a3`). `manifest.json` freezes exact wrapper text, catalogue-to-source byte mappings, original and rebased primary spans, and source/payload SHA256. Each `.json` file preserves the exact result-value bytes from its producer response; the two producers have equal DTO structure, without requiring equal object-key serialization order.

The expected code and span come from the accepted catalogue before compiler API execution. Provider error envelopes are excluded. The integration test must first verify the complete source/DTO pair, then use only public declaration, body, instantiation, original ownership, layout and executable verification APIs. Earlier phase failures are reported as observed, never rewritten into the catalogue code. No source, raw DTO, or oracle may be repaired to fit current behavior.

The other 32 producer-rejected catalogue cases require a separate source-rejection authority design and are deliberately absent. The 19 admitted cases belong to positive source conformance.


The `generic_diagnostics` integration suite includes 18 producer-backed catalogue cases and the independently authored duplicate-arm case. The unrooted expanding-declaration case requires a rejection-only semantic definition proof at its fixed original application span; it must not gain an invented closed instance. The unchanged by-value-cycle fixture still requires a separate layout-owned symbolic-definition authority. Full catalogue acceptance and issue #416 remain open.
