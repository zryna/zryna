// Select from the already pinned candidate records, never a latest-version metadata search.
import { bytes, requireValue } from './canonical.mjs';

export function rustNoticeOwner(record, file, selected, metadata) {
  requireValue(Array.isArray(selected) && selected.some(item => bytes(item).equals(bytes(record)))
    && record.files.some(item => bytes(item).equals(bytes(file))), 'Rust selected notice membership');
  let owner = record;
  if (/^https:\/\/raw\.githubusercontent\.com\/bytecodealliance\/wasmtime\//.test(file.origin)) {
    requireValue(/^https:\/\/raw\.githubusercontent\.com\/bytecodealliance\/wasmtime\/[a-f0-9]{40}\/LICENSE$/.test(file.origin)
      && file.sourcePath === 'LICENSE', 'Rust upstream notice origin');
    const owners = selected.filter(item => item.name === 'wasmtime'
      && item.files.some(notice => notice.sourcePath === file.sourcePath
        && notice.size === file.size && notice.sha256 === file.sha256));
    requireValue(owners.length === 1, 'Rust exact selected upstream notice owner');
    [owner] = owners;
  } else {
    requireValue(file.origin === `https://static.crates.io/crates/${record.name}/${record.name}-${record.version}.crate`,
      'Rust exact selected crate notice origin');
  }
  const packages = metadata.packages.filter(item => item.name === owner.name
    && item.version === owner.version
    && item.source === 'registry+https://github.com/rust-lang/crates.io-index');
  requireValue(packages.length === 1, 'Rust exact selected notice metadata owner');
  return packages[0];
}
