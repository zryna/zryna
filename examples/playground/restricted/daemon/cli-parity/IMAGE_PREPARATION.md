# Private CLI image preparation

This operator-only producer has its own closed plan domain and validator. It does
not change production preparation roles, modes, compiler policies or guest limits.
The image contains all 349 original release files with original 0644/0755 modes,
eight pinned DSOs, two pinned copyright files and the dedicated CLI-only idle supervisor with fixed nofile1024.
The complete 360-file image has 160,488,297 file bytes.

`cli_image_plan.py` requires the exact original archive pin, complete file counts,
original modes, fixed runtime pins, directory closure and public image config.
The caller externally selects exact plan bytes and SHA256. `cli_image_sources.py`
and `cli_image_archive.py` retain the existing no-follow source-handle and USTAR
primitives under separate imports. No source hashing or emission precedes exact
plan selection.

`cli_image_import.import_image(plan_path, plan_bytes, plan_sha256, record_path)`
reserves a create-only 0400 result before mutation. It materializes the fixed
archive into a newly created private inert 0400 cache, using flat generated names
through a creation-owned directory handle. It validates every original header,
mode, hash and complete topology before import. The original archive and every
emitted source are rechecked before and after emission.

The random tag must be absent before import. The imported image must have exactly
one RootFS layer whose DiffID equals the complete emitted USTAR SHA256. Image
identity, target, configuration and actual tag are read back. The separate local
preparation deadline is 180 seconds; runtime owners retain their 10+2-second
budget. A failed import retains its result, tag and cache for explicit recovery.
The producer never starts a container, CLI, provider or generated program.

Every returned record remains `unapproved-operator-image`. Independent review of
the current producer, externally selected plan, original authenticated release
proof and complete image readback must precede runtime approval. Current tests
use inert sources and Engine doubles; a full real emission/import is a separate
operator action and must not inherit a synthetic pass.
