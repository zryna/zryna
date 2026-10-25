//! Test-only explicit Node admission and real held output root, with ordinary cleanup.
use crate::{ArtifactOutputRoot, runtime::NodeRuntimeCapability};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Root {
    path: PathBuf,
}
impl Root {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "m416-generic-run-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("new test-owned workspace");
        Self { path: path.canonicalize().expect("ordinary absolute root") }
    }
    pub(super) fn output(&self) -> ArtifactOutputRoot {
        ArtifactOutputRoot::prepare_for_workspace(&self.path).expect("real output capability")
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
pub(super) fn node(root: &Root) -> NodeRuntimeCapability {
    let filename = if cfg!(windows) { "node.exe" } else { "node" };
    let executable = ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .chain(
            std::env::var_os("PATH")
                .into_iter()
                .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
                .map(|path| path.join(filename)),
        )
        .find(|path| path.is_file())
        .expect("existing documented test Node");
    NodeRuntimeCapability::discover(
        &executable.canonicalize().expect("ordinary Node path"),
        &root.path,
    )
    .expect("existing bounded Node admission")
}
