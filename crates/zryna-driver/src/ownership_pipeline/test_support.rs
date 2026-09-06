//! Isolated fixture and serialization support for candidate-route tests.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{
        MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use super::OWNERSHIP_ROUTE_TEST_LOCK;

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct FixtureWorkspace {
    root: PathBuf,
}

impl FixtureWorkspace {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for FixtureWorkspace {
    fn drop(&mut self) {
        if self.root.exists() {
            fs::remove_dir_all(&self.root).expect("candidate fixture workspace cleanup");
        }
    }
}

pub(crate) fn route_guard() -> MutexGuard<'static, ()> {
    OWNERSHIP_ROUTE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn fixture_workspace() -> FixtureWorkspace {
    let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
    let root =
        env::temp_dir().join(format!("zryna-ownership-driver-{}-{sequence}", std::process::id()));
    fs::create_dir(&root).expect("candidate fixture workspace");
    let source = repository_root().join("tests/m3-fixtures/candidate-modules");
    for name in ["main.zry", "math.zry"] {
        fs::copy(source.join(name), root.join(name)).expect("candidate fixture source copy");
    }
    FixtureWorkspace { root }
}

pub(crate) fn adapter_root() -> PathBuf {
    repository_root().join("adapters/typescript-6")
}

pub(crate) fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .chain(
            env::var_os("PATH")
                .into_iter()
                .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
                .map(move |directory| directory.join(executable)),
        )
        .find(|path| path.is_file())
        .expect("Node.js")
        .canonicalize()
        .expect("canonical Node.js")
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repository root")
}
