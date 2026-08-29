# Roadmap

## Foundation

- strict repository contract and fail-closed architecture engine;
- stable diagnostics and source spans;
- frontend handshake and normalized snapshot contract;
- exact TypeScript 6 adapter pin;
- verified Universal IR;
- direct JavaScript and native-backend boundaries;
- Linux and Windows CI.

## First executable vertical slice

- parse one `.zry` entrypoint through the TypeScript 6 adapter;
- lower functions, parameters, literals, returns, `bool`, and `i32`;
- reject `any` and unsupported syntax with stable source diagnostics;
- emit and execute an ECMAScript module;
- lower native MIR to a real object with an initial Rust-native codegen backend;
- link and run a Linux x86-64 executable;
- compare JavaScript and native results.

## Control flow and modules

- arithmetic and comparisons with boundary tests;
- local bindings, calls, `if`, and `while`;
- deterministic module resolution;
- JavaScript/native differential suites.

## Data and memory

- structs, enums, fixed arrays, and layout verification;
- owned strings and vectors;
- move checking and deterministic drop insertion;
- borrowing;
- explicit shared and weak references;
- versioned native runtime ABI.

## Language growth

- generics and monomorphization;
- `Option` and `Result`;
- native-only FFI profile;
- native Zryna frontend;
- language server and thin editor extension;
- additional native platforms after conformance gates exist.

An optional tracing-GC profile requires a separate language and ABI proposal. It is not implied by the no-GC ownership profile.
