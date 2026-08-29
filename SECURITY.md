# Security policy

Please report suspected vulnerabilities privately through the repository security advisory feature. Do not open a public issue for an unpatched vulnerability.

Security-sensitive boundaries include workspace containment, symlink handling, adapter message validation, generated output paths, native ABI layouts, linker invocation, and unsafe Rust. The current workspace forbids unsafe Rust globally; any future exception requires an isolated approved component, a documented safety invariant, and dedicated tests.
