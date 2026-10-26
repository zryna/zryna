// Synthetic unit observations only: these do not authenticate a source/host/run or execute a proof.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { sha256 } from '../scripts/stability-gates/input.mjs';
import { parseRequiredDeclarations } from '../scripts/stability-gates/required-owner-logs.mjs';
export const source = { role: 'tested-checkout', revision: '1'.repeat(40), tree: '2'.repeat(40) };
export const index = parseRequiredDeclarations(readFileSync(new URL('./stability-required-ignored-v2.json', import.meta.url)));
const actualRoot = process.env.ZRYNA_REQUIRED_OWNER_TEST_SOURCE_ROOT ?? fileURLToPath(new URL('../', import.meta.url));
export const readMaterial = path => readFileSync(join(actualRoot, path));
export function transcript(rows, filtered = 0) {
  const count = status => rows.filter(row => row.outcome === status).length;
  return `running ${rows.length} tests\n${rows.map(row => `test ${row.name} ... ${row.outcome}`).join('\n')}\n\ntest result: ok. ${count('ok')} passed; ${count('FAILED')} failed; ${count('ignored')} ignored; 0 measured; ${filtered} filtered out; finished in 0.01s\n`;
}
export function fixture(owner = 'preflight-abi', host = 'linux-x86_64') {
  const required = index.rows.filter(row => row.hosts.includes(host) && (row.primaryOwnerByHost[host] === owner || row.alternatives.includes(owner)));
  const keys = [...new Set(required.map(row => `${row.package}/${row.target}`))];
  const logs = new Map(); let next = 0;
  function descriptor(text) {
    const path = `logs/fixture-${next++}.log`, bytes = Buffer.from(text);
    logs.set(path, bytes); return { path, size: bytes.length, sha256: sha256(bytes) };
  }
  const targets = keys.map(key => {
    const [pkg, target] = key.split('/');
    const rows = required.filter(row => row.package === pkg && row.target === target).map(row => ({ name: row.fullTestName, outcome: 'ok' }));
    return { package: pkg, target, stdout: descriptor(transcript(rows, 91)), stderr: descriptor('') };
  });
  const defaults = keys.map(key => {
    const [pkg, target] = key.split('/');
    const rows = index.rows.filter(row => row.hosts.includes(host) && row.package === pkg && row.target === target).map(row => ({ name: row.fullTestName, outcome: 'ignored' }));
    return { package: pkg, target, stdout: descriptor(transcript([{ name: 'ordinary_nonrequired_control', outcome: 'ok' }, ...rows])), stderr: descriptor('') };
  });
  return { observation: { format: 'zryna.required-owner-log-observation.v1', source: structuredClone(source), host, owner,
    argv: structuredClone(index.owners[owner].childArgv), exitCode: 0, signal: null, error: null, defaults, targets },
  context: { source, host, readLog: path => logs.get(path), readMaterial }, logs,
  rewrite(descriptor, text) { const bytes = Buffer.from(text); logs.set(descriptor.path, bytes); descriptor.size = bytes.length; descriptor.sha256 = sha256(bytes); } };
}
