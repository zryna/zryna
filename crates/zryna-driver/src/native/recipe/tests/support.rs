//! Real resolver/FFI issuers and explicit audit-only object data.
use crate::native::native_c_v0::resource_identity::HandleLinkRequirements;
use crate::native::recipe::{
    approval::OperatorApproval,
    plan::{
        self, CompilePlanRequest, NativeCompilePlan, NativeCompileStep, ReadOnlyMaterial,
        SourceInput,
    },
};
use crate::package_build::{BuildPlanConfiguration, PackageBuildMode};
use crate::{PackageLockMode, PackageResolutionRequest, PackageResolutionSuccess, resolve_package};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use zryna_source::SourceMap;

mod configuration;
mod ffi;
mod object;
pub(super) fn encoded() -> Vec<u8> {
    object::encoded_with_dependencies(&[])
}
pub(super) fn encoded_with_dependencies(dependencies: &[&str]) -> Vec<u8> {
    object::encoded_with_dependencies(dependencies)
}

pub(super) const ZRYNA_PATH: &str = "tests/native-c-sqlite-v0/source-buffer.zry";
pub(super) const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-sqlite-v0/source-buffer.zry"
));
pub(super) const C_SOURCE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-sqlite-v0/library.c"
));
pub(super) const HEADER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-sqlite-v0/library.h"
));
pub(super) const POLICY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-sqlite-v0/library-policy.json"
));
pub(super) const DECLARATIONS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-sqlite-v0/declarations.ffi.json"
));
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn canonical(mut value: Value) -> Vec<u8> {
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value).expect("canonical fixture");
    bytes.push(b'\n');
    bytes
}
fn write_package(root: &Path, c_source: &[u8], zryna: &[u8]) {
    let directory = root.join("pkg");
    let files = [("src/library.c", c_source), (ZRYNA_PATH, zryna)];
    for (path, body) in files {
        let path = directory.join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("fixture parent");
        fs::write(path, body).expect("fixture source");
    }
    let inventory =
        files.map(|(path, body)| json!({"path":path,"size":body.len(),"sha256":sha(body)}));
    let manifest = json!({"format":"zryna.package.v1","name":"native-fixture","version":"1.0.0",
        "source":{"kind":"local","locator":"pkg","revision":""},
        "compatibility":{"compiler":"0.1.0","profile":"data-ownership-v1","targets":["native-linux-x86_64"]},
        "files":inventory,"dependencies":[]});
    fs::write(directory.join("zryna.package.json"), canonical(manifest)).expect("fixture manifest");
}
pub(in crate::native::recipe) struct Fixture {
    pub root: PathBuf,
    pub resolution: PackageResolutionSuccess,
    pub configuration: BuildPlanConfiguration,
    pub sources: SourceMap,
    pub requirements: HandleLinkRequirements,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with_c_source(C_SOURCE)
    }
    pub fn with_c_source(c_source: &[u8]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "zryna-native-plan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("owned temporary fixture");
        write_package(&root, c_source, SOURCE.as_bytes());
        resolve_package(&PackageResolutionRequest {
            source_root: root.clone(),
            package: "pkg".into(),
            git_cache: None,
            mode: PackageLockMode::Update,
        })
        .expect("publish exact lock");
        let resolution = resolve_package(&PackageResolutionRequest {
            source_root: root.clone(),
            package: "pkg".into(),
            git_cache: None,
            mode: PackageLockMode::Frozen,
        })
        .expect("real frozen resolver");
        let (sources, requirements) = ffi::capture();
        fs::create_dir_all(root.join(".zryna/cache")).expect("existing cache");
        fs::create_dir_all(root.join(".zryna/out")).expect("existing output");
        fs::write(root.join(".zryna/cache/retain"), b"unrelated cache").expect("cache sentinel");
        fs::write(root.join(".zryna/out/retain"), b"unrelated output").expect("output sentinel");
        Self {
            root,
            resolution,
            configuration: configuration::configuration(),
            sources,
            requirements,
        }
    }
    pub fn request(&self) -> CompilePlanRequest<'_> {
        self.request_for(&self.resolution)
    }
    pub fn request_for<'a>(
        &'a self,
        resolution: &'a PackageResolutionSuccess,
    ) -> CompilePlanRequest<'a> {
        let package = &resolution.sources()[0];
        CompilePlanRequest {
            resolution,
            configuration: &self.configuration,
            mode: PackageBuildMode::Frozen,
            step: NativeCompileStep {
                id: "compile-adapter".into(),
                tool: "gcc".into(),
                source: SourceInput {
                    package_id: package.package_id().into(),
                    source_sha256: package.source_sha256().into(),
                    path: "src/library.c".into(),
                },
                sysroots: vec![ReadOnlyMaterial {
                    id: "headers".into(),
                    files: BTreeMap::from([("library.h".into(), HEADER.to_vec())]),
                }],
                output: "adapter".into(),
            },
            ffi_package_id: package.package_id(),
            sources: &self.sources,
            requirements: &self.requirements,
        }
    }
    pub fn plan(&self) -> NativeCompilePlan {
        plan::prepare(self.request()).expect("source-authenticated input plan")
    }
    pub fn approval(&self) -> OperatorApproval {
        let plan = self.plan();
        let object = encoded();
        OperatorApproval {
            source_plan_key: plan.source_plan().cache_key().into(),
            recipe_sha256: plan.recipe_sha256().into(),
            tool_root_sha256: "a".repeat(64),
            image_sha256: "b".repeat(64),
            policy: self.configuration.execution_policy.clone(),
            approved_tools: self.configuration.host_tools.clone(),
            output_path: plan.output_path().into(),
            output_bytes: object.len(),
            output_sha256: Sha256::digest(&object).into(),
            dependency_symbols: vec![],
        }
    }
    pub fn rewrite_source(&self, c_source: &[u8], zryna: &[u8]) {
        write_package(&self.root, c_source, zryna);
    }
    pub fn frozen(&self) -> Result<PackageResolutionSuccess, zryna_package::ResolveError> {
        resolve_package(&PackageResolutionRequest {
            source_root: self.root.clone(),
            package: "pkg".into(),
            git_cache: None,
            mode: PackageLockMode::Frozen,
        })
    }
    pub fn snapshot(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        snapshot(&self.root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("owned fixture cleanup");
    }
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(path).expect("fixture directory") {
            let entry = entry.expect("fixture entry");
            let path = entry.path();
            let kind = entry.file_type().expect("fixture type");
            assert!(kind.is_file() || kind.is_dir());
            out.insert(
                path.strip_prefix(root).expect("relative fixture").to_path_buf(),
                kind.is_file().then(|| fs::read(&path).expect("fixture body")),
            );
            if kind.is_dir() {
                walk(root, &path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
