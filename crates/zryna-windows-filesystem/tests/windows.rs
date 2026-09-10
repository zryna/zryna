//! Windows-only exact directory lifecycle tests.

#![cfg(windows)]

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use same_file::Handle;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE,
};
use zryna_windows_filesystem::create_directory;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
const SHARING_VIOLATION: i32 = 32;

#[test]
fn exact_handle_supports_the_complete_directory_lifecycle() -> Result<(), Box<dyn std::error::Error>>
{
    let root = TemporaryRoot::new("lifecycle")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;

    stage.directory().write("artifact.txt", b"artifact")?;
    let source = create_directory(stage.directory(), OsStr::new("src"))?;
    source.directory().write("main.zry", b"export fn main(): i32 { return 7; }\n")?;
    assert_eq!(stage.directory().read("artifact.txt")?, b"artifact");
    assert_eq!(source.directory().read("main.zry")?, b"export fn main(): i32 { return 7; }\n");
    assert_eq!(
        entry_names(stage.directory())?,
        [OsString::from("artifact.txt"), OsString::from("src")]
    );
    assert!(stage.directory().dir_metadata()?.is_dir());

    let reopen = parent.open_dir("stage").expect_err("a second cap directory open must conflict");
    assert_eq!(reopen.raw_os_error(), Some(SHARING_VIOLATION));

    stage.rename_noreplace(&parent, OsStr::new("final"))?;
    assert!(!root.path().join("stage").exists());
    assert_eq!(stage.directory().read("artifact.txt")?, b"artifact");
    assert_eq!(
        entry_names(stage.directory())?,
        [OsString::from("artifact.txt"), OsString::from("src")]
    );

    let retained_identity = Handle::from_file(stage.directory().try_clone()?.into_std_file())?;
    let final_identity = Handle::from_path(root.path().join("final"))?;
    assert_eq!(retained_identity, final_identity);
    drop(retained_identity);
    drop(final_identity);

    stage.rename_noreplace(&parent, OsStr::new("stage"))?;
    assert!(!root.path().join("final").exists());
    assert_eq!(source.directory().read("main.zry")?, b"export fn main(): i32 { return 7; }\n");

    source.directory().remove_file("main.zry")?;
    source.remove_empty()?;
    stage.directory().remove_file("artifact.txt")?;
    stage.remove_empty()?;
    assert!(!root.path().join("stage").exists());
    Ok(())
}

#[test]
fn retained_source_cannot_be_reselected_by_path() -> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("source-substitution")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    stage.directory().write("owned.txt", b"owned")?;
    fs::create_dir(root.path().join("replacement"))?;
    fs::write(root.path().join("replacement/sentinel.txt"), b"foreign")?;

    let rename_error = fs::rename(root.path().join("stage"), root.path().join("displaced"))
        .expect_err("retained source must deny ambient rename");
    assert_eq!(rename_error.raw_os_error(), Some(SHARING_VIOLATION));
    let replace_error = fs::rename(root.path().join("replacement"), root.path().join("stage"))
        .expect_err("replacement cannot be installed over the retained source");
    assert!(replace_error.raw_os_error().is_some());

    stage.rename_noreplace(&parent, OsStr::new("final"))?;
    assert_eq!(stage.directory().read("owned.txt")?, b"owned");
    assert_eq!(fs::read(root.path().join("replacement/sentinel.txt"))?, b"foreign");

    stage.directory().remove_file("owned.txt")?;
    stage.remove_empty()?;
    assert!(!root.path().join("final").exists());
    assert_eq!(fs::read(root.path().join("replacement/sentinel.txt"))?, b"foreign");
    Ok(())
}

#[test]
fn no_replace_preserves_empty_and_nonempty_destinations() -> Result<(), Box<dyn std::error::Error>>
{
    for (label, nonempty) in [("empty", false), ("nonempty", true)] {
        let root = TemporaryRoot::new(label)?;
        let parent = root.open()?;
        let mut stage = create_directory(&parent, OsStr::new("stage"))?;
        stage.directory().write("owned.txt", b"owned")?;
        fs::create_dir(root.path().join("final"))?;
        if nonempty {
            fs::write(root.path().join("final/sentinel.txt"), b"foreign")?;
        }

        let mutation = fs::rename(root.path().join("stage"), root.path().join("displaced"))
            .expect_err("retained source mutation must fail before the collision check");
        assert_eq!(mutation.raw_os_error(), Some(SHARING_VIOLATION));

        stage
            .rename_noreplace(&parent, OsStr::new("final"))
            .expect_err("an existing destination must reject commit");
        assert_eq!(stage.directory().read("owned.txt")?, b"owned");
        assert!(root.path().join("stage").is_dir());
        assert!(root.path().join("final").is_dir());
        if nonempty {
            assert_eq!(fs::read(root.path().join("final/sentinel.txt"))?, b"foreign");
        }

        stage.directory().remove_file("owned.txt")?;
        stage.remove_empty()?;
    }
    Ok(())
}

