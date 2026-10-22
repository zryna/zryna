// Fixed corpus, fresh contained project for each operation. Nothing in this module executes a CLI.
import { corpus } from '../../corpus.mjs';
import { canonical, fail, freeze, packageBytes, packageDigest, sha256 } from './records.mjs';

export const release = freeze({ version: '0.2.3', tag: 'v0.2.3',
  sourceCommit: '0decfd2056a77824003e386fcd78839b5b78d6b9', sourceTree: '67f7d033003689a8cfcb7dc5691df86c85c9e603',
  archive: { bytes: 52949082, sha256: 'b258d45e1eb58a701026ced7c74d03dc74c911c90a5ec7da93c656884a97159f', files: 349 },
  compiler: { path: 'bin/zryna', bytes: 17376136, sha256: '00ac9084bbff5c4d0d56eb1e8cd03e63e49f2ca38fa5dcc63af05f32fc50d632', mode: 493 },
  node: { path: 'runtime/node/bin/node', bytes: 124674920, sha256: '243fd8938011479f41b3de101842150fa990f33fbbb3f7aabd330857f2d79e1d', mode: 493 },
  envelopeSha256: '49cbe33ae33d8c926b2ced151435d5e347e42bbe293eef539643a70fccea3ad6',
  bundleSha256: '6ff8a11cbb2ba3bc5f99add0e509fc5802b85401a13766d6c9ff827ab15e03c1' });
// Snapshot the trusted static fixture; mutating an imported corpus cannot replace a plan later.
export const fixedCorpus = freeze(structuredClone(corpus));

export function projectFiles(source) {
  if (!fixedCorpus.some(entry => entry.source === source)) fail();
  const data = Buffer.from(source);
  const files = [{ path: 'src/main.zry', size: data.length, sha256: sha256(data) }];
  const compatibility = { compiler: '0.2.3', profile: 'i32-v1', targets: ['javascript'] };
  const manifest = { format: 'zryna.package.v1', name: 'project', version: '0.1.0',
    source: { kind: 'local', locator: 'project', revision: '' }, compatibility, files, dependencies: [] };
  const id = packageDigest('manifest', manifest);
  const lock = { format: 'zryna.lock.v1', root: id, compatibility,
    packages: [{ id, sourceSha256: packageDigest('source-files', files), dependencies: [] }] };
  return [['src/main.zry', data], ['zryna.lock.json', packageBytes(lock)], ['zryna.package.json', packageBytes(manifest)]]
    .map(([path, bytes]) => ({ path, bytes: bytes.length, sha256: sha256(bytes), mode: 384, bodyBase64: bytes.toString('base64') }));
}
function operation(entry, kind, index, logical = null, args = [], value = null) {
  const argv = [kind, 'src/main.zry', '--target', 'javascript', '--project-root', '/work/project', '--name', 'parity'];
  if (kind === 'run') argv.push('--export', logical, ...args.map(argument => `--arg=i32:${argument}`));
  argv.push('--json');
  return { id: kind + '-' + entry.id + (kind === 'run' ? '-' + index : ''), kind, corpusId: entry.id,
    sourceSha256: sha256(Buffer.from(entry.source)), sourceBytes: Buffer.byteLength(entry.source),
    logical, args: [...args], expected: { status: entry.cases ? 'compiled' : 'rejected', value,
      code: entry.code ?? null }, executable: '/opt/zryna-release/bin/zryna', cwd: '/work/project', argv,
    stdin: 'none', activeMs: 10000, cleanupMs: 2000, projectFiles: projectFiles(entry.source) };
}
export function createParityPlan() {
  const operations = [];
  for (const entry of fixedCorpus) {
    operations.push(operation(entry, 'build', 0));
    for (const [index, item] of (entry.cases ?? []).entries()) {
      operations.push(operation(entry, 'run', index, entry.export, item.args, item.value));
    }
    if (entry.extra) operations.push(operation(entry, 'run', entry.cases.length,
      entry.extra.export, entry.extra.args, entry.extra.value));
  }
  if (operations.length !== 19 || operations.filter(value => value.kind === 'run').length !== 10) fail();
  return freeze({ format: 'zryna.playground-cli-parity-plan.v1', version: 1,
    scope: 'published-default-i32-v1-javascript-vs-playground-browser-scalars',
    release, executionApproved: false, hostNativeExecutionAllowed: false,
    fullInstalledInventoryRequired: true, originalArchiveModesRequired: true,
    browserComponentProfileInOldCli: false, operations });
}
export const parityPlanSha256 = () => sha256(Buffer.from(canonical(createParityPlan())));
export function findOperation(id) {
  const operation = createParityPlan().operations.find(value => value.id === id);
  if (!operation) fail();
  return operation;
}
