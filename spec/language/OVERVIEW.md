# Zryna language profile v1

Zryna v1 begins with TypeScript-compatible declaration and expression syntax so the bootstrap frontend can parse it. Compatibility of syntax does not imply compatibility of semantics.

## Universal profile

The universal profile must compile to JavaScript, WebAssembly, and native output with specified matching observable behavior.

- `any` and implicit `any` are errors.
- `unknown` requires explicit narrowing.
- public function parameters and results require declared types.
- exact numeric types are intrinsic Zryna types.
- dynamic property creation, `eval`, `Proxy`, prototype mutation, and sparse arrays are unavailable.
- unsupported syntax is rejected before IR construction.

The first intrinsic types are `unit`, `bool`, and `i32`. Additional types enter only with complete source, IR, JavaScript, WebAssembly, native, conversion, boundary, and diagnostic specifications.

The implemented explicit M2 [`ControlFlowV1`](CONTROL_FLOW_MODULES_V1.md) profile freezes exact wrapping scalar
arithmetic without a language trap surface, Boolean comparisons, typed lexical locals and
assignment, direct nonrecursive calls, `if`, `while`, compiler-owned relative modules, structured
verified IR, and resource budgets. It is selected only by exact `--profile control-flow-v1` and is
covered by the fixed-oracle three-target M2 gate. Omitting `--profile` continues to select the M1
`I32V1` slice.

## Cross-target composition

The specified-only [cross-target profile composition v1](CROSS_TARGET_PROFILES_V1.md) separates
language admission, output target and deployment-host permissions. It defines transitive
dependency requirements and explicit denial policies without adding a public selector or
package/host implementation. Backend choice and native metadata do not grant host authority
or establish execution isolation. Its WIT mapping depends on Issue #167's pinned-world contract.

## WebAssembly profiles

The implemented `I32V1` scalar subset maps directly to core WebAssembly without passing through
JavaScript, native MIR, or LLVM. It exports capability-minimal pure `i32` functions and has no
imports or implicit filesystem, network, clock, randomness, environment, heap, or
garbage-collection facility, and it carries no bundled Zryna runtime. The Node conformance harness
and public `run` command use the standard browser-compatible WebAssembly API, but browser execution
and a generated loader remain untested future work, as does a general strict typed host wrapper.
The CLI validates its `I32V1` invocation before making the raw host call.

The specified-only M4 [WIT capability contract](../wit/CAPABILITY_PROFILES_V1.md) pins separate
browser, command, and server world identities plus a WASI version and denied-by-default host
policy. It does not activate WASI or Component Model support. WIT and Canonical ABI types describe
component boundaries; they do not redefine Zryna's internal ownership or memory model.

The specified-only [typed JS/WASM adapter design](../interop/JS_WASM_ADAPTERS_V1.md) separately
defines ESM carriers, component conversion and resource lifetime rules. Public String/list/resource
bridges, library API alignment, generation, conformance and activation remain separately gated;
internal M3 layouts and borrows are not a public interface. No browser, Node package or WASI
support is added by those declarations.

## Native profile

The universal `I32V1` slice emits audited Linux x86-64 ELF relocatable objects using the System V
ABI. The driver can link one ABI-validated invocation into an audited executable and observe its
full-width typed `i32` result. The public CLI composes that boundary only for an explicit verified
invocation; it is not a general native runtime. Boolean source/IR remains gated. A later native
profile may extend the language with
features that have no JavaScript representation, such as raw FFI; such modules must be separated
by an explicit target boundary.