#[test]
fn rollback_collision_preserves_the_committed_and_foreign_directories()
-> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("rollback-collision")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    stage.directory().write("owned.txt", b"owned")?;
    stage.rename_noreplace(&parent, OsStr::new("final"))?;

    fs::create_dir(root.path().join("stage"))?;
    fs::write(root.path().join("stage/sentinel.txt"), b"foreign")?;
    stage
        .rename_noreplace(&parent, OsStr::new("stage"))
        .expect_err("rollback must not replace a foreign stage-name collision");
    assert_eq!(stage.directory().read("owned.txt")?, b"owned");
    assert_eq!(fs::read(root.path().join("stage/sentinel.txt"))?, b"foreign");

    stage.directory().remove_file("owned.txt")?;
    stage.remove_empty()?;
    assert!(!root.path().join("final").exists());
    assert_eq!(fs::read(root.path().join("stage/sentinel.txt"))?, b"foreign");
    Ok(())
}

#[test]
fn cleanup_is_exact_and_failure_closes_the_handle() -> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("cleanup")?;
    let parent = root.open()?;
    let stage = create_directory(&parent, OsStr::new("stage"))?;
    stage.directory().write("sentinel.txt", b"owned")?;

    stage.remove_empty().expect_err("nonempty exact directory must be preserved");
    assert_eq!(fs::read(root.path().join("stage/sentinel.txt"))?, b"owned");

    fs::remove_file(root.path().join("stage/sentinel.txt"))?;
    fs::remove_dir(root.path().join("stage"))?;

    fs::create_dir(root.path().join("collision"))?;
    create_directory(&parent, OsStr::new("collision"))
        .expect_err("atomic create must preserve an existing directory");
    fs::remove_dir(root.path().join("collision"))?;
    Ok(())
}

#[test]
fn rejects_non_component_and_unbounded_names() -> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("names")?;
    let parent = root.open()?;
    let maximum = "a".repeat(255);
    let accepted = create_directory(&parent, OsStr::new(&maximum))?;
    accepted.remove_empty()?;
    let too_long = "a".repeat(256);
    let invalid = [
        OsStr::new(""),
        OsStr::new("."),
        OsStr::new(".."),
        OsStr::new("../stage"),
        OsStr::new("stage\\child"),
        OsStr::new("C:stage"),
        OsStr::new("stage."),
        OsStr::new("NUL"),
        OsStr::new("COM1.txt"),
        OsStr::new("stage name"),
        OsStr::new("café"),
        OsStr::new(&too_long),
    ];
    for name in invalid {
        let error = create_directory(&parent, name)
            .expect_err("unsafe component must fail before filesystem access");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
    assert!(entry_names(&parent)?.is_empty());
    Ok(())
}

#[test]
fn opaque_source_rejects_an_existing_regular_file() -> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("regular-file")?;
    let parent = root.open()?;
    fs::write(root.path().join("stage"), b"foreign")?;

    create_directory(&parent, OsStr::new("stage"))
        .expect_err("a regular file cannot become an owned directory capability");
    assert_eq!(fs::read(root.path().join("stage"))?, b"foreign");
    Ok(())
}

#[test]
fn cleanup_reports_an_extra_share_delete_handle() -> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("pending-delete")?;
    let parent = root.open()?;
    let stage = create_directory(&parent, OsStr::new("stage"))?;
    let extra = fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(root.path().join("stage"))?;

    stage.remove_empty().expect_err("delete-pending is not confirmed cleanup");
    drop(extra);
    assert!(!root.path().join("stage").exists());
    Ok(())
}

fn entry_names(directory: &Dir) -> io::Result<Vec<OsString>> {
    let mut names = directory
        .entries()?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<Vec<_>>>()?;
    names.sort();
    Ok(names)
}

struct TemporaryRoot {
    path: PathBuf,
}

impl TemporaryRoot {
    fn new(label: &str) -> io::Result<Self> {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("zryna-windows-filesystem-{label}-{}-{sequence}", std::process::id()));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn open(&self) -> io::Result<Dir> {
        Dir::open_ambient_dir(&self.path, ambient_authority())
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
