//! Real public executable, byte-pinned original sources and owned checkout fixture paths.
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT: AtomicU64 = AtomicU64::new(0);
static SERIAL: Mutex<()> = Mutex::new(());
pub fn guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub fn pins() -> Value {
    serde_json::from_str(include_str!("PRE_EXECUTION.json"))
        .expect("frozen source-only expectations")
}
pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("source checkout")
}
pub fn original(row: &Value) -> Vec<u8> {
    let bytes = fs::read(repo().join(row["path"].as_str().expect("original path")))
        .expect("original source");
    assert_eq!(u64::try_from(bytes.len()).expect("bounded source bytes"), row["bytes"]);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
    bytes
}

pub struct Case {
    pub root: PathBuf,
    pub directory: String,
    pub source: String,
    pub stem: String,
}
impl Case {
    pub fn new(fixture: &str) -> Self {
        let root = repo();
        let stem = format!(
            "generic_public_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let directory = format!(".zryna/cache/{stem}");
        fs::create_dir_all(root.join(&directory)).expect("owned fixture directory");
        fs::create_dir_all(root.join(".zryna/out")).expect("output exists before source census");
        for row in pins()["fixtures"][fixture]["files"].as_array().expect("frozen files") {
            fs::write(
                root.join(&directory).join(row["member"].as_str().expect("member")),
                original(row),
            )
            .expect("exact source copy without rewriting");
        }
        let source = format!("{directory}/main.zry");
        Self { root, directory, source, stem }
    }
    pub fn command(
        &self,
        action: &str,
        target: &str,
        equals: bool,
        provider: &str,
        node: Option<&Path>,
    ) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_zryna"));
        command
            .args([
                action,
                &self.source,
                "--target",
                target,
                "--name",
                &self.stem,
                "--json",
                "--root",
            ])
            .arg(&self.root)
            .args(["--provider", provider]);
        if equals {
            command.arg("--profile=bounded-generics-v1");
        } else {
            command.args(["--profile", "bounded-generics-v1"]);
        }
        if let Some(node) = node {
            command.arg("--node").arg(node);
        }
        command
    }
    pub fn bundle(&self, action: &str) -> PathBuf {
        self.root.join(".zryna/out").join(format!("{}.{action}", self.stem))
    }
    pub fn clear(&self, action: &str) {
        let path = self.bundle(action);
        if path.is_dir() {
            fs::remove_dir_all(path).expect("own bundle cleanup");
        } else {
            fs::remove_file(path).expect("own file collision cleanup");
        }
    }
    pub fn no_output(&self) {
        assert!(!self.bundle("build").exists());
        assert!(!self.bundle("run").exists());
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        for action in ["build", "run"] {
            if self.bundle(action).exists() {
                self.clear(action);
            }
        }
        fs::remove_dir_all(self.root.join(&self.directory)).expect("owned source fixture cleanup");
    }
}
pub fn output(command: &mut Command) -> Output {
    command.output().expect("actual CARGO_BIN_EXE_zryna starts")
}
pub fn inventory(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("owned published inventory") {
            let path = entry.expect("entry").path();
            assert!(!fs::symlink_metadata(&path).expect("entry metadata").file_type().is_symlink());
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).expect("own path").to_string_lossy().replace('\\', "/"),
                    fs::read(path).expect("artifact bytes"),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
pub fn manifest(case: &Case, action: &str) -> Value {
    let bytes = inventory(&case.bundle(action));
    let value: Value = serde_json::from_slice(&bytes["zryna-generic-manifest-v1.json"])
        .expect("plain strict metadata");
    assert_eq!(value["format"], "zryna.generic-bundle.v1");
    assert_eq!(value["profile"], "bounded-generics-v1");
    assert_eq!(value["syntaxProtocol"], 5);
    assert_eq!(value["command"], action);
    assert_eq!(value["artifactStem"], case.stem);
    let mut keys =
        value.as_object().expect("manifest object").keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "artifactStem",
            "artifacts",
            "command",
            "exports",
            "format",
            "invocation",
            "native",
            "profile",
            "source",
            "syntaxProtocol"
        ]
    );
    let artifacts = value["artifacts"].as_array().expect("artifact records");
    assert_eq!(bytes.len(), artifacts.len() + 1);
    for record in artifacts {
        let actual = &bytes[record["path"].as_str().expect("artifact path")];
        assert_eq!(u64::try_from(actual.len()).expect("bounded artifact bytes"), record["bytes"]);
        assert_eq!(format!("{:x}", Sha256::digest(actual)), record["sha256"]);
    }
    value
}
pub fn node() -> PathBuf {
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
        .expect("Node22.22.1 required")
        .canonicalize()
        .expect("absolute direct Node");
    let version = Command::new(&node).arg("--version").output().expect("actual Node version probe");
    assert!(version.status.success() && version.stderr.is_empty());
    assert!(matches!(version.stdout.as_slice(), b"v22.22.1\n" | b"v22.22.1\r\n"));
    node
}
