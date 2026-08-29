# Zryna semantics

Permanent compiler phase boundary from verified provider-neutral syntax to unverified Universal IR.

Issue #7 registers the boundary and its dependency direction. Name resolution, strict type checking,
unsupported-syntax rejection, and lowering are implemented by later focused issues; this crate must
never depend on a replaceable frontend provider. `SemanticInput::try_new` also rejects pairing a
verified snapshot with any source-map instance other than the one that issued its opaque file and
span identities. It also rejects every snapshot containing a provider error, so parse recovery or
unsupported syntax cannot enter name resolution, type checking, or lowering as a smaller program.
