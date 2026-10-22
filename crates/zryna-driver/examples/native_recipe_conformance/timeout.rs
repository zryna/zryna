//! Separately approved trusted fault compiler; never a timeout flag or package-native tool grant.
use super::{configuration, fixture, hex, operator, prepare};
use std::time::{Duration, Instant};
use zryna_driver::{
    NativeRecipeError, NativeToolRootApproval, NativeToolRootKind, approve_native_recipe,
    execute_native_recipe, native_tool_root_identity,
};
use zryna_package::SourceTrustMode;

fn validate(
    normal: &NativeToolRootApproval,
    fault: &NativeToolRootApproval,
) -> Result<(), Box<dyn std::error::Error>> {
    const GCC: &str = "/tool/bin/gcc";
    native_tool_root_identity(normal)?;
    native_tool_root_identity(fault)?;
    assert_eq!(normal.engine_uid, fault.engine_uid);
    assert_ne!(normal.image_sha256, fault.image_sha256);
    let mut normal_tools = normal.tools.clone();
    let mut fault_tools = fault.tools.clone();
    let normal_gcc = normal_tools.remove(GCC).expect("approved ordinary compiler");
    let fault_gcc = fault_tools.remove(GCC).expect("separately approved trusted fault compiler");
    assert_ne!(normal_gcc, fault_gcc);
    assert_eq!(normal_tools, fault_tools, "no other executable authority may change");
    let original = normal.entries.get(GCC).expect("ordinary compiler material");
    let replacement = fault.entries.get(GCC).expect("trusted fault compiler material");
    assert_eq!(
        serde_json::to_value(original.metadata)?,
        serde_json::to_value(replacement.metadata)?,
        "compiler metadata authority must remain identical"
    );
    match (&original.kind, &replacement.kind) {
        (
            NativeToolRootKind::File { sha256: original, .. },
            NativeToolRootKind::File { sha256: replacement, .. },
        ) => {
            assert_eq!(hex(original), normal_gcc);
            assert_eq!(hex(replacement), fault_gcc);
        }
        _ => panic!("only the independently approved regular compiler body may change"),
    }
    let mut original = serde_json::to_value(&normal.entries)?;
    let mut replacement = serde_json::to_value(&fault.entries)?;
    original.as_object_mut().expect("closed material map").remove(GCC);
    replacement.as_object_mut().expect("closed material map").remove(GCC);
    assert_eq!(original, replacement, "all other files, directories and aliases must match");
    Ok(())
}

pub(super) fn run(
    normal: &NativeToolRootApproval,
    fault: &NativeToolRootApproval,
    sqlite: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    validate(normal, fault)?;
    let f = fixture::Fixture::new(|requirements| {
        configuration::build(fault, "timeout-control", requirements)
    });
    let before = f.snapshot();
    let plan = prepare(&f, sqlite);
    assert_eq!(plan.source_bytes(), fixture::C_SOURCE);
    let root_sha = native_tool_root_identity(fault)?;
    let operator = operator(&plan, fault, &root_sha, 1232);
    let approved = approve_native_recipe(plan, SourceTrustMode::TrustedNative, operator)?;
    let started = Instant::now();
    let error = execute_native_recipe(approved, fault.clone())
        .expect_err("trusted nonterminating compiler cannot produce a native result");
    let elapsed = started.elapsed();
    assert_eq!(error.reason(), "TRUST-TIMEOUT");
    assert_eq!(error.detail(), "bounded local Engine deadline exceeded");
    assert!(matches!(&error, NativeRecipeError::Rejected { .. }));
    assert!(elapsed >= Duration::from_secs(29) && elapsed < Duration::from_secs(36));
    assert_eq!(f.snapshot(), before, "deadline refusal cannot mutate source, lock or cache");
    Ok(())
}
