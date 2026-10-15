//! A native-looking success record is not pure compiler cache authority.
use super::{approval, support};
use crate::package_build::{
    self, BuildTargetId, CachedTargetAuthentication, PackageBuildError, PackageBuildMode,
    PackageBuildRequest, PackageCompilationResult, PackageCompilationUnit, PureSourceCompiler,
};
use crate::{ArtifactCacheRoot, ArtifactOutputRoot};
use serde::Serialize;
use std::fs;
use zryna_package::SourceTrustMode;

const FORGED: &[u8] =
    b"{\"kind\":\"NativeRecipeSuccess\",\"trustedNative\":true,\"isolated\":true}\n";
struct RejectForeignCache {
    authentications: usize,
}
impl PureSourceCompiler for RejectForeignCache {
    fn compile(
        &mut self,
        _unit: PackageCompilationUnit<'_>,
    ) -> Result<PackageCompilationResult, PackageBuildError> {
        panic!("found native-looking cache cannot fall through to compilation");
    }
    fn authenticate_cached_outputs(
        &mut self,
        record: CachedTargetAuthentication<'_>,
    ) -> Result<(), PackageBuildError> {
        self.authentications += 1;
        assert_eq!(record.outputs.len(), 1);
        assert_eq!(record.outputs[0].bytes, FORGED);
        Err(PackageBuildError::compiler("native JSON success is not pure compiler authority"))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OfferedEntry<'a> {
    format: &'static str,
    plan_cache_key: &'a str,
    target: &'static str,
    target_cache_key: &'a str,
    version: u8,
    outputs: Vec<OfferedOutput>,
}
#[derive(Serialize)]
struct OfferedOutput {
    bytes: u64,
    path: &'static str,
    sha256: String,
}
fn metadata(plan_key: &str, key: &str) -> Vec<u8> {
    let entry = OfferedEntry {
        format: "zryna.build-cache-entry.v0",
        plan_cache_key: plan_key,
        target: "native-linux-x86_64",
        target_cache_key: key,
        version: 0,
        outputs: vec![OfferedOutput {
            bytes: u64::try_from(FORGED.len()).expect("bounded fixture"),
            path: "native/adapter.o",
            sha256: support::sha(FORGED),
        }],
    };
    let mut bytes =
        serde_json::to_vec(&entry).expect("independently authored canonical cache fixture");
    bytes.push(b'\n');
    bytes
}

#[test]
fn forged_native_success_cache_cannot_authenticate_or_repair_a_pure_source_result() {
    let f = support::Fixture::new();
    let identity = f.plan();
    let cache = ArtifactCacheRoot::prepare_for_project(&f.root).expect("retained cache root");
    let output = ArtifactOutputRoot::prepare_for_workspace(&f.root).expect("retained output root");
    let key = identity
        .source_plan()
        .target_cache_key(&BuildTargetId::NativeLinuxX86_64)
        .expect("target key");
    let entry = cache.path().join("build-plan-v0").join(key);
    fs::create_dir_all(entry.join("native")).expect("complete forged cache");
    fs::write(entry.join("native/adapter.o"), FORGED).expect("offered bytes");
    fs::write(entry.join("entry.json"), metadata(identity.source_plan().cache_key(), key))
        .expect("offered metadata");
    let before = f.snapshot();
    let request = PackageBuildRequest {
        resolution: &f.resolution,
        configuration: f.configuration.clone(),
        mode: PackageBuildMode::Frozen,
        cache_root: &cache,
        output_root: &output,
    };
    let mut compiler = RejectForeignCache { authentications: 0 };
    assert_eq!(
        package_build::execute_package_build(&request, &mut compiler)
            .expect_err("self-consistent bytes are not compiler authority")
            .code(),
        "ZRYNA-B4102"
    );
    assert_eq!(compiler.authentications, 1);
    assert_eq!(f.snapshot(), before);
    fs::write(entry.join("entry.json"), metadata(&"0".repeat(64), key))
        .expect("incompatible metadata");
    let before = f.snapshot();
    let mut compiler = RejectForeignCache { authentications: 0 };
    assert_eq!(
        package_build::execute_package_build(&request, &mut compiler)
            .expect_err("corrupt found entry is not a miss")
            .code(),
        "ZRYNA-B4102"
    );
    assert_eq!(compiler.authentications, 0);
    assert_eq!(f.snapshot(), before);
    let approved =
        approval::prepare_and_admit(f.request(), SourceTrustMode::TrustedNative, f.approval())
            .expect("fresh-only input admission does not consume a cache");
    assert_eq!(approved.plan().source_bytes(), support::C_SOURCE);
    assert_eq!(f.snapshot(), before);
}
