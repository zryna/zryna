use super::*;
use cap_std::ambient_authority;
use same_file::Handle;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn diagnostic_only_reports_open_descendant_rename_outcomes()
-> Result<(), Box<dyn std::error::Error>> {
    let outcomes = [
        no_descendants()?,
        child_without_delete_sharing()?,
        child_with_delete_sharing()?,
        retained_project_topology()?,
    ];
    let report = outcomes.iter().map(Outcome::report).collect::<Vec<_>>().join("; ");

    println!("diagnostic-only open-descendant matrix (not a contract assertion): {report}");
    Ok(())
}

fn no_descendants() -> io::Result<Outcome> {
    let root = TemporaryRoot::new("diagnostic-none")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    Ok(Outcome::new("no descendants", stage.rename_noreplace(&parent, OsStr::new("final"))))
}

fn child_without_delete_sharing() -> io::Result<Outcome> {
    let root = TemporaryRoot::new("diagnostic-child-exclusive")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    let _child = create_directory(stage.directory(), OsStr::new("src"))?;
    Ok(Outcome::new(
        "child without delete sharing",
        stage.rename_noreplace(&parent, OsStr::new("final")),
    ))
}

fn child_with_delete_sharing() -> io::Result<Outcome> {
    let root = TemporaryRoot::new("diagnostic-child-shared")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    let _child = create_test_directory(
        stage.directory(),
        OsStr::new("src"),
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    Ok(Outcome::new(
        "child with delete sharing",
        stage.rename_noreplace(&parent, OsStr::new("final")),
    ))
}

fn retained_project_topology() -> io::Result<Outcome> {
    let root = TemporaryRoot::new("diagnostic-project")?;
    let parent = root.open()?;
    let mut stage = create_directory(&parent, OsStr::new("stage"))?;
    let source = create_directory(stage.directory(), OsStr::new("src"))?;
    let _manifest = retain_file(stage.directory(), "zryna.package.json", b"manifest")?;
    let _lock = retain_file(stage.directory(), "zryna.lock.json", b"lock")?;
    let _source = retain_file(source.directory(), "main.zry", b"source")?;

    Ok(Outcome::new(
        "project topology with three file identities and directory clones",
        stage.rename_noreplace(&parent, OsStr::new("final")),
    ))
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

struct Outcome {
    label: &'static str,
    result: io::Result<()>,
}

impl Outcome {
    fn new(label: &'static str, result: io::Result<()>) -> Self {
        Self { label, result }
    }

    fn report(&self) -> String {
        match &self.result {
            Ok(()) => format!("{}=success", self.label),
            Err(error) => format!(
                "{}=error(kind={:?}, raw={:?}, message={error})",
                self.label,
                error.kind(),
                error.raw_os_error()
            ),
        }
    }
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

    fn open(&self) -> io::Result<Dir> {
        Dir::open_ambient_dir(&self.path, ambient_authority())
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
