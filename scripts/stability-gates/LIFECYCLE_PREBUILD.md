# Local Linux lifecycle prebuild observations

`captureLifecyclePrebuild` in `lifecycle-owner-capture.mjs` collects trusted-code build
observations on Linux x86-64 with Node 22.22.1. Call it under
`linux-operator-owner.py`, with one local Cargo operation at a time. Supply the
selected source commit/tree, canonical checkout, fresh external output directory,
explicit Cargo/rustc/Git paths, the fixed thirteen-key environment and the original
pinned Wasmtime license file. The exact Cargo command uses locked, offline release
mode and `--test lifecycle --no-run --message-format=json -vv`. It never executes
either discovered application artifact.

The collector retains commands, original child streams and termination records before
interpreting them. It checks the complete source census, tool and Cargo input bytes,
Cargo configuration absence and original crate/cache/license correspondence before
and after the build. The final source snapshot follows both post-build queries.
Uncertain cleanup stops all further subprocesses and records unavailable after-state;
a confirmed ordinary build failure still retains postflight observations.

`discovery.json` records discovered artifacts, not independently approved selections.
Source/tool origin, host/runtime approval and M7 qualification remain false. Injected
observation adapters are explicitly labelled synthetic and cannot establish admission.
A native lifecycle proof still requires authenticated materials/runtime closure and a
separately owned process envelope with demonstrated effective resource isolation.
