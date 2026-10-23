import { createRequire } from 'node:module';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const root='/tmp/m416-nominal-transport-candidate';
const require=createRequire(`${root}/adapters/typescript-6/package.json`);
const Ajv2020=require('ajv/dist/2020.js').default;
const schemaBytes=readFileSync(`${root}/schemas/zryna-syntax-v5.schema.json`);
const validate=new Ajv2020({strict:true,allErrors:true}).compile(JSON.parse(schemaBytes));
const output=process.argv[2];
const records=JSON.parse(readFileSync(`${output}/provider-receipts.json`)).records;
let checked=0;
for(const row of records){
  if(row.packet===null)continue;
  const bytes=readFileSync(`${output}/${row.packet}`);
  if(createHash('sha256').update(bytes).digest('hex')!==row.packetSha256)throw Error(`changed packet ${row.packet}`);
  if(!validate(JSON.parse(bytes)))throw Error(`${row.packet}: ${JSON.stringify(validate.errors)}`);
  checked++;
}
writeFileSync(`${output}/schema-validation.json`,JSON.stringify({status:'Actual strict closed raw v5 schema validation; no source/semantic/seal/runtime authority',packets:checked,schemaSha256:createHash('sha256').update(schemaBytes).digest('hex'),passed:true},null,2)+'\n');
console.log(`${checked} actual full packets pass strict raw v5 schema`);
