use serde_json::Value;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
use zryna_driver::{DataOwnershipBuildRequest, TargetSelection};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn registry() -> Value {
    serde_json::from_str(include_str!("../../../../tests/m3-conformance-v1.json"))
        .expect("public M3 fixture invariant")
}
pub struct Case {
    pub root: PathBuf,
    pub source: String,
    pub stem: String,
    pub node: PathBuf,
}
impl Case {
    pub fn new(fixture: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("public M3 fixture invariant");
        let stem =
            format!("m3_public_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
        let source = format!(".zryna/cache/{stem}/main.zry");
        fs::create_dir_all(root.join(&source).parent().expect("public M3 fixture invariant"))
            .expect("public M3 fixture invariant");
        let reg = registry();
        let entry = reg["fixtures"]
            .as_array()
            .expect("public M3 fixture invariant")
            .iter()
            .find(|f| f["id"] == fixture)
            .expect("public M3 fixture invariant");
        fs::copy(
            root.join(entry["path"].as_str().expect("public M3 fixture invariant")),
            root.join(&source),
        )
        .expect("public M3 fixture invariant");
        if let Some(dependency) = entry["dependency"].as_str() {
            let other = reg["fixtures"]
                .as_array()
                .expect("public M3 fixture invariant")
                .iter()
                .find(|f| f["id"] == dependency)
                .expect("public M3 fixture invariant");
            fs::copy(
                root.join(other["path"].as_str().expect("public M3 fixture invariant")),
                root.join(&source).with_file_name("math.zry"),
            )
            .expect("public M3 fixture invariant");
        }
        Self { root, source, stem, node: node_executable() }
    }
    pub fn bundle(&self, command: &str) -> PathBuf {
        self.root.join(".zryna/out").join(format!("{}.{command}", self.stem))
    }
    pub fn run(&self, command: &str, target: &str, extra: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_zryna"))
            .args([
                command,
                &self.source,
                "--profile",
                "data-ownership-v1",
                "--target",
                target,
                "--name",
                &self.stem,
                "--json",
                "--root",
            ])
            .arg(&self.root)
            .arg("--node")
            .arg(&self.node)
            .args(extra)
            .output()
            .expect("public M3 fixture invariant")
    }
    pub fn request(&self, targets: TargetSelection) -> DataOwnershipBuildRequest {
        DataOwnershipBuildRequest {
            workspace_root: self.root.clone(),
            entrypoint: self.source.clone(),
            artifact_stem: self.stem.clone(),
            targets,
            node_runtime: self.node.clone(),
        }
    }
    pub fn clear(&self, command: &str) {
        fs::remove_dir_all(self.bundle(command)).expect("public M3 fixture invariant");
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        for command in ["build", "run"] {
            let _ = fs::remove_dir_all(self.bundle(command));
        }
        let _ = fs::remove_dir_all(
            self.root.join(&self.source).parent().expect("public M3 fixture invariant"),
        );
    }
}
pub fn successful(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("public M3 fixture invariant");
    assert_eq!(value["ok"], true);
    value
}
pub fn invocation(case: &Value) -> Vec<String> {
    let mut result = vec![
        "--export".to_owned(),
        case["export"].as_str().expect("public M3 fixture invariant").to_owned(),
    ];
    result.extend(
        case["arguments"]
            .as_array()
            .expect("public M3 fixture invariant")
            .iter()
            .map(|arg| format!("--arg=i32:{arg}")),
    );
    result
}
pub fn inventory(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("public M3 fixture invariant") {
            let path = entry.expect("public M3 fixture invariant").path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .expect("public M3 fixture invariant")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).expect("public M3 fixture invariant"),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
pub fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    let node = ["ZRYNA_TEST_NODE", "NODE"]
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
        .expect("Node.js must be installed")
        .canonicalize()
        .expect("Node.js executable must canonicalize");
    let version = Command::new(&node).arg("--version").output().expect("Node probe must start");
    assert!(version.status.success());
    assert!(matches!(version.stdout.as_slice(), b"v22.22.1\n" | b"v22.22.1\r\n"));
    assert!(version.stderr.is_empty());
    node
}

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn guard() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
