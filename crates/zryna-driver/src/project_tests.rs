use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::json;
use sha2::{Digest as _, Sha256};
use zryna_package::{
    LockMode, PackageFile, PackageMaterial, PackageSource, PackageSourceKind,
    PackageSourceProvider, ResolveError,
};

use super::{ProjectAdmission, ProjectBuildRequest};
use crate::TargetSelection;

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Case {
    parent: PathBuf,
    project: PathBuf,
    source: Vec<u8>,
}

impl Case {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "zryna-project-admission-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        let project = parent.join("example");
        fs::create_dir_all(project.join("src")).expect("project source directory");
        let source = b"export function main(): i32 { return 42; }\n".to_vec();
        fs::write(project.join("src/main.zry"), &source).expect("project source");
        let package_source = PackageSource {
            kind: PackageSourceKind::Local,
            locator: "example".to_owned(),
            revision: String::new(),
        };
        let manifest = canonical(&json!({
            "format": "zryna.package.v1",
            "name": "example",
            "version": "0.1.0",
            "source": package_source,
            "compatibility": {
                "compiler": env!("CARGO_PKG_VERSION"),
                "profile": "i32-v1",
                "targets": ["javascript"],
            },
            "files": [{
                "path": "src/main.zry",
                "size": source.len(),
                "sha256": format!("{:x}", Sha256::digest(&source)),
            }],
            "dependencies": [],
        }));
        fs::write(project.join("zryna.package.json"), &manifest).expect("project manifest");
        let mut provider = OnePackage {
            source: package_source.clone(),
            material: Some(PackageMaterial {
                manifest,
                files: vec![PackageFile { path: "src/main.zry".to_owned(), bytes: source.clone() }],
            }),
        };
        let graph = zryna_package::resolve(&mut provider, package_source, LockMode::Update)
            .expect("project graph");
        fs::write(project.join("zryna.lock.json"), graph.lock_bytes()).expect("project lock");
        Self { parent, project, source }
    }

    fn request(&self) -> ProjectBuildRequest {
        ProjectBuildRequest {
            compiler_root: self.parent.clone(),
            project_root: self.project.clone(),
            entrypoint: "src/main.zry".to_owned(),
            artifact_stem: "example".to_owned(),
            targets: TargetSelection::JavaScript,
            node_runtime: absolute_dummy_node(),
        }
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.parent).expect("project fixture cleanup");
    }
}

struct OnePackage {
    source: PackageSource,
    material: Option<PackageMaterial>,
}

impl PackageSourceProvider for OnePackage {
    fn load(&mut self, source: &PackageSource) -> Result<PackageMaterial, ResolveError> {
        if source != &self.source {
            return Err(ResolveError::source("unexpected test package"));
        }
        self.material.take().ok_or_else(|| ResolveError::source("duplicate test package load"))
    }
}

fn canonical(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("canonical JSON");
    bytes.push(b'\n');
    bytes
}

fn absolute_dummy_node() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\zryna-test-node.exe")
    } else {
        PathBuf::from("/zryna-test-node")
    }
}

fn replace(path: &Path, bytes: &[u8]) {
    let replacement = path.with_extension("replacement");
    fs::write(&replacement, bytes).expect("replacement source");
    fs::remove_file(path).expect("remove prior source");
    fs::rename(replacement, path).expect("replace source");
}

#[test]
fn admitted_entrypoint_bytes_survive_changed_then_restored_host_paths() {
    let case = Case::new();
    let admission = ProjectAdmission::discover(&case.request()).expect("project admission");
    let source_path = case.project.join("src/main.zry");
    replace(&source_path, b"export function main(): i32 { return 7; }\n");
    replace(&source_path, &case.source);
    assert_eq!(admission.entrypoint_text().expect("retained source").as_bytes(), case.source);
    admission.revalidate().expect("exact restored bytes must replay the graph");
}

#[test]
fn current_entrypoint_mutation_fails_before_project_execution_can_continue() {
    let case = Case::new();
    let admission = ProjectAdmission::discover(&case.request()).expect("project admission");
    replace(&case.project.join("src/main.zry"), b"export function main(): i32 { return 7; }\n");
    let failure = admission.revalidate().expect_err("mutated project must reject");
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-P4004");
}
