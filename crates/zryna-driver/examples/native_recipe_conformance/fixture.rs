//! Actual resolver/ABI fixture for an explicitly invoked public-API conformance example.
//! No fake provider, output object or host-policy authority is constructed here.
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use zryna_driver::{
    BuildPlanConfiguration, NativeCompilePlan, NativeCompileRecipeRequest, NativeCompileRecipeStep,
    NativeCompileSourceInput, NativeRecipeReadOnlyMaterial, NativeRecipeRequirements,
    PackageBuildMode, PackageLockMode, PackageResolutionRequest, PackageResolutionSuccess,
    prepare_native_recipe, resolve_package,
};
use zryna_source::SourceMap;

#[path = "fixture/ffi.rs"]
mod ffi;
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
fn sha(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}
fn canonical(mut value: serde_json::Value) -> Vec<u8> {
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value).expect("fixture canonical JSON");
    bytes.push(b'\n');
    bytes
}
pub(super) struct Fixture {
    root: PathBuf,
    resolution: PackageResolutionSuccess,
    configuration: BuildPlanConfiguration,
    sources: SourceMap,
    requirements: NativeRecipeRequirements,
}
impl Fixture {
    pub(super) fn new(
        build_configuration: impl FnOnce(&NativeRecipeRequirements) -> BuildPlanConfiguration,
    ) -> Self {
        Self::with_c_source(C_SOURCE, build_configuration)
    }
    pub(super) fn with_c_source(
        c_source: &[u8],
        build_configuration: impl FnOnce(&NativeRecipeRequirements) -> BuildPlanConfiguration,
    ) -> Self {
        use std::os::unix::fs::DirBuilderExt as _;
        assert!(!c_source.is_empty() && c_source.len() <= 1024);
        let (sources, requirements) = ffi::capture();
        let configuration = build_configuration(&requirements);
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root =
            std::env::temp_dir().join(format!("zryna-native-live-{}-{suffix}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&root).expect("new private fixture root");
        let package = root.join("pkg");
        let files = [("src/library.c", c_source), (ZRYNA_PATH, SOURCE.as_bytes())];
        for (path, body) in files {
            let path = package.join(path);
            fs::create_dir_all(path.parent().expect("source parent")).expect("source directory");
            fs::write(path, body).expect("fixed source");
        }
        let inventory = files.map(
            |(path, body)| serde_json::json!({"path":path,"size":body.len(),"sha256":sha(body)}),
        );
        let manifest = serde_json::json!({"format":"zryna.package.v1","name":"native-fixture","version":"1.0.0",
            "source":{"kind":"local","locator":"pkg","revision":""},
            "compatibility":{"compiler":configuration.compiler.version,"profile":configuration.profile.id,"targets":["native-linux-x86_64"]},
            "files":inventory,"dependencies":[]});
        fs::write(package.join("zryna.package.json"), canonical(manifest)).expect("fixed manifest");
        let request = PackageResolutionRequest {
            source_root: root.clone(),
            package: "pkg".into(),
            git_cache: None,
            mode: PackageLockMode::Update,
        };
        resolve_package(&request).expect("separate explicit lock update");
        let resolution =
            resolve_package(&PackageResolutionRequest { mode: PackageLockMode::Frozen, ..request })
                .expect("original frozen resolver");
        fs::create_dir_all(root.join(".zryna/cache")).expect("cache sentinel directory");
        fs::create_dir_all(root.join(".zryna/out")).expect("output sentinel directory");
        fs::write(root.join(".zryna/cache/retain"), b"retain unrelated cache")
            .expect("cache sentinel");
        fs::write(root.join(".zryna/out/retain"), b"retain unrelated output")
            .expect("output sentinel");
        Self { root, resolution, configuration, sources, requirements }
    }
    pub(super) fn prepare(&self, headers: BTreeMap<String, Vec<u8>>) -> NativeCompilePlan {
        let package = &self.resolution.sources()[0];
        prepare_native_recipe(NativeCompileRecipeRequest {
            resolution: &self.resolution,
            configuration: &self.configuration,
            mode: PackageBuildMode::Frozen,
            step: NativeCompileRecipeStep {
                id: "compile-adapter".into(),
                tool: "gcc".into(),
                source: NativeCompileSourceInput {
                    package_id: package.package_id().into(),
                    source_sha256: package.source_sha256().into(),
                    path: "src/library.c".into(),
                },
                sysroots: vec![NativeRecipeReadOnlyMaterial {
                    id: "headers".into(),
                    files: headers,
                }],
                output: "adapter".into(),
            },
            ffi_package_id: package.package_id(),
            sources: &self.sources,
            requirements: &self.requirements,
        })
        .expect("public genuine retained plan")
    }
    pub(super) fn snapshot(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(
            root: &std::path::Path,
            path: &std::path::Path,
            output: &mut BTreeMap<PathBuf, Option<Vec<u8>>>,
        ) {
            for entry in fs::read_dir(path).expect("fixture directory") {
                let entry = entry.expect("entry");
                let path = entry.path();
                let kind = entry.file_type().expect("type");
                assert!(kind.is_file() || kind.is_dir());
                output.insert(
                    path.strip_prefix(root).expect("relative").into(),
                    kind.is_file().then(|| fs::read(&path).expect("bytes")),
                );
                if kind.is_dir() {
                    walk(root, &path, output);
                }
            }
        }
        let mut output = BTreeMap::new();
        walk(&self.root, &self.root, &mut output);
        output
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("owned fixture cleanup");
    }
}
