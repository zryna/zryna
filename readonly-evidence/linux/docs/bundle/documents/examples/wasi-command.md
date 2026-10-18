# Bounded H1 command examples

Follow [the command walkthrough](../../docs/WASI_COMMAND_GETTING_STARTED.md) for complete
checkout and fresh content-verified installation commands, pinned runtime requirements and
Linux/Windows private-file creation. `pure.zry` returns WIT `ok(())`; `error.zry` returns
`err(())`; `lookup.zry` has one literal `MODE` environment lookup and requires a matching
explicit caller-private request. Pure source rejects a grant file. No ambient environment
value is read and no additional capability is granted.

These examples map to the two command modes in [the M4 matrix](../../docs/M4_SUPPORT.md).
Existing immutable beta archives remain excluded. Content verification does not establish a
signed package release or production admission. A runnable example and a successful process
exit do not authenticate complete both-host M4 evidence or live publication.
