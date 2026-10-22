use super::{NodeRuntimeCapability, is_pinned_node_version};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) struct RuntimeRoot {
    path: PathBuf,
}

impl RuntimeRoot {
    pub(super) fn new(label: &str) -> Self {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "zryna runtime {label} & literal $HOME ; %PATH% {}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("runtime fixture root must be created");
        Self { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn harness(&self, label: &str, source: &str) -> PathBuf {
        let path = self.path.join(format!("{label} & literal $HOME ; %PATH%.mjs"));
        fs::write(&path, source).expect("runtime harness must be written");
        path
    }
}

impl Drop for RuntimeRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    let candidate = ["ZRYNA_TEST_NODE", "NODE"]
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
        .expect("Node.js must be installed for runtime tests")
        .canonicalize()
        .expect("Node.js executable must canonicalize");
    let version = crate::process_spawn::output(Command::new(&candidate).arg("--version"))
        .expect("Node.js version probe must start");
    assert!(version.status.success());
    assert!(is_pinned_node_version(&version.stdout));
    assert!(version.stderr.is_empty());
    candidate
}

pub(super) fn capability(root: &RuntimeRoot) -> NodeRuntimeCapability {
    NodeRuntimeCapability::discover(&node_executable(), root.path())
        .expect("documented Node.js runtime must validate")
}
