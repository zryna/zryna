# M0 conformance and closure gate

M0 closes only through the repository-owned `pnpm m0:check` command. The command reads the frozen
[`tests/m0-conformance-v1.json`](../tests/m0-conformance-v1.json) registry, validates it before
starting subprocesses, and then runs every registered command directly without a shell. It has no
mode that omits the architecture check or selects only a favorable subset.

The same registry is an exact inventory of every external file under `tests/fixtures`. Each fixture
has a frozen phase, pass/fail mode, and expected outcome. Registry self-tests prove that an unlisted
addition, listed deletion, duplicate entry, changed expectation, unknown phase, or unknown field
fails the gate. Component-owned inline adversarial cases remain in their Rust or adapter suites;
the registry maps those authoritative suites without reimplementing their validators.

Run it from a dependency-installed checkout with Rust 1.97.1, Node.js 22.22.1, and pnpm 11.18.0:

```bash
pnpm install --frozen-lockfile
pnpm m0:check
```

## Registered proof suites

The registry maps each foundation boundary to its component-owned tests. It does not move language
semantics or validation decisions into the runner.

| Boundary | Proof owner | Required command |
| --- | --- | --- |
| Workspace scan, manifest, and complete Cargo graph | `zryna-architecture` | Rust workspace tests |
| Portable source identity and authoritative spans | `zryna-source` | Rust workspace tests |
| Stable, source-bound diagnostics | `zryna-diagnostics` | Rust workspace tests |
| Frontend handshake and bounded worker process | `zryna-frontend` | Rust workspace tests |
| Executable syntax wire and graph verifier | `zryna-syntax` | Rust workspace tests |
| Provider-error semantic stop gate | `zryna-semantics` | Rust workspace tests |
| Sealed Universal IR verification | `zryna-ir` | Rust workspace tests |
| Verified-only JavaScript emission | `zryna-backend-javascript` | Rust workspace tests |
| Sealed native MIR verification | `zryna-native-mir` | Rust workspace tests |
| Verified-only native emission and raw-input compile failure | `zryna-backend-native` | Rust workspace tests and doc-tests |
| Independent backend orchestration | `zryna-driver` | Rust workspace tests |
| TypeScript bootstrap adapter | `typescript-6` | adapter check and tests |
| Shared protocol-v2 schema and fixtures | root `tests/fixtures` | protocol check and tests |

The gate also requires locked dependency fetching, formatting, strict Clippy, warning-free rustdoc,
and registry self-tests. GitHub runs the same complete gate in the required
`rust (ubuntu-latest)` and `rust (windows-latest)` checks. The existing adapter matrix remains an
additional platform proof. Pull requests must be current with `main`; force pushes and branch
deletion are disabled, administrators are subject to the rules, and conversations must be
resolved.

## Closure checklist

- All dependency-ordered M0 issues through verified native MIR are merged.
- Scanner, manifest, dependency graph, source, diagnostic, protocol, adapter, IR, MIR, and backend
  adversarial tests are registered and run by the canonical gate.
- Linux and Windows run the same complete gate.
- Raw protocol, IR, and MIR claims cannot enter their downstream trusted consumers.
- The checked-in status text distinguishes implemented proof boundaries from planned execution.
- Independent closure review has no unresolved P0 or P1 M0 finding.

## Deliberately unsupported after M0

M0 is an architecture foundation, not an executable compiler release. Zryna-owned source semantic
lowering is not implemented. There is no WebAssembly backend. Native output is textual LLVM IR,
not an object or linked executable. The current verified Universal IR and native MIR profiles admit
only the documented `i32` proof slice; scalar ABI v1, `bool`, host invocation, and concrete target
symbol mapping begin in M1. No website, package, or release status may claim otherwise.
