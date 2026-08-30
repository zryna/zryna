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

## WebAssembly profiles

The implemented `I32V1` scalar subset maps directly to core WebAssembly without passing through
JavaScript, native MIR, or LLVM. It exports capability-minimal pure `i32` functions and has no
imports or implicit filesystem, network, clock, randomness, environment, heap, or
garbage-collection facility, and it carries no bundled Zryna runtime. The Node conformance harness
uses the standard browser-compatible WebAssembly API, but browser execution and a generated loader
remain untested future work, as does a strict typed host wrapper.

WASI and Component Model support is a separately versioned host profile. WIT and Canonical ABI types describe component boundaries; they do not redefine Zryna's internal ownership or memory model.

## Native profile

The native profile will extend the universal profile with features that have no JavaScript representation, such as raw FFI. Such modules cannot be emitted as JavaScript and must be separated by an explicit target boundary.
