// Current security-candidate crate observations; no release or execution authority.
import { bytes, exactKeys, requireValue, sha256 } from '../distribution/canonical.mjs';
import { securityNativeRecipe, securityRustMaterials } from '../distribution/rust-security-materials.mjs';
import { captureTarGzipMembers } from '../distribution/npm-materials.mjs';
import { selectCompilerGraph } from '../m6/compiler-release/material-graph.mjs';
import { compilerTreeSelection } from '../m6/compiler-release/material-tree.mjs';
import { identity } from '../m6/compiler-release/material-lock.mjs';
import { validateCompilerCrateCache } from '../m6/compiler-release/material-cache.mjs';

const TARGET = 'x86_64-unknown-linux-gnu';
const UPSTREAM = 'https://raw.githubusercontent.com/bytecodealliance/wasmtime/'
  + 'c6846e13f64b07b7912d372db141397058b55f99/LICENSE';
const APPROVED = new Map(securityRustMaterials(TARGET).map(pin => [identity(pin), pin]));

export function selectLifecycleCrates(input) {
  exactKeys(input, ['metadata', 'sourceRoot', 'cargoLock', 'cargoTree']);
  const { metadata, sourceRoot, cargoLock, cargoTree } = input;
  requireValue(Buffer.isBuffer(cargoLock) && cargoLock.length === securityNativeRecipe.cargoLock.bytes
    && sha256(cargoLock) === securityNativeRecipe.cargoLock.sha256, 'lifecycle security-candidate exact lock');
  const app = metadata?.packages?.filter(pkg => pkg.name === 'zryna-playground-compiler' && pkg.source === null);
  requireValue(app?.length === 1 && Array.isArray(app[0].dependencies)
    && !app[0].dependencies.some(dep => dep.kind === 'dev'), 'lifecycle normal/build selection forbids root dev dependencies');
  const { packageIds, proof: tree } = compilerTreeSelection(cargoTree, metadata, sourceRoot);
  const { external, proof } = selectCompilerGraph({ metadata, sourceRoot, cargoLock,
    approved: APPROVED, packageIds });
  // Do not expose mutable references to the independently pinned module-level records.
  return structuredClone({ external, proof, tree });
}

export function captureLifecycleCrates(input) {
  exactKeys(input, ['metadata', 'sourceRoot', 'cargoLock', 'cargoTree', 'crateArchives', 'upstreamLicense']);
  const { metadata, sourceRoot, cargoLock, cargoTree, crateArchives, upstreamLicense } = input;
  const selection = selectLifecycleCrates({ metadata, sourceRoot, cargoLock, cargoTree });
  requireValue(Array.isArray(crateArchives) && crateArchives.length === selection.external.length,
    'lifecycle exact original crate capture count');
  const captures = new Map();
  for (const capture of crateArchives) {
    exactKeys(capture, ['identity', 'archive', 'cacheRoot']);
    requireValue(typeof capture.identity === 'string' && !captures.has(capture.identity), 'lifecycle unambiguous original capture');
    captures.set(capture.identity, capture);
  }
  requireValue(upstreamLicense === null || Buffer.isBuffer(upstreamLicense), 'lifecycle upstream license carrier');
  const cacheProof = [], licenses = [];
  for (const { pin, cacheRoot } of selection.external) {
    const name = `${pin.name}-${pin.version}`, capture = captures.get(name);
    requireValue(capture?.cacheRoot === cacheRoot, 'lifecycle selected cache correspondence');
    cacheProof.push(validateCompilerCrateCache(capture, pin.crateSha256));
    const local = pin.files.filter(file => file.origin !== UPSTREAM);
    requireValue(local.every(file => file.origin === `https://static.crates.io/crates/${pin.name}/${name}.crate`),
      'lifecycle closed license origins');
    if (local.length) licenses.push(...captureTarGzipMembers(capture.archive, {
      root: name, size: capture.archive.length, sha256: pin.crateSha256,
      files: local.map(file => ({ sourcePath: file.sourcePath, path: file.path, mode: 0o644,
        size: file.size, sha256: file.sha256 })),
    }));
    for (const file of pin.files.filter(file => file.origin === UPSTREAM)) {
      requireValue(Buffer.isBuffer(upstreamLicense) && upstreamLicense.length === file.size
        && sha256(upstreamLicense) === file.sha256, 'lifecycle pinned upstream license');
      licenses.push({ path: file.path, mode: 0o644, data: Buffer.from(upstreamLicense) });
    }
  }
  return { closure: bytes({ format: 'zryna.lifecycle-crate-observations.v1', graph: selection.proof,
    tree: selection.tree, cacheProof, licenses: licenses.map(file => ({ path: file.path,
      bytes: file.data.length, sha256: sha256(file.data) })), sourceAuthenticated: false,
    toolsAuthenticated: false, executionApproved: false, proofQualified: false }), licenses };
}
