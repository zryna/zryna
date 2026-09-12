import { roleFor } from '../distribution/inventory.mjs';
import { exactKeys, parseCanonical, sha256 } from '../distribution/canonical.mjs';
import { canonicalBounded } from './canonical.mjs';
import { validateReleaseQualificationArchitecture } from './validate-release-qualification-architecture.mjs';
import { validateReleaseQualificationGates } from './validate-release-qualification-gates.mjs';
import { validateReleaseQualificationInput } from './validate-release-qualification-input.mjs';
import { validateReleaseQualificationSourceText } from './validate-release-qualification-source.mjs';

function reject(message) {
  throw new Error(`R406-QUALIFICATION-INPUT-PRODUCER: ${message}`);
}

function artifact(logicalPath, bytes) {
  if (!Buffer.isBuffer(bytes) || bytes.length < 1) reject(`${logicalPath} bytes are missing`);
  return { logicalPath, size: bytes.length, sha256: sha256(bytes) };
}

function material(path, target) {
  const node = target === 'x86_64-pc-windows-msvc'
    ? 'runtime/node/node.exe' : 'runtime/node/bin/node';
  if (path === node || path === 'licenses/node-LICENSE') return 'node-22.22.1';
  if (path.includes('/@typescript/typescript6/') || path === 'licenses/typescript6-LICENSE.txt') {
    return 'typescript6-6.0.2';
  }
  if (path.includes('/@typescript/old/') || path.startsWith('licenses/typescript-')) {
    return 'typescript-6.0.3';
  }
  if (path.startsWith('licenses/rust/')) return sha256(Buffer.from(path.split('/')[2], 'ascii'));
  return 'source';
}

function licenses(path, role, origin) {
  if (role === 'license') return [path];
  if (origin === 'node-22.22.1') return ['licenses/node-LICENSE'];
  if (origin === 'typescript6-6.0.2') return ['licenses/typescript6-LICENSE.txt'];
  if (origin === 'typescript-6.0.3') {
    return ['licenses/typescript-LICENSE.txt', 'licenses/typescript-ThirdPartyNoticeText.txt'];
  }
  return ['LICENSE'];
}

function materialDescriptor(file, target) {
  if (!file || !Buffer.isBuffer(file.data)) reject('captured material bytes are missing');
  const role = roleFor(file.path, target);
  const origin = material(file.path, target);
  return { path: file.path, mode: file.mode, size: file.data.length, sha256: sha256(file.data),
    role, material: origin, licenses: licenses(file.path, role, origin) };
}

function recipe(bytes, target) {
  const value = parseCanonical(bytes);
  exactKeys(value, ['compile', 'format', 'productionAdmission', 'status', 'versionCandidate']);
  exactKeys(value.compile, ['argv', 'targets']);
  exactKeys(value.compile.targets, ['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu']);
  for (const entry of Object.values(value.compile.targets)) {
    exactKeys(entry, ['encodedLinkerFlags', 'encodedRustFlags', 'environment']);
  }
  if (value.format !== 'zryna.distribution-recipe.v1'
    || value.productionAdmission !== 'forbidden' || value.status !== 'qualification-proposal'
    || value.versionCandidate !== '0.2.0'
    || JSON.stringify(value.compile.argv) !== JSON.stringify([
      'cargo', 'rustc', '--locked', '--release', '--target', '@target@',
      '-p', 'zryna', '--bin', 'zryna',
    ])) reject('qualification recipe proposal differs');
  return { value, target: value.compile.targets[target] };
}

function qualificationEnvironment(entries, hostEnvironment = {}) {
  return entries.map(({ name, value }) => {
    const match = /^@host-([A-Za-z][A-Za-z0-9_]*)@$/.exec(value);
    if (!match) return { name, value };
    const observed = hostEnvironment[match[1]];
    if (typeof observed !== 'string' || !/^[ -~]{1,512}$/.test(observed)) {
      reject(`host environment ${match[1]} differs`);
    }
    return { name, value: observed };
  });
}

export function createReleaseQualificationInput({
  sourceBytes, architectureBytes, gateBytes, recipeBytes, target, capturedMaterials, tools,
}) {
  const source = validateReleaseQualificationSourceText(sourceBytes.toString('utf8'));
  const architecture = validateReleaseQualificationArchitecture(parseCanonical(architectureBytes));
  const gates = validateReleaseQualificationGates(parseCanonical(gateBytes));
  const proposal = recipe(recipeBytes, target);
  if (!proposal.target || architecture.source.commit !== source.source.commit
    || architecture.source.tree !== source.source.tree || gates.sourceCommit !== source.source.commit) {
    reject('qualification source projections differ');
  }
  const sourceReceipt = artifact('qualification/source.json', sourceBytes);
  const architectureReceipt = {
    ...artifact('qualification/architecture.json', architectureBytes),
    sourceCommit: source.source.commit, sourceTree: source.source.tree,
  };
  const gateReceipt = {
    ...artifact('qualification/gates.json', gateBytes),
    runId: gates.runId, runAttempt: gates.runAttempt, sourceCommit: gates.sourceCommit,
    requiredJobs: gates.requiredJobs.map(({ name, conclusion, sourceCommit }) => ({
      name, conclusion, sourceCommit,
    })),
  };
  const windows = target === 'x86_64-pc-windows-msvc';
  return validateReleaseQualificationInput({
    format: 'zryna.release-qualification-input.v1',
    status: 'provisional-candidate', productionAdmission: 'forbidden', versionCandidate: '0.2.0',
    source: source.source,
    workflow: { path: source.workflow.path, sha256: source.workflow.sha256 },
    target: windows
      ? { triple: target, platformBaseline: { os: 'windows', product: 'windows-server',
        version: '2022', architecture: 'x86_64', runtime: 'operating-system-ucrt' } }
      : { triple: target, platformBaseline: { os: 'linux', distribution: 'ubuntu',
        version: '24.04', architecture: 'x86_64' } },
    recipeProposal: { gitPath: 'scripts/distribution/release-recipe-v1.json',
      size: recipeBytes.length, sha256: sha256(recipeBytes) },
    sourceReceipt,
    architectureReceipt,
    gateReceipt,
    toolchains: tools.toolchains,
    nativeTools: tools.nativeTools,
    materials: { files: capturedMaterials.map((file) => materialDescriptor(file, target)) },
    compile: {
      argv: proposal.value.compile.argv.map((value) => value === '@target@' ? target : value),
      environment: qualificationEnvironment(proposal.target.environment, tools.hostEnvironment),
      encodedRustFlags: proposal.target.encodedRustFlags,
      encodedLinkerFlags: proposal.target.encodedLinkerFlags,
    },
    archive: windows
      ? { format: 'zip', root: `zryna-qualification-0.2.0-${target}-${source.source.commit.slice(0, 12)}`,
        method: 'store', creator: 'unix-2.0', timestamp: '1980-01-01T00:00:00',
        extraFields: false, comments: false }
      : { format: 'tar-gzip',
        root: `zryna-qualification-0.2.0-${target}-${source.source.commit.slice(0, 12)}`,
        tarFormat: 'ustar', uid: 0, gid: 0, owner: '', group: '', entryMtime: 'source-epoch',
        gzipLevel: 9, gzipMtime: 0, gzipOs: 255 },
  });
}

export function releaseQualificationInputBytes(options) {
  return Buffer.from(`${canonicalBounded(createReleaseQualificationInput(options))}\n`);
}
