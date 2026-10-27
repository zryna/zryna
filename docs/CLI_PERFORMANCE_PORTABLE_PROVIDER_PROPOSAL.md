# Portable GNU time provider

The Ubuntu 24.04 x86-64 acquisition recipe is implemented in merged #576. It
produces a private provider binding; source, host and performance admission remain
separate. Authority flags stay false and thresholds stay null.

```sh
node scripts/stability-gates/cli-performance-linux-provision-provider.mjs /absolute/canonical/checkout
```

The fixed [v2 recipe](../scripts/stability-gates/provider-inputs/linux-gnu-time-debian-trixie-v2.json)
pins the sole package URL, 128 KiB download bound, package/executable hashes and
runtime record. The command accepts no replacement URL, hash, program or grant.
It extracts into a fresh mode-0700 root outside the checkout, with no install,
maintainer scripts, elevation or system overwrite. Incomplete roots remain visible.

Capture binds the actual provider root and receipt digest. Package, executable,
loader, runtime libraries and persistent file identities are checked around each
call. Historical GPG provenance is explicitly not current-host verification.
Trusted canonical ancestors and the operator/system account remain prerequisites;
this is not a hostile same-UID sandbox.

The [collection contract](CLI_PERFORMANCE_DRAFT.md) retains 24 Linux cells and six
attempts per cell. Memory remains GNU wait4 maximum RSS KiB, not simultaneous
summed tree RSS. Current package/runtime observation does not grant baseline or
release authority.

The [original proposal](https://github.com/zryna/zryna/blob/c1ae34fe421e3dfe63d1182054c9753ea2128dad/docs/CLI_PERFORMANCE_PORTABLE_PROVIDER_PROPOSAL.md)
preserves the historical packet scope and pre-execution author record.
