// Compiler-only publication authority; independent of the proposed outer toolkit format.
import { LIMITS, bytes, exactKeys, requireValue, sha256 } from '../../../../scripts/distribution/canonical.mjs';

export { LIMITS, bytes, exactKeys, requireValue, sha256 };
export const product = 'zryna-playground-compiler';
export const productVersion = '0.1.0';
export const target = 'x86_64-unknown-linux-gnu';
export const profile = 'restricted-browser-v1';
export const sourceRef = 'refs/tags/v0.1.0-playground-compiler.3';
export const repository = 'https://github.com/zryna/zryna';
export const workflowPath = '.github/workflows/playground-compiler-release.yml';
export const issuer = 'https://token.actions.githubusercontent.com';
export const certificateIdentity = `${repository}/${workflowPath}@${sourceRef}`;
export const archiveRoot = `${product}-${productVersion}-${target}`;
export const archivePath = `${archiveRoot}.tar.gz`;
export const binaryPath = `bin/${product}`;
export const metadataPaths = Object.freeze({ buildReceipt: 'metadata/build-receipt.json',
  rustMaterials: 'metadata/rust-materials.json', sbom: 'metadata/compiler.spdx.json' });
export const inputPaths = Object.freeze({ cargoLock: 'Cargo.lock', workspaceCargo: 'Cargo.toml',
  packageCargo: `apps/${product}/Cargo.toml`, pnpmLock: 'pnpm-lock.yaml',
  rustToolchain: 'rust-toolchain.toml', workspaceManifest: 'zryna.workspace.json' });
export const verifierPins = Object.freeze({ version: '3.0.5', bytes: 135112256,
  sha256: 'db15cc99e6e4837daabab023742aaddc3841ce57f193d11b7c3e06c8003642b2',
  trustedRootBytes: 6787,
  trustedRootSha256: '6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66' });

export const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export const gitId = value => typeof value === 'string' && /^[a-f0-9]{40}$/.test(value);
export const equal = (left, right) => bytes(left).equals(bytes(right));
export const positive = (value, maximum = Number.MAX_SAFE_INTEGER) =>
  Number.isSafeInteger(value) && value > 0 && value <= maximum;

export function validateSource(source) {
  exactKeys(source, ['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch', 'tagObject', 'tagType']);
  requireValue(source.repository === repository && source.ref === sourceRef &&
    gitId(source.commit) && gitId(source.tree) && gitId(source.tagObject) &&
    source.tagObject !== source.commit && source.tagType === 'annotated' &&
    Number.isSafeInteger(source.sourceDateEpoch) && source.sourceDateEpoch >= 0 &&
    source.sourceDateEpoch <= 0xffffffff, 'compiler publication source');
}

export function descriptor(value, expectedPath, maximum = LIMITS.text) {
  exactKeys(value, ['path', 'bytes', 'sha256']);
  requireValue(value.path === expectedPath && positive(value.bytes, maximum) && hash(value.sha256),
    'compiler publication descriptor');
}
