use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::InstallationTree;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Case {
    parent: PathBuf,
    root: PathBuf,
}

impl Case {
    fn new() -> Self {
        let parent = loop {
            let path = std::env::temp_dir().join(format!(
                "zryna-installation-files-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            match fs::create_dir(&path) {
                Ok(()) => break path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("private fixture directory: {error}"),
            }
        };
        let root = parent.join("installation");
        fs::create_dir(&root).expect("installation directory");
        fs::create_dir(root.join("provider")).expect("provider directory");
        fs::write(root.join("provider/worker.mjs"), b"original").expect("provider bytes");
        Self { parent, root }
    }

    fn capture(&self) -> InstallationTree {
        let mut tree = InstallationTree::capture(&self.root).expect("real installation root");
        tree.capture_file("provider/worker.mjs", 8, true).expect("bounded provider capture");
        tree.revalidate().expect("exact original inventory");
        tree
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.parent).expect("remove exclusively created fixture directory");
    }
}

#[test]
fn captured_provider_bytes_are_retained_and_unknown_names_are_unavailable() {
    let case = Case::new();
    let tree = case.capture();
    assert_eq!(tree.bytes("provider/worker.mjs").expect("retained bytes"), b"original");
    assert!(tree.bytes("provider/extra.mjs").is_err());
    assert!(tree.bytes("../worker.mjs").is_err());
}

#[test]
fn same_length_provider_mutation_is_rejected_or_prevented_by_retained_handle() {
    let case = Case::new();
    let tree = case.capture();
    match fs::write(case.root.join("provider/worker.mjs"), b"modified") {
        Ok(()) => assert!(tree.revalidate().is_err()),
        Err(_) => tree.revalidate().expect("unchanged protected file"),
    }
    assert_eq!(tree.bytes("provider/worker.mjs").expect("original bytes remain"), b"original");
}

#[test]
fn replacement_with_identical_bytes_does_not_preserve_file_identity() {
    let case = Case::new();
    let tree = case.capture();
    let original = case.root.join("provider/worker.mjs");
    let moved = case.parent.join("retained-worker.mjs");
    match fs::rename(&original, &moved) {
        Ok(()) => {
            fs::write(&original, b"original").expect("same-byte replacement");
            assert!(tree.revalidate().is_err());
        }
        Err(_) => tree.revalidate().expect("rename was prevented"),
    }
}

#[test]
fn extra_file_or_empty_directory_fails_complete_inventory_admission() {
    for directory in [false, true] {
        let case = Case::new();
        let tree = case.capture();
        let extra = case.root.join("provider/extra");
        if directory {
            fs::create_dir(extra).expect("extra directory");
        } else {
            fs::write(extra, b"extra").expect("extra file");
        }
        assert!(tree.revalidate().is_err());
    }
}

#[test]
fn hard_linked_provider_is_rejected_before_capture() {
    let case = Case::new();
    fs::hard_link(case.root.join("provider/worker.mjs"), case.parent.join("alias.mjs"))
        .expect("hard link on fixture volume");
    let mut tree = InstallationTree::capture(&case.root).expect("real root");
    assert!(tree.capture_file("provider/worker.mjs", 8, true).is_err());
}

#[test]
fn first_byte_over_capture_budget_is_rejected_with_and_without_retention() {
    for retain in [false, true] {
        let case = Case::new();
        let mut tree = InstallationTree::capture(&case.root).expect("real root");
        assert!(tree.capture_file("provider/worker.mjs", 7, retain).is_err());
    }
}

#[test]
fn approved_rust_notice_version_build_metadata_remains_a_portable_path() {
    assert!(super::super::manifest::portable(
        "licenses/rust/toml-0.9.12+spec-1.1.0/LICENSE-APACHE"
    ));
    assert!(!super::super::manifest::portable("licenses/rust/../LICENSE-APACHE"));
    assert!(!super::super::manifest::portable("licenses/rust/CON.txt/LICENSE-APACHE"));
}

#[test]
fn replacing_an_ancestor_cannot_redirect_the_retained_installation() {
    let case = Case::new();
    let tree = case.capture();
    match fs::rename(&case.root, case.parent.join("old-installation")) {
        Ok(()) => {
            fs::create_dir_all(case.root.join("provider")).expect("replacement tree");
            fs::write(case.root.join("provider/worker.mjs"), b"original").expect("same bytes");
            assert!(tree.revalidate().is_err());
        }
        Err(_) => tree.revalidate().expect("ancestor rename was prevented"),
    }
}

#[cfg(unix)]
#[test]
fn symbolic_file_and_directory_links_are_rejected_without_following_them() {
    for directory in [false, true] {
        let case = Case::new();
        let outside = case.parent.join("outside");
        fs::create_dir(&outside).expect("outside directory");
        fs::write(outside.join("worker.mjs"), b"original").expect("outside bytes");
        if directory {
            fs::remove_file(case.root.join("provider/worker.mjs")).expect("remove fixture file");
            fs::remove_dir(case.root.join("provider")).expect("remove fixture directory");
            std::os::unix::fs::symlink(&outside, case.root.join("provider"))
                .expect("directory link");
        } else {
            fs::remove_file(case.root.join("provider/worker.mjs")).expect("remove fixture file");
            std::os::unix::fs::symlink(
                outside.join("worker.mjs"),
                case.root.join("provider/worker.mjs"),
            )
            .expect("file link");
        }
        let mut tree = InstallationTree::capture(&case.root).expect("real root");
        assert!(tree.capture_file("provider/worker.mjs", 8, true).is_err());
        assert_eq!(fs::read(outside.join("worker.mjs")).expect("outside preserved"), b"original");
    }
}

#[cfg(unix)]
#[test]
fn socket_entries_are_rejected_as_nonregular_files() {
    let case = Case::new();
    let path = case.root.join("provider/worker.mjs");
    fs::remove_file(&path).expect("remove fixture file");
    let _socket = std::os::unix::net::UnixListener::bind(path).expect("fixture socket");
    let mut tree = InstallationTree::capture(&case.root).expect("real root");
    assert!(tree.capture_file("provider/worker.mjs", 8, true).is_err());
}
