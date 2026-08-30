# Zryna WebAssembly backend

Direct core WebAssembly lowering from `VerifiedProgram`.

The backend consumes only sealed verified function views and exact scalar ABI WebAssembly export
names. It emits deterministic, import-free WebAssembly 1.0 modules for the current `I32V1`
profile and validates every completed binary with the exactly pinned validator before returning an
artifact. It does not depend on a frontend provider, JavaScript output, native MIR, LLVM, or a WAT
tool.

The current instruction surface is `local.get`, `i32.const`, `i32.add`, and function `end` over
pure `i32` parameters and results. Modules contain only type, function, export, and code sections;
they contain no imports, tables, memory, globals, start function, element/data segments, custom
sections, WASI, WIT, Component Model, GC, threads, SIMD, reference types, or ambient capabilities.

Boolean core carriers remain specified and tested through the shared scalar ABI fixture, but this
does not enable Boolean source or IR. Source orchestration, artifact publication, runtime
execution, browser loaders, and public host wrappers do not belong to this backend.
