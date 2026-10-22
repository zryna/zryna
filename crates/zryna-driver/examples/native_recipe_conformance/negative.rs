//! Real compiler/output failure controls; ordinary cleanup errors cannot satisfy either case.
use super::{configuration, fixture, operator, prepare};
use zryna_driver::{
    NativeRecipeError, NativeToolRootApproval, approve_native_recipe, execute_native_recipe,
};
use zryna_package::SourceTrustMode;

pub(super) fn run(
    root: &NativeToolRootApproval,
    root_sha: &str,
    gcc_version: &str,
    sqlite: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let compiler_error =
        [b"#error native recipe refusal control\n".as_slice(), fixture::C_SOURCE].concat();
    assert!(compiler_error.len() < 1024);
    for (source, output_bytes, reason, detail) in [
        (
            fixture::C_SOURCE.to_vec(),
            1233,
            "TRUST-OUTPUT",
            "complete output archive differs from independent inventory",
        ),
        (compiler_error, 1232, "TRUST-ENFORCEMENT", "actual native compiler exited unsuccessfully"),
    ] {
        let f = fixture::Fixture::with_c_source(&source, |requirements| {
            configuration::build(root, gcc_version, requirements)
        });
        let before = f.snapshot();
        let plan = prepare(&f, sqlite);
        assert_eq!(plan.source_bytes(), source);
        let operator = operator(&plan, root, root_sha, output_bytes);
        let approved = approve_native_recipe(plan, SourceTrustMode::TrustedNative, operator)?;
        let error = execute_native_recipe(approved, root.clone())
            .expect_err("real wrong-output or compiler failure cannot produce native success");
        assert_eq!(error.reason(), reason);
        assert_eq!(error.detail(), detail, "failure must reach the intended owning phase");
        assert!(matches!(&error, NativeRecipeError::Rejected { .. }));
        assert_eq!(f.snapshot(), before, "failed session cannot mutate source, lock or cache");
    }
    Ok(())
}
