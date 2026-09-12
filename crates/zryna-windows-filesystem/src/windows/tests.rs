use super::*;
use cap_std::ambient_authority;
use same_file::Handle;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn ancestor_rename_rejects_a_live_child_regardless_of_delete_sharing()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, share_access) in [
        ("exclusive", FILE_SHARE_READ | FILE_SHARE_WRITE),
        ("share-delete", FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE),
    ] {
        let root = TemporaryRoot::new(label)?;
        let parent = root.open()?;
        let mut stage = create_directory(&parent, OsStr::new("stage"))?;
        let child = create_test_directory(stage.directory(), OsStr::new("src"), share_access)?;

        let error = stage
            .rename_noreplace(&parent, OsStr::new("final"))
            .expect_err("a live descendant must reject the ancestor rename");
        assert_eq!(error.raw_os_error(), Some(ERROR_ACCESS_DENIED as i32));
        assert!(root.path().join("stage").is_dir());
        assert!(!root.path().join("final").exists());

        drop(child);
        stage.rename_noreplace(&parent, OsStr::new("final"))?;
        stage.directory().remove_dir("src")?;
        stage.remove_empty()?;
    }
    Ok(())
}

#[test]
fn project_topology_renames_only_after_every_descendant_handle_closes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = TemporaryRoot::new("project-topology")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    let source = create_directory(stage.directory(), OsStr::new("src"))?;
    let manifest = retain_file(stage.directory(), "zryna.package.json", b"manifest")?;
    let lock = retain_file(stage.directory(), "zryna.lock.json", b"lock")?;
    let source_file = retain_file(source.directory(), "main.zry", b"source")?;

    let error = stage
        .rename_noreplace(&parent, OsStr::new("final"))
        .expect_err("the complete live project topology must reject the ancestor rename");
    assert_eq!(error.raw_os_error(), Some(ERROR_ACCESS_DENIED as i32));
    assert!(root.path().join("stage").is_dir());
    assert!(!root.path().join("final").exists());

    drop(source_file);
    drop(source);
    drop(lock);
    drop(manifest);

    fs::create_dir(root.path().join("collision"))?;
    fs::write(root.path().join("collision/sentinel"), b"foreign")?;
    let collision = stage
        .rename_noreplace(&parent, OsStr::new("collision"))
        .expect_err("the exact root must not replace a foreign destination");
    assert_eq!(collision.raw_os_error(), Some(ERROR_ALREADY_EXISTS as i32));
    assert_eq!(fs::read(root.path().join("collision/sentinel"))?, b"foreign");

    stage.rename_noreplace(&parent, OsStr::new("final"))?;
    assert!(!root.path().join("stage").exists());
    let retained = Handle::from_file(stage.directory().try_clone()?.into_std_file())?;
    let published = Handle::from_path(root.path().join("final"))?;
    assert_eq!(retained, published);
    drop(retained);
    drop(published);

    assert_eq!(stage.directory().read("zryna.package.json")?, b"manifest");
    assert_eq!(stage.directory().read("zryna.lock.json")?, b"lock");
    let source = stage.directory().open_dir("src")?;
    assert_eq!(source.read("main.zry")?, b"source");
    source.remove_file("main.zry")?;
    drop(source);
    stage.directory().remove_dir("src")?;
    stage.directory().remove_file("zryna.lock.json")?;
    stage.directory().remove_file("zryna.package.json")?;
    stage.remove_empty()?;
    assert_eq!(fs::read(root.path().join("collision/sentinel"))?, b"foreign");
    Ok(())
}

fn create_test_directory(
    parent: &Dir,
    name: &OsStr,
    share_access: u32,
) -> io::Result<OwnedDirectory> {
    let name = encode_component(name)?;
    let retained_parent = parent.try_clone()?;
    let source = open_relative(
        parent.as_handle(),
        &name,
        GENERIC_READ | DELETE | SYNCHRONIZE,
        share_access,
        FILE_CREATE,
    )?;
    Ok(OwnedDirectory { directory: Dir::from_std_file(source), parent: retained_parent, name })
}

fn retain_file(directory: &Dir, name: &str, bytes: &[u8]) -> io::Result<RetainedFile> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = directory.open_with(name, &options)?;
    file.write_all(bytes)?;
    let identity = Handle::from_file(file.into_std())?;
    Ok(RetainedFile { _directory: directory.try_clone()?, _identity: identity })
}

struct RetainedFile {
    _directory: Dir,
    _identity: Handle,
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
