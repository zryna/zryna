import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseCanonical, requireValue, sha256, writeNew } from './common.mjs';
import { readResponse } from './github.mjs';
const path=new URL('../../distribution/materials-rust-v1.json',import.meta.url);
const original=readFileSync(path);
requireValue(sha256(original)==='32b6343a6dfae19d7c3cdcc50658762c2c0ad08aca15124bdce85b28db38a84b',
  'compiler upstream approved base descriptor');
const url='https://raw.githubusercontent.com/bytecodealliance/wasmtime/7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';
const pins=parseCanonical(original).packages.flatMap(pkg=>pkg.files).filter(file=>file.origin===url);
requireValue(pins.length>0 && pins.every(pin=>pin.sha256===pins[0].sha256 && pin.size===pins[0].size),
  'compiler upstream exact license identity');
export async function acquireUpstreamLicense(output,fetchImpl=fetch) {
  const response=await fetchImpl(url,{redirect:'error',signal:AbortSignal.timeout(30000),
    headers:{'Accept-Encoding':'identity'}});
  requireValue(response.status===200,'compiler actual upstream license acquisition');
  const data=await readResponse(response,2*1024*1024);
  requireValue(data.length===pins[0].size && sha256(data)===pins[0].sha256,'compiler upstream original license bytes');
  writeNew(output,data);return {bytes:data.length,sha256:sha256(data)};
}
if(process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  try{requireValue(process.argv.length===3,'compiler upstream CLI');await acquireUpstreamLicense(resolve(process.argv[2]));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
