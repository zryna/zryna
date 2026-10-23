import { createRequire } from 'node:module';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root = '/tmp/m416-nominal-transport-candidate';
const require = createRequire(`${root}/adapters/typescript-6/package.json`);
const Ajv2020 = require('ajv/dist/2020.js').default;
const schema = JSON.parse(readFileSync(`${root}/schemas/zryna-syntax-v5.schema.json`));
const validate = new Ajv2020({strict:true, allErrors:true}).compile(schema);
const dir = fileURLToPath(new URL('./', import.meta.url));
const receipts = JSON.parse(readFileSync(`${dir}provider-receipts.json`));
for (const row of receipts.records) {
  const bytes = readFileSync(`${dir}${row.packet}`);
  if (createHash('sha256').update(bytes).digest('hex') !== row.packetSha256) throw Error(row.packet);
  if (!validate(JSON.parse(bytes))) throw Error(`${row.packet}: ${JSON.stringify(validate.errors)}`);
}
writeFileSync(`${dir}schema-validation.json`, JSON.stringify({
  status:'Closed raw JSON schema validation only; no authenticated syntax or compiler proof claimed',
  nodeVersion:process.version, packets:receipts.records.length,
  schemaSha256:createHash('sha256').update(readFileSync(`${root}/schemas/zryna-syntax-v5.schema.json`)).digest('hex'),
  passed:true,
},null,2)+'\n');
console.log(`${receipts.records.length} complete packets pass strict v5 raw schema`);
