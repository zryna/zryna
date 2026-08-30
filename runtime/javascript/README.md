# JavaScript runtime helpers

This directory is reserved for versioned helpers required to preserve specified Zryna semantics
in direct JavaScript output.

The current `I32V1` ECMAScript module is self-contained: the backend emits its private arity and
scalar validators directly, so there is no runtime package or ambient dependency to install.
`zryna run --target javascript` executes the sealed module with an exact validated Node.js
22.22.1 host under the driver's bounded, controlled process contract. Node is not a bundled Zryna
runtime, and browser execution is not claimed.

Moving a helper here requires an explicit runtime ABI and versioning change; generated modules
must not silently depend on unversioned files.
