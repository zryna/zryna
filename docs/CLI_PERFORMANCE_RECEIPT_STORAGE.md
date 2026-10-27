# Historical receipt storage

The shared `tests/cli-performance-receipt-storage.test.mjs` proof covers the prior
Struct Clone receipts plus four source-room extensions: contextual-extra,
partial-static, dynamic-index and borrowed-match. Their only receipt change is
JSON formatting whitespace. Together the four extensions save 47,820 bytes before
the proof and this note. No limit or source inventory is reduced.

For every family, the test reconstructs `JSON.stringify(parsed, null, 2) + '\n'`
and checks its original complete byte length and SHA-256. It separately pins the
compact physical encoding and checks the complete ordered case/provider inventory
against unchanged packet identities. All original receipt fields survive.

Contextual-extra keeps its existing `originalReceiptsSha256` unchanged. The three
execution families have packet inventories but no such historical receipt pin;
the test asserts that absence rather than adding a new authority claim. Their
old byte hashes in the storage test identify exact preimages only.

The active contextual-extra oracle and the three execution source-seal readers
use separate frozen source/expected/packet files and input identity tables. Those
tables exclude `provider-receipts.json`; no active physical receipt pin or
canonical receipt reader was found. First-vertical and while-assignment receipts
have active byte authorities and are untouched.

This proof preserves historical syntax-only observations. It adds no source,
semantic, backend, runtime, host or performance authentication, and no baseline,
threshold, waiver or execution qualification.
