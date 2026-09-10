# Zryna Windows filesystem foundation

This crate owns Zryna's only approved unsafe-Rust exception. It exposes three Windows-only,
handle-relative directory operations on one opaque capability needed by create-only transactions:

- atomic creation of one new directory with an immediately retained authoritative handle;
- no-replace rename of that exact source handle beneath a retained destination parent; and
- empty-directory deletion through that exact owned handle, followed by retained-parent absence
  confirmation.

The creation handle requests `DELETE`, permits read and write sharing, and excludes delete sharing.
The returned `OwnedDirectory` wraps that handle once as a `cap_std::fs::Dir` and retains its parent
and current name through child operations, commit, rollback, and cleanup. No public mutation accepts
an arbitrary file handle. The API accepts one bounded portable ASCII component, never an ambient
path, and provides no path-based fallback or replacement mode.

Removal consumes `OwnedDirectory`, marks the exact empty directory for deletion, closes its handle,
and attempts a no-reparse open of the bound name relative to the retained parent. It returns success
only for an unambiguous not-found result. A foreign replacement or another share-delete handle that
keeps deletion pending returns an error; after marking, that error can outlive this capability until
the other handle closes.

Unsafe operations are confined to the private Windows syscall module. Its invariants are:

1. every raw pointer references a live, correctly aligned and fully initialized ABI buffer;
2. UTF-16 and total buffer lengths are checked before narrowing to Windows integer types;
3. every successful raw handle acquisition is transferred exactly once into RAII ownership;
4. borrowed handles remain live for each synchronous call; and
5. native errors are surfaced after NTSTATUS-to-Win32 mapping, and deletion is not called confirmed
   merely because its authoritative handle accepted a delete disposition.

Focused Windows tests exercise the complete same-handle lifecycle, sharing behavior, destination
collisions, hostile pathname mutation, regular-file rejection, deletion-pending contention, exact
255/256-unit bounds, and failure-path handle close. The repository structure gate independently
enforces canonical lint tables and lexically inspects every Rust source path, including unreferenced,
case-varied, and configuration-disabled files, while excluding comments and literals. This keeps the
exact component manifest and private-module unsafe allowance as the only exception. The crate grants
no general filesystem authority and has no non-Windows API.
