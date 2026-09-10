import { exactKeys, parseCanonical, requireValue } from './canonical.mjs';
import { digestValue } from './inventory.mjs';

// This is an installed-content consistency check. The protected workflow establishes that
// the source command actually ran; a receipt submitted by an arbitrary producer is no proof.
export function validateSourceReceipt(input, distribution) {
  const receipt = parseCanonical(input);
  exactKeys(receipt, ['format', 'source', 'command', 'toolchain', 'inputs', 'report']);
  requireValue(receipt.format === 'zryna.source-build-receipt.v1', 'source receipt format');
  exactKeys(receipt.source, ['repository', 'commit', 'tree']);
  for (const key of ['repository', 'commit', 'tree']) {
    requireValue(receipt.source[key] === distribution.source[key], 'source receipt identity');
  }
  requireValue(Array.isArray(receipt.command) && receipt.command.join('\0')
    === ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'].join('\0'),
  'source architecture command');
  exactKeys(receipt.toolchain, ['channel', 'cargoVersion', 'rustcVersion']);
  requireValue(receipt.toolchain.channel === '1.97.1', 'source receipt toolchain');
  for (const tool of ['cargo', 'rustc']) {
    const version = receipt.toolchain[`${tool}Version`];
    requireValue(typeof version === 'string' && version.startsWith(`${tool} 1.97.1 `)
      && version.length <= 96 && /^[ -~]+$/.test(version), 'source tool version');
  }
  const paths = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json'];
  requireValue(Array.isArray(receipt.inputs) && receipt.inputs.length === paths.length,
    'source receipt inputs');
  for (let index = 0; index < paths.length; index++) {
    const entry = receipt.inputs[index];
    exactKeys(entry, ['logicalPath', 'size', 'sha256']);
    requireValue(entry.logicalPath === paths[index] && Number.isSafeInteger(entry.size)
      && entry.size >= 1 && entry.size <= 262144, 'source input identity');
    digestValue(entry.sha256);
  }
  exactKeys(receipt.report, ['diagnostics']);
  requireValue(Array.isArray(receipt.report.diagnostics) && receipt.report.diagnostics.length === 0,
    'source architecture result');
  return receipt;
}
