use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use zryna_package::{PackageSource, PackageSourceKind};

use super::{PackageLockMode, PackageResolutionRequest, git_cache_key, resolve_package};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "zryna-package-{label}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temporary root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("temporary root cleanup");
    }
}

fn canonical(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("canonical JSON");
    bytes.push(b'\n');
    bytes
}

fn source(kind: &str, locator: &str, revision: &str) -> Value {
    json!({ "kind": kind, "locator": locator, "revision": revision })
}

fn write_package(
    directory: &Path,
    name: &str,
    source: Value,
    body: &[u8],
    dependencies: Vec<Value>,
) {
    fs::create_dir_all(directory.join("src")).expect("source directory");
    fs::write(directory.join("src/main.zry"), body).expect("source file");
    let source = serde_json::to_value(source).expect("source value");
    let dependencies = serde_json::to_value(dependencies).expect("dependency values");
    let manifest = json!({
        "compatibility": {
            "compiler": "0.1.0",
            "profile": "control-flow-v1",
            "targets": ["javascript", "webassembly"]
        },
        "dependencies": dependencies,
        "files": [{
            "path": "src/main.zry",
            "sha256": format!("{:x}", Sha256::digest(body)),
            "size": body.len()
        }],
        "format": "zryna.package.v1",
        "name": name,
        "source": source,
        "version": "1.0.0"
    });
    fs::write(directory.join("zryna.package.json"), canonical(&manifest)).expect("manifest");
}

fn request(root: &TemporaryRoot, mode: PackageLockMode) -> PackageResolutionRequest {
    PackageResolutionRequest {
        source_root: root.path().to_path_buf(),
        package: "packages/app".to_owned(),
        git_cache: None,
        mode,
    }
}

#[test]
fn local_update_is_atomic_and_frozen_replay_is_exact() {
    let root = TemporaryRoot::new("local");
    let app = root.path().join("packages/app");
    let library = root.path().join("packages/library");
    write_package(
        &app,
        "app",
        source("local", "packages/app", ""),
        b"export function main(): i32 { return 1; }\n",
        vec![json!({
            "alias": "math",
            "name": "library",
            "source": source("local", "packages/library", ""),
            "version": "1.0.0"
        })],
    );
    write_package(
        &library,
        "library",
        source("local", "packages/library", ""),
        b"export function value(): i32 { return 1; }\n",
        vec![],
    );
    let update = resolve_package(&request(&root, PackageLockMode::Update)).expect("update");
    assert!(update.published());
    let lock = fs::read(update.lock_path()).expect("published lock");
    assert_eq!(lock, update.graph().lock_bytes());
    let repeated =
        resolve_package(&request(&root, PackageLockMode::Update)).expect("atomic update");
    assert_eq!(fs::read(repeated.lock_path()).expect("replaced lock"), lock);
    let frozen = resolve_package(&request(&root, PackageLockMode::Frozen)).expect("frozen replay");
    assert!(!frozen.published());
    assert_eq!(frozen.graph(), update.graph());
    assert!(!app.join(".zryna.lock.pending").exists());
}

#[test]
fn prepopulated_exact_commit_cache_is_used_without_git_execution() {
    let root = TemporaryRoot::new("git");
    let cache = TemporaryRoot::new("cache");
    let revision = "0123456789abcdef0123456789abcdef01234567";
    let git = PackageSource {
        kind: PackageSourceKind::Git,
        locator: "https://example.com/zryna/math.git".to_owned(),
        revision: revision.to_owned(),
    };
    write_package(
        &root.path().join("packages/app"),
        "app",
        source("local", "packages/app", ""),
        b"app\n",
        vec![json!({
            "alias": "math",
            "name": "math",
            "source": source("git", &git.locator, revision),
            "version": "1.0.0"
        })],
    );
    write_package(
        &cache.path().join(git_cache_key(&git)),
        "math",
        source("git", &git.locator, revision),
        b"math\n",
        vec![],
    );
    let mut request = request(&root, PackageLockMode::Update);
    request.git_cache = Some(cache.path().to_path_buf());
    assert_eq!(resolve_package(&request).expect("Git-cache graph").graph().packages().len(), 2);
}

#[cfg(unix)]
#[test]
fn linked_source_is_rejected_without_publication() {
    use std::os::unix::fs::symlink;

    let root = TemporaryRoot::new("link");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    fs::remove_file(package.join("src/main.zry")).expect("remove source");
    fs::write(root.path().join("outside.zry"), b"app\n").expect("outside source");
    symlink(root.path().join("outside.zry"), package.join("src/main.zry")).expect("source link");
    let error =
        resolve_package(&request(&root, PackageLockMode::Update)).expect_err("link rejection");
    assert_eq!(error.code(), "ZRYNA-P4004");
    assert!(!package.join("zryna.lock.json").exists());
}
