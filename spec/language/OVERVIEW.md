# UTS language profile v1

UTS v1 begins with TypeScript-compatible declaration and expression syntax so the bootstrap frontend can parse it. Compatibility of syntax does not imply compatibility of semantics.

## Universal profile

The universal profile must compile to JavaScript and native output with specified matching observable behavior.

- `any` and implicit `any` are errors.
- `unknown` requires explicit narrowing.
- public function parameters and results require declared types.
- exact numeric types are intrinsic UTS types.
- dynamic property creation, `eval`, `Proxy`, prototype mutation, and sparse arrays are unavailable.
- unsupported syntax is rejected before IR construction.

The first intrinsic types are `unit`, `bool`, and `i32`. Additional types enter only with complete source, IR, JavaScript, native, conversion, boundary, and diagnostic specifications.

## Native profile

The native profile will extend the universal profile with features that have no JavaScript representation, such as raw FFI. Such modules cannot be emitted as JavaScript and must be separated by an explicit target boundary.
