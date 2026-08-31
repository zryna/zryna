# M1 three-target conformance

Status: implemented for the checked `I32V1` slice. M1 remains open until the compiler-owned status
bundle and zryna.com synchronization in Issue #21 are verified.

The versioned [`m1-conformance-v1.json`](../tests/m1-conformance-v1.json) registry freezes one
source entrypoint, one logical export, canonical target order, exact typed arguments, and exact
typed results. The suite invokes the public CLI and compares structured outcomes and committed
manifests; it does not reproduce backend implementation logic or use one target as the oracle.

Install the locked JavaScript dependencies first, then run the focused suite:

```bash
pnpm install --frozen-lockfile
pnpm m1:check
```

The complete required repository gate remains `pnpm m0:check`. Its locked workspace tests execute
the same M1 cases on GitHub's Linux and Windows runners. The focused command is for local M1
triage; it does not replace the complete gate.

## Evidence

| Behavior | Frozen case or authority | Executable proof | Platform claim |
| --- | --- | --- | --- |
| `1 + 2` returns `3` | `one-plus-two` | `m1_linux_three_target_results_and_artifacts_match_every_fixture` | JavaScript, core WebAssembly, Linux x86-64 native |
| `i32::MAX + 1` returns `i32::MIN` | `maximum-plus-one-wraps` | same three-target test | JavaScript, core WebAssembly, Linux x86-64 native |
| `i32::MIN - 1` returns `i32::MAX` | `minimum-minus-one-wraps` | same three-target test | JavaScript, core WebAssembly, Linux x86-64 native |
| Portable result behavior | all three numeric cases | `m1_javascript_and_webassembly_match_every_portable_fixture` | JavaScript and core WebAssembly on Linux and Windows |
| Target-independent source rejection | `invalidSource` / `ZRYNA-M1004` | `m1_invalid_source_is_target_independent_and_publishes_nothing` | Linux and Windows |
| Typed Boolean host normalization | [`scalar-v1-fixtures.json`](../spec/abi/scalar-v1-fixtures.json) | `m1_bool_host_normalization_matches_without_enabling_bool_source` plus component-owned ABI tests | ABI carrier contract only |
| Boolean source remains closed | `gatedBooleanSource` / `ZRYNA-I1006` | same Boolean gate test | Every target selection; no artifact |
| Unsupported native execution closes without output | `ZRYNA-N4002` | `m1_windows_native_and_all_runs_are_rejected_without_a_bundle` | Windows portability, not native execution |

For every numeric case the Linux test requires exactly three results in JavaScript, WebAssembly,
native order. Each result must equal the fixed registry value and the other two typed outcomes.
The committed manifest must independently contain the same results, and the final bundle must
contain exactly one `.mjs`, one `.wasm`, one `.elf`, and its manifest. Three equally wrong targets
therefore cannot pass.

The invalid-source test uses the same source identity for `javascript`, `webassembly`, `native`,
and `all`. It requires byte-equivalent structured diagnostics, source exit status `3`, a null
manifest, empty results, and no final bundle for every selection. This proves rejection occurs
before backend selection can change behavior or publish an artifact.

## Boolean boundary

Scalar ABI v1 specifies Boolean carriers, but M1 does not enable Boolean source execution. The
conformance proof normalizes primitive JavaScript `false`/`true` and core-WebAssembly/native `i32`
lanes `0`/`1` to the same typed `ScalarValue::Bool`. JavaScript numeric truthiness and noncanonical
WebAssembly/native lanes are rejected. The checked Boolean source still fails at the `I32V1`
verifier and publishes nothing.

This is not a claim of a public native Boolean wrapper, Boolean IR/backend support, Windows native
execution, macOS native execution, control flow, modules, heap values, WASI, or browser execution.

## CI platform split

- Linux x86-64 GNU runs the full JavaScript/WebAssembly/native differential matrix and audits the
  exact artifact inventory.
- Linux and Windows both run the JavaScript/WebAssembly matrix, invalid-source matrix, Boolean ABI
  normalization, registry checks, and full repository portability suite.
- Windows verifies that native and `all` execution fail closed with `ZRYNA-N4002` and no bundle.

The public `zryna run --target all` command reports the ordered typed observations. Equivalence is
enforced as repository conformance evidence rather than as a second runtime semantics authority.
