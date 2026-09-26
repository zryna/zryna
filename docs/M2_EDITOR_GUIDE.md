# Edit and run M2 scalar programs

The Zryna editor's `control-flow-v1` mode handles local `const` and `let` bindings,
direct function calls, `if`/`else`, and `while` in saved `.zry` files. The compiler
checks exact `i32` and `bool` types. JavaScript and WebAssembly are the portable Run
targets. This guide uses the M2 editor candidate in an isolated VS Code profile;
the compiler's explicit `--profile control-flow-v1` selector remains authoritative.

## Install and select the profile

Verify the reviewer-supplied setup archive and `setup.json` digest before installation,
then follow [Portable setup](PORTABLE_SETUP.md) into a **new** installation and editor
profile. Open a trusted local folder containing your `.zry` file. Run **Zryna: Select
Editor Profile** and choose **M2 control flow** before editing M2 source. The default
profile remains **Scalar (i32-v1)** for existing scalar files. Changing the profile
reconnects the active document so its diagnostics and formatter use the selected
compiler boundary. Selecting a Run profile also selects the matching editor profile
after all Run choices are complete.

The editor does not run code when a file opens, changes, formats, or saves. Use
**Zryna: Run Saved File** explicitly. Select the profile, an exported function,
JavaScript or WebAssembly, then enter each prompted value. The function signature
determines whether a prompt accepts an integer or Boolean. Save first; an unsaved
revision is never substituted for the saved source. Canceling a picker starts no
compiler run.

## Try local bindings and direct calls

Save this as `main.zry` in a disposable folder:

```typescript
function twice(value: i32): i32 {
  return value * 2;
}

export function main(value: i32): i32 {
  const offset: i32 = 3;
  let total: i32 = twice(value);
  total = total + offset;
  return total;
}
```

Run `main` and enter `5` for its integer input. The expected result is `i32 13`
on both JavaScript and WebAssembly. **Format Document** applies the canonical
two-space/LF style; a second format should make no edit. Formatting incomplete or
unsupported source makes no edit.
For a selected range, choose complete functions. Range formatting returns edits only
for complete functions and leaves bytes outside the selection untouched. A partial
function selection makes no edit.

## Try both branch paths and a loop

Replace the file with the following branch, then save:

```typescript
export function main(positive: bool, value: i32): i32 {
  if (positive) {
    return value;
  } else {
    return -value;
  }
}
```

Enter `true` and `13` to get `i32 13`. Enter `false` and `13` to get
`i32 -13`. The prompts follow the function's `bool`, `i32` signature; `bool`
has no numeric truthiness conversion.

For a loop, replace the file with this source and save:

```typescript
export function main(count: i32): i32 {
  let i: i32 = 0;
  let sum: i32 = 0;
  while (i < count) {
    i = i + 1;
    sum = sum + i;
  }
  return sum;
}
```

Entering `0` returns `i32 0`; entering `5` returns `i32 15` on both targets. Each Run
uses a fresh isolated compiler project and records the saved source identity in the
**Zryna Run** output. **Zryna: Open Generated JavaScript** opens the last successful
JavaScript artifact, and **Zryna: Reveal Run Output** selects the actual output file.
Neither command changes the original source.

## Read errors and current limits

In M2 mode, assigning to `const` reports `ZRYNA-M2005`, an unresolved local name
reports `ZRYNA-M2004`, and an exact local type mismatch reports `ZRYNA-M2006`.
The compiler may report additional related diagnostics on the same invalid source.
Incomplete syntax receives a diagnostic and no formatting edit or Run bundle.
Top-level variables are outside this profile; use initialized locals inside a
function. The editor stages one saved file for Run, so imports are not available
there. The formatter also excludes imports and standalone grouping parentheses
around expressions, such as `(a + b)`. `break`,
`continue`, `for`, classes, implicit truthiness, recursion, and M3 ownership/data
syntax are outside this M2 editor mode. M2 does not offer Go to
Definition; the scalar editor mode retains its existing definition support.

The extension serves one active local document at a time and requires workspace
trust. M2 formatted output is capped at 131,072 UTF-8 bytes; M2 Run accepts at
most 2 MiB. The existing scalar Run limit remains 1,024 UTF-8 bytes. Run accepts
only saved files, exact `i32`/`bool` inputs and scalar results. The `i32:5` and
`bool:true` spellings belong to the CLI's `--arg` syntax; enter just `5` or `true`
in the editor prompts.
It does not execute native Windows programs, download toolchains, or provide a
debugger. The compiler remains responsible for source validity, exact types, source
authentication and published artifacts. For the complete language boundary, see
[M2 control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md) and the [CLI reference](CLI.md).
