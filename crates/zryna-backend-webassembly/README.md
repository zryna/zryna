# Zryna WebAssembly backend

Direct core WebAssembly lowering from the M1 `VerifiedProgram` and the isolated M2
`control_flow_v1::VerifiedProgram`.

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

The separate internal M2 entrypoint consumes only sealed `ControlFlowV1` views. It lowers every
current scalar operation, direct call, return, branch, jump, loop, and parallel block edge into a
deterministic core module with only type, function, export, and code sections. It validates the
complete module, exhaustively audits indexes, exports, locals, operators, and capabilities, and
caps incremental encoding at 32 MiB. This internal API does not activate protocol v3, manifest v2,
or the public M2 CLI. See [the M2 WebAssembly contract](../../docs/M2_WEBASSEMBLY_BACKEND.md).
