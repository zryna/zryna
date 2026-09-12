# Standalone source projects

Status: implemented on current `main` for source-checkout builds using the default `i32-v1`
profile, outside the advertised v0.1.0 preview policy. Distribution, registry templates, hooks,
watch mode, and editor integration remain separate work.

## Create a project

`zryna new` takes one explicit destination whose final component is the lowercase package name:

```text
zryna new <PATH> [--json]
```

For a project named `hello`, it publishes this complete tree with one create-only directory commit:

```text
hello/
  zryna.package.json
  zryna.lock.json
  src/main.zry
```

The manifest and frozen lock consume the implemented
[source-package authority](PACKAGE_RESOLUTION.md). The source returns `i32:42`. Repeating the same
command fails without replacing any existing file. The destination, every parent, and the private
staging path must be real directories without links or Windows reparse points. Portable package
rules reject traversal, case-unstable spellings, and Windows device stems such as `con`, `nul`,
`com1`, and `lpt1` on every host.

Creation retains the resolved parent, stage, source directory, and each file by filesystem handle.
It writes and cleans up only through those retained directories, rechecks their identities, exact
inventories, and bytes before the no-replace commit, and leaves unexpected foreign content in a
failed stage instead of deleting it. This detects persistent replacement observed at those checks;
it is not an operating-system sandbox against an arbitrary same-user process that continues racing
after validation.

On Windows, stage and source creation return opaque capabilities whose authoritative handles stay
live through commit or confirmed cleanup. The same stage handle performs the no-replace rename;
cleanup consumes the exact source and stage handles after known child handles close. A pathname
replacement is never reopened as the mutation source, and a foreign entry makes cleanup fail closed
without recursively deleting the retained directory. Linux retains its handle-relative
`renameat2(RENAME_NOREPLACE)` transaction and directory synchronization.

Generated compiler state is confined to the project-owned `.zryna` directory. That exact reserved
directory is excluded from the package source inventory; no other undeclared project file is
ignored. Deleting the project tree therefore removes its source, lock, build output, and run output
without editing the compiler checkout.

## Build and run from a source checkout

The two roots have different authority. `--root` is the real compiler checkout and always runs the
complete architecture gate. `--project-root` is the generated project and can supply only
manifest-authenticated source plus its `.zryna` state. A project cannot provide frontend/runtime
files or bypass compiler validation.

On Linux, with the repository and pinned Node.js 22.22.1 at absolute paths:

```bash
cargo build --locked --manifest-path /opt/zryna/Cargo.toml -p zryna
/opt/zryna/target/debug/zryna new /work/hello
/opt/zryna/target/debug/zryna build src/main.zry --target javascript --name hello-js --root /opt/zryna --project-root /work/hello --node /opt/node-v22.22.1/bin/node
/opt/zryna/target/debug/zryna run src/main.zry --target webassembly --name hello-wasm --export main --root /opt/zryna --project-root /work/hello --node /opt/node-v22.22.1/bin/node
rm -r /work/hello
```

On Windows PowerShell:

```powershell
cargo build --locked --manifest-path C:\src\zryna\Cargo.toml -p zryna
C:\src\zryna\target\debug\zryna.exe new C:\work\hello
C:\src\zryna\target\debug\zryna.exe build src/main.zry --target javascript --name hello-js --root C:\src\zryna --project-root C:\work\hello --node C:\tools\node-v22.22.1\node.exe
C:\src\zryna\target\debug\zryna.exe run src/main.zry --target webassembly --name hello-wasm --export main --root C:\src\zryna --project-root C:\work\hello --node C:\tools\node-v22.22.1\node.exe
Remove-Item -Recurse -LiteralPath C:\work\hello
```

The run prints `webassembly: i32 42`. Artifacts and manifests are under
`<project>/.zryna/out/<name>.<build|run>`. Publication remains create-only, so choose a fresh
`--name` for each command kind. JavaScript and core WebAssembly builds/runs are supported on Linux
and Windows; native execution retains its separate Linux x86-64 GNU requirements.

## Admission failures

Build and run use frozen package resolution before compilation and revalidate the same graph before
publishing. Missing or noncanonical manifests/locks, changed checksums, extra files, local packages
outside the selected project tree, links, unsafe roots, incompatible compiler/profile/targets, or
an existing output bundle fail without publication. Explicit profiles and `component` are not yet
accepted with `--project-root`; the generated package declares default `i32-v1` only.
