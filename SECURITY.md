# Security policy

Please report suspected vulnerabilities privately through the repository security advisory feature. Do not open a public issue for an unpatched vulnerability.

Security-sensitive boundaries include workspace containment, symlink handling, adapter message
validation, generated output paths, WebAssembly feature/capability validation, native ABI layouts,
linker invocation, and unsafe Rust. Core WebAssembly artifacts are sealed only after validation
with explicitly pinned WebAssembly 1.0 features and a fail-closed import/section/operator audit.
The current workspace forbids unsafe Rust globally; any future exception requires an isolated
approved component, a documented safety invariant, and dedicated tests.
