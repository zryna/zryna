use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use zryna_package::{PackageSource, PackageSourceKind, PackageSourceProvider as _};

use super::{
    CapturedRoot, FilesystemProvider, MAX_SOURCE_ENTRIES, PackageLockMode,
    PackageResolutionRequest, git_cache_key, resolve_package, resolve_project_package,
};

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
    write_package_file(directory, name, source, "src/main.zry", body, dependencies);
}

fn write_package_file(
    directory: &Path,
    name: &str,
    source: Value,
    file_path: &str,
    body: &[u8],
    dependencies: Vec<Value>,
) {
    let source_path = directory.join(file_path);
    fs::create_dir_all(source_path.parent().expect("source parent")).expect("source directory");
    fs::write(source_path, body).expect("source file");
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
            "path": file_path,
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

fn local_source() -> PackageSource {
    PackageSource {
        kind: PackageSourceKind::Local,
        locator: "packages/app".to_owned(),
        revision: String::new(),
    }
}

fn provider(root: &TemporaryRoot) -> FilesystemProvider {
    FilesystemProvider {
        source_root: CapturedRoot::capture(root.path()).expect("captured source root"),
        git_cache: None,
        root_source: local_source(),
        local_scope: None,
        loaded_files: BTreeMap::new(),
        retained_files: Vec::new(),
        retained_packages: Vec::new(),
    }
}

fn add_empty_directories(package: &Path, count: usize) {
    for index in 0..count {
        fs::create_dir(package.join(format!("empty-{index:03}"))).expect("empty directory");
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
    assert_eq!(update.sources().len(), 2);
    assert!(update.sources().iter().all(|package| !package.files().is_empty()));
    assert!(
        update
            .sources()
            .iter()
            .flat_map(super::AuthenticatedPackageSources::files)
            .all(|file| { file.sha256() == format!("{:x}", Sha256::digest(file.bytes())) })
    );
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

#[test]
fn root_project_state_is_reserved_but_other_extra_files_still_reject() {
    let root = TemporaryRoot::new("project-state");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    fs::create_dir_all(package.join(".zryna/out/example.build")).expect("project state directory");
    fs::write(package.join(".zryna/out/example.build/artifact"), b"generated")
        .expect("project state artifact");
    resolve_package(&request(&root, PackageLockMode::Update)).expect("reserved project state");
    resolve_package(&request(&root, PackageLockMode::Frozen)).expect("frozen project replay");

    fs::write(package.join("undeclared.txt"), b"undeclared").expect("undeclared source");
    let error = resolve_package(&request(&root, PackageLockMode::Frozen))
        .expect_err("extra file rejection");
    assert_eq!(error.code(), "ZRYNA-P4004");
}

#[test]
fn root_project_state_name_must_be_a_real_directory() {
    let root = TemporaryRoot::new("project-state-file");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    fs::write(package.join(".zryna"), b"not a directory").expect("reserved state file");
    let error = resolve_package(&request(&root, PackageLockMode::Update))
        .expect_err("state file rejection");
    assert_eq!(error.code(), "ZRYNA-P4004");
    assert!(!package.join("zryna.lock.json").exists());
}

#[test]
fn project_scope_rejects_a_sibling_dependency_before_filesystem_acquisition() {
    let root = TemporaryRoot::new("project-sibling");
    let package = root.path().join("packages/app");
    write_package(
        &package,
        "app",
        source("local", "packages/app", ""),
        b"app\n",
        vec![json!({
            "alias": "sibling",
            "name": "sibling",
            "source": source("local", "packages/sibling", ""),
            "version": "1.0.0"
        })],
    );
    let error = resolve_project_package(&request(&root, PackageLockMode::Update))
        .expect_err("out-of-tree dependency must reject before acquisition");
    assert_eq!(error.code(), "ZRYNA-P4004");
    assert_eq!(error.detail(), "local package dependency escapes the explicit project tree");
    assert!(!root.path().join("packages/sibling").exists());
}

#[test]
fn exact_directory_entry_budget_is_accepted() {
    let root = TemporaryRoot::new("entries-exact");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    // The manifest, `src`, and `src/main.zry` consume three entries.
    add_empty_directories(&package, MAX_SOURCE_ENTRIES - 3);
    resolve_package(&request(&root, PackageLockMode::Update)).expect("exact entry budget");
}

#[test]
fn first_extra_directory_entry_is_rejected_without_truncation() {
    let root = TemporaryRoot::new("entries-extra");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    add_empty_directories(&package, MAX_SOURCE_ENTRIES - 2);
    let error =
        resolve_package(&request(&root, PackageLockMode::Update)).expect_err("first extra entry");
    assert_eq!(error.code(), "ZRYNA-P4004");
    assert!(!package.join("zryna.lock.json").exists());
}

#[test]
fn source_path_deeper_than_thirty_two_directories_is_accepted() {
    let root = TemporaryRoot::new("deep-source");
    let package = root.path().join("packages/app");
    let file_path = format!("{}x.zry", "a/".repeat(33));
    assert_eq!(file_path.matches('/').count(), 33);
    assert!(file_path.len() < 96);
    write_package_file(
        &package,
        "app",
        source("local", "packages/app", ""),
        &file_path,
        b"app\n",
        vec![],
    );
    resolve_package(&request(&root, PackageLockMode::Update)).expect("deep valid source path");
}

#[test]
fn exact_source_path_byte_budget_is_accepted() {
    let root = TemporaryRoot::new("path-exact");
    let package = root.path().join("packages/app");
    let file_path = format!("{}xx.zry", "a/".repeat(45));
    assert_eq!(file_path.len(), 96);
    write_package_file(
        &package,
        "app",
        source("local", "packages/app", ""),
        &file_path,
        b"app\n",
        vec![],
    );
    resolve_package(&request(&root, PackageLockMode::Update)).expect("exact path budget");
    resolve_package(&request(&root, PackageLockMode::Frozen)).expect("exact frozen replay");
}

#[test]
fn first_extra_source_path_byte_is_rejected_without_publication() {
    let root = TemporaryRoot::new("path-extra");
    let package = root.path().join("packages/app");
    let file_path = format!("{}xxx.zry", "a/".repeat(45));
    assert_eq!(file_path.len(), 97);
    write_package_file(
        &package,
        "app",
        source("local", "packages/app", ""),
        &file_path,
        b"app\n",
        vec![],
    );
    let error = resolve_package(&request(&root, PackageLockMode::Update))
        .expect_err("first extra path byte");
    assert_eq!(error.code(), "ZRYNA-P4005");
    assert!(!package.join("zryna.lock.json").exists());
}

#[cfg(unix)]
#[test]
fn interleaved_ascii_case_collision_is_rejected() {
    let root = TemporaryRoot::new("case-collision");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    for name in ["A", "B", "a"] {
        fs::create_dir(package.join(name)).expect("collision fixture directory");
    }
    let error = resolve_package(&request(&root, PackageLockMode::Update))
        .expect_err("interleaved case collision");
    assert_eq!(error.code(), "ZRYNA-P4004");
    assert!(!package.join("zryna.lock.json").exists());
}

#[test]
fn new_nested_entry_after_load_is_rejected() {
    let root = TemporaryRoot::new("entry-mutation");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    let mut provider = provider(&root);
    provider.load(&local_source()).expect("initial package load");
    fs::create_dir(package.join("src/added")).expect("new nested entry");
    let error = provider.revalidate().expect_err("entry-set mutation");
    assert_eq!(error.code(), "ZRYNA-P4004");
}

#[cfg(unix)]
#[test]
fn removed_empty_directory_after_load_is_rejected() {
    let root = TemporaryRoot::new("entry-removal");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    fs::create_dir(package.join("empty")).expect("empty directory");
    let mut provider = provider(&root);
    provider.load(&local_source()).expect("initial package load");
    fs::remove_dir(package.join("empty")).expect("remove empty directory");
    let error = provider.revalidate().expect_err("removed entry");
    assert_eq!(error.code(), "ZRYNA-P4004");
}

#[cfg(unix)]
#[test]
fn replaced_nested_directory_after_load_is_rejected() {
    let root = TemporaryRoot::new("directory-replacement");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    let mut provider = provider(&root);
    provider.load(&local_source()).expect("initial package load");
    fs::rename(package.join("src"), package.join("old-src")).expect("move retained directory");
    fs::create_dir(package.join("src")).expect("replacement directory");
    fs::write(package.join("src/main.zry"), b"app\n").expect("replacement source");
    let error = provider.revalidate().expect_err("nested directory replacement");
    assert_eq!(error.code(), "ZRYNA-P4004");
}

#[cfg(unix)]
#[test]
fn directory_to_file_kind_change_after_load_is_rejected() {
    let root = TemporaryRoot::new("kind-mutation");
    let package = root.path().join("packages/app");
    write_package(&package, "app", source("local", "packages/app", ""), b"app\n", vec![]);
    fs::create_dir(package.join("empty")).expect("empty directory");
    let mut provider = provider(&root);
    provider.load(&local_source()).expect("initial package load");
    fs::remove_dir(package.join("empty")).expect("remove empty directory");
    fs::write(package.join("empty"), b"replacement\n").expect("replacement file");
    let error = provider.revalidate().expect_err("entry kind mutation");
    assert_eq!(error.code(), "ZRYNA-P4004");
}
