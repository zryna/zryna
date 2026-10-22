use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, io::Read as _, path::Path};
use zryna_driver::{
    NativeCompilePlan, NativeRecipeOperatorApproval, NativeToolRootApproval, approve_native_recipe,
    execute_native_recipe, native_tool_root_identity,
};
use zryna_package::SourceTrustMode;
#[path = "configuration.rs"]
mod configuration;
#[path = "fixture.rs"]
mod fixture;
#[path = "negative.rs"]
mod negative;
#[path = "timeout.rs"]
mod timeout;
const HEADER_SHA: &str = "cd20a2877bc82b94e36c1093b727dd87cfa3c1d581aac6e240925535cbd90166";
const OUTPUT_SHA: &str = "b0c13c1bbbfc07653dab908fd6f781a1fa051b2f9fae9d933d17b1ff7c1d1681";
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(DIGITS[usize::from(*byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(*byte & 15)]));
    }
    result
}
fn digest(value: &str) -> [u8; 32] {
    let mut result = [0; 32];
    assert_eq!(value.len(), 64);
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        result[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII"), 16).expect("hex");
    }
    result
}
fn read(path: &Path, limit: usize) -> Vec<u8> {
    let file = std::fs::File::open(path).expect("explicit operator input");
    let metadata = file.metadata().expect("operator input metadata");
    assert!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= limit as u64);
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).expect("bounded input");
    assert!(bytes.len() <= limit);
    bytes
}
fn prepare(f: &fixture::Fixture, sqlite: &[u8]) -> NativeCompilePlan {
    f.prepare(BTreeMap::from([
        ("library.h".into(), fixture::HEADER.to_vec()),
        ("sqlite3.h".into(), sqlite.to_vec()),
    ]))
}
fn operator(
    plan: &NativeCompilePlan,
    root: &NativeToolRootApproval,
    root_sha: &str,
    output_bytes: usize,
) -> NativeRecipeOperatorApproval {
    NativeRecipeOperatorApproval {
        source_plan_key: plan.source_plan().cache_key().into(),
        recipe_sha256: plan.recipe_sha256().into(),
        tool_root_sha256: root_sha.into(),
        image_sha256: root.image_sha256.clone(),
        policy: plan.configuration().execution_policy.clone(),
        approved_tools: plan.configuration().host_tools.clone(),
        output_path: plan.output_path().into(),
        output_bytes,
        output_sha256: digest(OUTPUT_SHA),
        dependency_symbols: vec!["sqlite3_free".into(), "sqlite3_malloc".into()],
    }
}
pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert!(
        matches!(args.len(), 3 | 4),
        "usage: native_recipe_conformance OPERATOR_ROOT_JSON SQLITE3_H GCC_VERSION [TIMEOUT_OPERATOR_ROOT_JSON]"
    );
    let root: NativeToolRootApproval =
        serde_json::from_slice(&read(Path::new(&args[0]), 4 * 1024 * 1024))?;
    let root_sha = native_tool_root_identity(&root)?;
    let sqlite = read(Path::new(&args[1]), 2 * 1024 * 1024);
    assert_eq!(sqlite.len(), 695_737);
    assert_eq!(hex(&Sha256::digest(&sqlite)), HEADER_SHA);
    let gcc_version = args[2].to_str().expect("ASCII GCC version");
    let mut observed = Vec::new();
    let mut owners = Vec::new();
    let mut source_identities = Vec::new();
    let mut plan_keys = Vec::new();
    for _ in 0..2 {
        let f = fixture::Fixture::new(|requirements| {
            configuration::build(&root, gcc_version, requirements)
        });
        let before = f.snapshot();
        let plan = prepare(&f, &sqlite);
        let source_identity = plan.sources().identity();
        source_identities.push(source_identity);
        plan_keys.push(plan.source_plan().cache_key().to_owned());
        let object_identity = *plan.requirements().object_sha256();
        let operator = operator(&plan, &root, &root_sha, 1232);
        let approved = approve_native_recipe(plan, SourceTrustMode::TrustedNative, operator)?;
        let success = execute_native_recipe(approved, root.clone())?;
        assert_eq!(success.object().bytes().len(), 1232);
        assert_eq!(hex(success.object().object_sha256()), OUTPUT_SHA);
        assert_eq!(success.object().library_id(), "fixture-sqlite-c-v0@0");
        assert_eq!(
            success.object().definitions().collect::<Vec<_>>(),
            ["sqlite_fixture_copy_bytes", "sqlite_fixture_release_bytes"]
        );
        assert_eq!(
            success.object().dependencies().collect::<Vec<_>>(),
            ["sqlite3_free", "sqlite3_malloc"]
        );
        success
            .object()
            .check_binding(success.approval().plan().requirements())
            .expect("actual foreign object retains its original source/machine issuer");
        assert_eq!(success.approval().plan().sources().identity(), source_identity);
        assert_eq!(*success.object().requirements().object_sha256(), object_identity);
        assert_eq!(native_tool_root_identity(success.tool_root())?, root_sha);
        let (recipe, holder) = success.container_ids();
        assert_eq!(recipe.len(), 64);
        assert_eq!(holder.len(), 64);
        assert_ne!(recipe, holder);
        assert!(!success.owner().is_empty());
        assert!(success.elapsed().as_secs() < 36);
        owners.push(success.owner().to_owned());
        observed.push(success.object().bytes().to_vec());
        assert_eq!(f.snapshot(), before, "no cache, output, lock or source change");
    }
    assert_ne!(owners[0], owners[1]);
    assert_ne!(source_identities[0], source_identities[1]);
    assert_eq!(plan_keys[0], plan_keys[1]);
    assert_eq!(observed[0], observed[1]);
    negative::run(&root, &root_sha, gcc_version, &sqlite)?;
    let timeout_checked = if let Some(path) = args.get(3) {
        let timeout_root: NativeToolRootApproval =
            serde_json::from_slice(&read(Path::new(path), 4 * 1024 * 1024))?;
        timeout::run(&root, &timeout_root, &sqlite)?;
        true
    } else {
        false
    };
    println!(
        "nativeRecipeConformance={}",
        serde_json::json!({"format":"zryna.native-recipe-conformance.v0",
        "freshSuccessfulSessions":2,"negativeSessions":2+usize::from(timeout_checked),
        "negativeOutputAndCompiler":true,"timeoutChecked":timeout_checked,
        "objectBytes":1232,"objectSha256":OUTPUT_SHA,"toolRootSha256":root_sha,
        "imageSha256":root.image_sha256,"owners":owners,"sourceBound":true,"projectUnchanged":true,
        "scope":"compile-only; cache/release/link/load denied"})
    );
    Ok(())
}
