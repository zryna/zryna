//! Complete bundle byte inventory for independent conformance comparisons.

use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn inventory(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(base: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).expect("inventory") {
            let path = entry.expect("entry").path();
            let metadata = fs::symlink_metadata(&path).expect("metadata");
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                visit(base, &path, files);
            } else {
                assert!(metadata.is_file());
                files.insert(
                    path.strip_prefix(base)
                        .expect("contained path")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).expect("bytes"),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
