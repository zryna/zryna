use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::publish_with_checkpoint;

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Case {
    root: PathBuf,
}

impl Case {
    fn new(label: &str) -> Self {
        let sequence = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("zryna-project-fs-{label}-{}-{sequence}", std::process::id()));
        fs::create_dir(&root).expect("fixture root");
        Self { root }
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let parent = self.root.join("parent");
        if parent.symlink_metadata().is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            let _ = fs::remove_file(parent);
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn parent_replacement_cannot_redirect_creation_or_cleanup() {
    use std::os::unix::fs::symlink;

    let case = Case::new("parent-replacement");
    let parent = case.root.join("parent");
    let moved = case.root.join("moved-parent");
    let foreign = case.root.join("foreign");
    fs::create_dir(&parent).expect("parent");
    fs::create_dir(&foreign).expect("foreign");
    fs::write(foreign.join("sentinel"), b"preserve").expect("foreign sentinel");
    let destination = parent.join("example");
    let error =
        publish_with_checkpoint(&destination, "example", b"manifest", b"lock", b"source", || {
            fs::rename(&parent, &moved).expect("move retained parent");
            symlink(&foreign, &parent).expect("replacement parent link");
        })
        .expect_err("parent replacement must reject");
    assert_eq!(error.code, "ZRYNA-C2003");
    assert_eq!(fs::read(foreign.join("sentinel")).expect("sentinel"), b"preserve");
    assert!(!foreign.join("example").exists());
    assert!(!moved.join(".zryna-new-example.pending").exists());
}

#[test]
fn cleanup_never_removes_injected_foreign_stage_content() {
    let case = Case::new("foreign-stage");
    let parent = case.root.join("parent");
    fs::create_dir(&parent).expect("parent");
    let destination = parent.join("example");
    let stage = parent.join(".zryna-new-example.pending");
    let error =
        publish_with_checkpoint(&destination, "example", b"manifest", b"lock", b"source", || {
            fs::create_dir(&destination).expect("racing destination");
            fs::write(stage.join("foreign"), b"preserve").expect("foreign stage content");
        })
        .expect_err("collision cleanup with foreign content must fail closed");
    assert_eq!(error.code, "ZRYNA-C2004");
    assert_eq!(fs::read(stage.join("foreign")).expect("foreign stage content"), b"preserve");
    assert!(destination.is_dir());
}
