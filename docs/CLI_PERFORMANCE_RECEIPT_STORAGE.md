# Historical receipt storage

The shared `tests/cli-performance-receipt-storage.test.mjs` proof covers the prior
Struct Clone receipts plus five source-room extensions: contextual-extra,
partial-static, dynamic-index, borrowed-match and original-deep. Their only receipt
change is JSON formatting whitespace. Together the five extensions save 52,320
bytes before the proof and this note. No limit or source inventory is reduced.

For every family, the test reconstructs `JSON.stringify(parsed, null, 2) + '\n'`
and checks its original complete byte length and SHA-256. It separately pins the
compact physical encoding and checks the complete ordered case/provider inventory
against unchanged packet identities. All original receipt fields survive.

Contextual-extra and original-deep keep existing `originalReceiptsSha256`
unchanged. The other three execution families have packet inventories but no such historical receipt pin;
the test asserts that absence rather than adding a new authority claim. Their
old byte hashes in the storage test identify exact preimages only.

The active contextual-extra oracle and the three execution source-seal readers
use separate frozen source/expected/packet files and input identity tables. Those
tables exclude `provider-receipts.json`; no active physical receipt pin or
canonical receipt reader was found. First-vertical and while-assignment receipts
have active byte authorities and are untouched, as does scalar-replay.
The original-deep IR test reads only `main.zry` and its two provider DTO packets;
it does not read the receipt or impose a physical receipt hash/format contract.

This proof preserves historical syntax-only observations. It adds no source,
semantic, backend, runtime, host or performance authentication, and no baseline,
threshold, waiver or execution qualification.

The same storage suite also covers the imported Copy enum v5 syntax DTO at
`tests/m7-generic-owned-copy-enum-phi/2-reference.json`. Compact JSON preserves
every value and ordering; reconstructing its original two-space representation
recovers all 508,297 bytes and their original SHA-256. The compact form has its
own physical pin. Existing source/arena/semantic checks consume the same DTO;
the single/imported source seal and nine-export attack/recovery proof still run.
