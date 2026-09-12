import { LIMITS, orderedPaths, portablePath, requireValue } from './canonical.mjs';

export function archiveEntries(root, files) {
  portablePath(root);
  requireValue(!root.includes('/'), 'archive root must be one segment');
  orderedPaths(files.map(file => file.path));
  let total = 0;
  const directories = new Set([`${root}/`]);
  const entries = [];
  for (const file of files) {
    requireValue(Buffer.isBuffer(file.data) && file.data.length <= LIMITS.binary,
      'archive file byte budget');
    requireValue(file.mode === 0o644 || file.mode === 0o755, 'archive file mode');
    total += file.data.length;
    requireValue(total <= LIMITS.expanded, 'archive expansion budget');
    const path = `${root}/${file.path}`;
    const segments = path.split('/');
    for (let index = 1; index < segments.length; index++) {
      directories.add(`${segments.slice(0, index).join('/')}/`);
    }
    entries.push({ path, mode: file.mode, data: file.data, directory: false });
  }
  for (const path of directories) {
    entries.push({ path, mode: 0o755, data: Buffer.alloc(0), directory: true });
  }
  return entries.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
}

export function decodedFiles(root, entries) {
  requireValue(entries.length <= LIMITS.files * LIMITS.depth + 1, 'archive entry count');
  const files = [];
  for (const entry of entries) {
    requireValue(entry.path.startsWith(`${root}/`), 'unexpected archive root');
    if (!entry.directory) {
      files.push({ path: entry.path.slice(root.length + 1), mode: entry.mode, data: entry.data });
    }
  }
  // Re-encoding by the caller verifies exact ancestor directories and ordering too.
  archiveEntries(root, files);
  return files;
}
