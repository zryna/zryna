# Security policy

Please report suspected vulnerabilities privately through the repository security advisory feature. Do not open a public issue for an unpatched vulnerability.

Security-sensitive boundaries include workspace containment, symlink handling, adapter message
validation, generated output paths, WebAssembly feature/capability validation, native ABI layouts,
linker invocation, and unsafe Rust. Core WebAssembly artifacts are sealed only after validation
with explicitly pinned WebAssembly 1.0 features and a fail-closed import/section/operator audit.
Native object requests accept one exact target capability. Pinned library codegen output is
independently parsed and bounded before sealing; the audit requires ELF64 x86-64 relocatable
identity, an exact section allowlist, sealed ABI symbols, no undefined symbols, and no relocations
for the current leaf profile. Only sealed objects reach the revalidated create-only publisher.
The driver-owned Linux executable boundary admits only a separately discovered capability for
canonical `/usr/bin/gcc`, its exact GNU x86-64 target, a bounded supported compiler version, and its
canonical bounded-version GNU linker. It ignores ambient compiler and linker selection, invokes no
shell, clears the child environment, rechecks tool identity, stages only known create-new files in
a private directory, applies fixed hardening flags, and fail-closed audits the ELF executable
before create-only publication. The opaque capability retains the audited executable bytes and
executes a fresh private copy, so later replacement of the public path cannot substitute code.
Execution uses no stdin, limits time and output, terminates and confirms disappearance of the
process group, and requires an exact four-byte result channel. This is defense in depth, not a
sandbox for hostile native code or a hostile installed system toolchain.

Public build and run requests add a bundle transaction boundary. Target names are closed enums;
entrypoints are one portable workspace-relative `.zry` path; artifact names use the portable stem
grammar; and there is no caller-controlled output directory. Arguments and executable paths are
passed literally through direct argument-vector process creation, never through a shell. The CLI
accepts no user linker flags, loaders, scripts, environment entries, or arbitrary command strings.
The Node executable must be an absolute direct regular file without a link or Windows reparse
point and must validate as exact Node.js 22.22.1. Target-runtime processes have a five-second hard
deadline, bounded output, no stdin, cleared environments with only documented platform essentials,
and a separate bounded process-tree cleanup deadline.

All selected artifacts and the deterministic manifest are created under one sibling transaction
directory, synchronized, containment-revalidated, and exposed with one create-only same-filesystem
directory rename. Unix applies mode `0700` to transaction directories. Windows inherits ACLs from
the validated compiler-owned output root, so the workspace and `.zryna/out` must already be private
to the invoking principal. Existing bundles are not overwritten; failures leave no final bundle.
Cleanup that cannot be confirmed fails separately. Manifests omit absolute and temporary paths,
timestamps, process identifiers, inherited environment values, credentials, and raw external-tool
output. These protections do not make generated JavaScript, WebAssembly, or native code a sandbox
for hostile source or a concurrently hostile workspace ancestor.

The current workspace forbids unsafe Rust globally; any future exception requires an isolated
approved component, a documented safety invariant, and dedicated tests.
