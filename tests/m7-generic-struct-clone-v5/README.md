# Historical receipt storage

`provider-receipts.json` stores the same 24 historical syntax-only observations in
compact JSON. Only formatting whitespace changed: 193,907 bytes became 139,599.
The historical `packet-identities.json.originalReceiptsSha256` remains unchanged.
Parsing the compact document and rendering `JSON.stringify(value, null, 2) + '\n'`
reconstructs every original byte and its original SHA-256. The separate storage
test pins both encodings and checks the complete case/provider inventory.

Run `node --test tests/cli-performance-receipt-storage.test.mjs`. This storage proof
does not authenticate sources or qualify any new semantic or target execution.
