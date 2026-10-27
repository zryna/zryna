# Historical receipt storage

`provider-receipts.json` stores the same six historical syntax-only observations
as compact JSON: 48,242 bytes became 34,750, saving 13,492 bytes. Only formatting
whitespace changed. The historical `packet-identities.json.originalReceiptsSha256`
remains unchanged. `JSON.stringify(JSON.parse(bytes), null, 2) + '\n'` reconstructs
every original byte and its SHA-256. The shared storage test pins both encodings
and the complete three-case, two-provider inventory for this fixture and the
previously compacted struct-clone-v5 fixture.

Run `node --test tests/cli-performance-receipt-storage.test.mjs`. This storage proof
adds no source authentication or semantic, backend, runtime or performance credit.
