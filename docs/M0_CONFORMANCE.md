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

Protocol-v3 schema, adapter, and M2 semantic request/result fixtures are appended to that exact
inventory; they do not replace or relax any frozen protocol-v2 record. The real v3 adapter test
replays each M2 request and requires its exact checked-in result. The root `protocol:check` and
`protocol:test` commands run the exact v2 and v3 suites, and the existing Linux/Windows adapter CI
matrix invokes both commands.

Run it from a dependency-installed checkout with Rust 1.97.1, Node.js 22.22.1, and pnpm 11.18.0:

```bash
pnpm install --frozen-lockfile
pnpm m2:quick
pnpm preflight
pnpm m0:check
```

Use `pnpm m2:quick` after an M2 JavaScript, WebAssembly, semantics, closure, or workspace-source
edit. It runs
only the deterministic M2 JavaScript and WebAssembly backends, control-flow semantic, closure, and
retained-filesystem security suites and therefore avoids unrelated runtime/toolchain integration
failures while preserving native operating-system behavior.

Use `pnpm preflight` during the edit loop. Its fixed order runs portable documentation, protocol,
adapter and formatting checks, then the complete M2 semantics and driver libraries before the
broader workspace, frontend, and syntax checks. It stops immediately at the first failure. GitHub runs the same
command before starting either full platform matrix, so a
preflight failure skips the expensive Linux and Windows jobs. This short gate does not claim full
conformance: `pnpm m0:check` on both supported operating systems and the final `m0` aggregate are
still mandatory before merge. Warm-run timings are observational only and are never pass/fail
criteria.

For fast triage of JavaScript build/run behavior, run the focused CLI test:

```bash
cargo test --locked -p zryna --test cli javascript_build_and_run_publish_exact_bundles -- --exact
```

Run it in native Windows when investigating Windows process or path behavior; WSL cannot reproduce
those operating-system boundaries. CI runs this smoke test early on Windows so a regression is
reported before the complete conformance command. The smoke test is diagnostic only and does not
replace `pnpm m0:check`.

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
| Provider-error stop gate and control-flow M2 semantics | `zryna-semantics` | Rust workspace tests |
| Sealed Universal IR verification | `zryna-ir` | Rust workspace tests |
| Verified-only M1 and deterministic sealed M2 JavaScript emission | `zryna-backend-javascript` | focused Rust and Node tests |
| Sealed native MIR verification | `zryna-native-mir` | Rust workspace tests |
| Verified-only native emission and raw-input compile failure | `zryna-backend-native` | Rust workspace tests and doc-tests |
| Independent backend orchestration | `zryna-driver` | Rust workspace tests |
| TypeScript bootstrap adapter and exact M2 fixture replay | `typescript-6` | adapter check and tests |
| Shared protocol-v2 and protocol-v3 schemas and fixtures | root `tests/fixtures` | version-specific protocol checks and tests |

The gate also requires locked dependency fetching, formatting, strict Clippy, warning-free rustdoc,
and registry self-tests. GitHub first requires the portable preflight, then runs the same complete gate in the required
`rust (ubuntu-latest)` and `rust (windows-latest)` checks. The existing adapter matrix remains an
additional platform proof, and the stable `m0` aggregate is itself a required `main` check. Pull
requests must be current with `main`; force pushes and branch deletion are disabled,
administrators are subject to the rules, and conversations must be resolved.

## Closure checklist

- All dependency-ordered M0 issues through verified native MIR are merged.
- Scanner, manifest, dependency graph, source, diagnostic, protocol, adapter, IR, MIR, and backend
  adversarial tests are registered and run by the canonical gate.
- Linux and Windows run the same complete gate.
- Raw protocol, IR, and MIR claims cannot enter their downstream trusted consumers.
- The checked-in status text distinguishes implemented proof boundaries from planned execution.
- Independent closure review has no unresolved P0 or P1 M0 finding.

## Closure evidence

- [M0 closure pull request](https://github.com/zryna/zryna/pull/31) records the reviewed change and
  required Linux, Windows, adapter, and aggregate results.
- `main` requires `rust (ubuntu-latest)`, `rust (windows-latest)`, `adapter`, and `m0` with strict
  branch currency.
- The deployed [compiler status](https://zryna.com/reference/compiler-status/) describes the same
  implemented proof boundaries and deliberately unsupported surface.

## Deliberately unsupported at M0 closure

M0 was an architecture foundation, not an executable compiler release. At its closure,
Zryna-owned source semantic lowering and scalar ABI v1 were not implemented. There was no
WebAssembly backend, native output was textual LLVM IR rather than an object or linked executable,
and the verified Universal IR and native MIR profiles admitted only the documented `i32` proof
slice. Subsequent M1 issues may extend those boundaries without changing this historical M0 gate;
the root README, roadmap, and public compiler status are authoritative for the current surface.
