//! Shared held ordinary fixture workspace and exact stage cleanup.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) struct Workspace {
    pub(super) path: PathBuf,
    pub(super) root: native::ArtifactOutputRoot,
    _held: export::Directory,
}
impl Workspace {
    pub(super) fn new(parent: &Path) -> Result<Self, Diagnostic> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            "rust-ordinary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let (path, held) = export::create(parent, &name)?;
        let (private, _) = export::create(&path, ".zryna")?;
        export::create(&private, "out")?;
        let root = native::ArtifactOutputRoot::for_workspace(&path)?;
        held.revalidate()?;
        Ok(Self { path, root, _held: held })
    }
    pub(super) fn stages(&self, count: usize) -> Result<Vec<native::NativeStage>, Diagnostic> {
        let mut result = Vec::new();
        for _ in 0..count {
            match native::NativeStage::create(&self.root, "rust-ordinary") {
                Ok(value) => result.push(value),
                Err(error) => {
                    for stage in &result {
                        let _cleanup = stage.cleanup();
                    }
                    return Err(error);
                }
            }
        }
        Ok(result)
    }
}
pub(super) fn cleanup(
    session: &mut Session<'_>,
    stages: &[native::NativeStage],
    extras: &[(usize, &str)],
) {
    for &(index, name) in extras {
        let stage = &stages[index];
        if let Err(error) = stage.revalidate() {
            session.diagnostics.push(error);
            continue;
        }
        if let Err(error) = stage.directory_handle.remove_file(name)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            session.diagnostics.push(rejected());
        }
    }
    for stage in stages {
        session.diagnostics.extend(stage.cleanup());
    }
}
