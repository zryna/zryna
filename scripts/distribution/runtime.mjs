import { requireValue } from './canonical.mjs';

// These version checks enforce compressor compatibility. The protected producer separately
// authenticates the upstream executable bytes before starting this process.
export function validateArchiveRuntime({ node, zlib, platform, architecture }) {
  requireValue(node === '22.22.1' && zlib === '1.3.1-e00f703', 'archive Node/zlib recipe mismatch');
  requireValue(['linux', 'win32'].includes(platform) && architecture === 'x64',
    'archive builder platform mismatch');
}

export function requireArchiveRuntime() {
  validateArchiveRuntime({ node: process.versions.node, zlib: process.versions.zlib,
    platform: process.platform, architecture: process.arch });
}
