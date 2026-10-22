# Private operator archive preparation

These Python 3.11+ helpers compose and emit finite input archives. The archive helpers never
call Docker. The separate `engine` and `install` modules perform explicitly selected private
operator preparation; they never execute source material, grant image or volume approval,
or produce a guest result receipt.
Run them only as a trusted operator. The source paths, their provenance, the complete plan,
and the exact helper sources belong to the independently reviewed preparation inputs.

`compose.candidate(kind, role, files)` accepts explicit descriptors with exactly `path`,
`sourcePath`, `bytes`, `sha256`, and `mode`. `compose.descriptor` verifies an independently
selected size and SHA256; it never selects a tool or searches for source files.
`compose.material_manifest` emits the compact canonical image-pinned material manifest.
`compose.encode` returns candidate plan bytes and their size/hash. Those calculated hashes
become authority only when independently reviewed and supplied by the operator.

The closed plan records format, version, kind, role, exact directory closure, sorted unique
files, aggregate bytes, and the fixed image config (or null for materials). Limits cover
raw plan bytes, path grammar/depth, directories, files, per-file bytes, aggregate bytes,
role-specific destinations and modes. Runtime plans require the complete fixed role
inventory; an evaluator without its independently composed evaluation identity is rejected.
Materials retain their separate compiler/evaluator inventories and admission limits.

Emit a reviewed plan with explicit external pins:

```text
python3 examples/playground/restricted/daemon/preparation/emit.py \
  /absolute/reviewed-plan.json --plan-bytes DECIMAL_BYTES --plan-sha256 SHA256
```

Stdout is an ordinary-file USTAR stream. Stderr receives one bounded success summary only
after all checks and source-handle cleanup complete. An error exits nonzero. The caller must
require both emitter exit zero and downstream consumer exit zero before retaining an
installed object for review. A failed stream may already contain a complete-looking archive;
an importer accepting that stream does not establish success or approval. Discard and clean
only the exact newly owned object on any failure. Retain the complete command, helper hashes,
external plan pins, stdout archive hash, stderr and actual process exit statuses.

The emitter reads the raw plan only after validating its finite externally selected size.
It rejects duplicate/unknown fields, validates every metadata record and USTAR header, and
hashes every source before writing the first archive byte. It opens every path component
without following links and retains bounded ancestor descriptors. Around every source use,
it checks the original named ancestor links as well as the held descriptors, so moving a
retained ancestor does not silently redirect access. Each file is reopened with the original
identity, streamed through bounded buffers, hashed, checked for the first extra byte, and
checked again after the archive completes. Source modes are observations; archive metadata
is explicitly UID/GID zero, mtime zero, directories 0555, and role-selected file modes.

All regular aliases, including the dynamic loader, are separate ordinary archive files.
There are no links, devices, arbitrary archive metadata, inherited image environment,
implicit source roots, host library search, Docker calls, installation, or deletion in the emitter.
The actual runtime authority, writable-alias removal, volume nonce and creation identity,
read-only mounts, per-operation full audit, kernel controls and deadlines remain separate
requirements of the reviewed provisioning and execution owners.

`install.populate_materials` accepts externally pinned material plans and runtime image IDs.
It creates a fresh labeled local volume and one fixed nonroot container that is never started.
Only this preparation mount uses `NoCopy:false`: the daemon copies the approved image's empty
`/materials` directory metadata into the empty volume. Before uploading any material bytes,
the helper checks the created/PID-zero container configuration and reads the complete bounded
archive of that directory. It requires exactly one empty directory, UID/GID zero and mode
0555, retains the original 1536 archive bytes as base64 with their SHA256, then rechecks the
never-started holder. Extra members, links, extended headers, wrong metadata or excess bytes
fail before upload. Guest runtime mounts continue to require `NoCopy:true` and read-only access.

After the selected material archive is uploaded, the holder must still be never started. The
helper removes that exact holder without force, confirms its absence and rereads the volume
identity. Every returned record remains unapproved. A failure retains the exact object IDs
for operator recovery; the helper never force-removes an uncertain object or deletes a volume.
The independently reviewed final metadata, full per-operation material audit and absence of
writable aliases are still required before any guest admission.

The separate `ui_plan` and `ui_image` modules describe and import only the private trusted
UI qualification image. Their closed schema fixes `/ui-supervisor` and the finite UI driver
inventory; production runtime plans do not accept this role. Import observations remain
unapproved and never start a container. Runtime admission must separately verify the image,
read-only evaluator volume, complete fixed material audit before the start capability, and
the reviewed UI controls. This trusted test role has a 300-second lifetime and two-second
cleanup bound; production compiler/evaluator deadlines remain unchanged.

Run the filesystem and archive failure checks from this directory:

```text
python3 -m unittest -v test_preparation.py
```

These tests cover exact archive metadata and bytes, external plan pin failures, oversized
plans, full-source preflight, symbolic-link parents, retained ancestor moves, content changes,
live mutation during streaming, and closed metadata/path/count limits. They launch no guest.
