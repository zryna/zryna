# JavaScript runtime helpers

This directory is reserved for versioned helpers required to preserve specified Zryna semantics
in direct JavaScript output.

The current `I32V1` ECMAScript module is self-contained: the backend emits its private arity and
scalar validators directly, so there is no runtime package or ambient dependency to install.
Moving a helper here requires an explicit runtime ABI and versioning change; generated modules
must not silently depend on unversioned files.
