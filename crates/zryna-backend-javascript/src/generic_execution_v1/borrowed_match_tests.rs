//! Genuine both-provider borrowed matches reach JS independently of other unfinished targets.
use std::process::Command;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
#[path = "../../../zryna-driver/tests/generic_execution_v1_borrowed_match_source_seal/negative.rs"]
mod negative;
#[path = "../../../zryna-driver/tests/generic_execution_v1_borrowed_match_source_seal/source.rs"]
mod source;
pub(in crate::generic_execution_v1) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&VerifiedExecution<'_>, &[u8]),
) {
    source::with_program(case, provider, check);
}
fn actual(case: &str, observed: bool) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let text =
                if observed {
                    let artifact = super::emit_observed(program)
                        .expect("genuine selected-arm observed JS owner");
                    assert_eq!(
                        artifact,
                        super::emit_observed(program).expect("deterministic selected-arm retry")
                    );
                    artifact.as_source().to_owned()
                } else {
                    let artifact =
                        super::emit(program).expect("genuine selected-arm ordinary JS owner");
                    assert_eq!(
                        artifact,
                        super::emit(program).expect("deterministic ordinary selected-arm retry")
                    );
                    assert!(artifact.source.contains(
                        "987fa645b5f81a9c6fd03f7edcbd23203e7cc61e8aa206b1b08a5ef61d3b1421"
                    ));
                    for forbidden in [
                        "globalThis",
                        "Symbol.for",
                        "import ",
                        "import(",
                        "require(",
                        "eval(",
                        "new Function",
                    ] {
                        assert!(!artifact.source.contains(forbidden));
                    }
                    artifact.source
                };
            launch(case, provider, &text, observed);
        });
    }
}
fn launch(case: &str, provider: &str, text: &str, observed: bool) {
    let mode = if observed { "observed" } else { "ordinary" };
    let file = std::env::temp_dir()
        .join(format!("m416-js-borrowed-{}-{case}-{provider}-{mode}.mjs", std::process::id()));
    std::fs::write(&file, text).expect("actual immutable new-seal emitted module");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../zryna-driver/tests/generic_execution_v1_borrowed_match_targets")
        .join(format!("javascript-{mode}.mjs"));
    let output = Command::new("node").arg(script).arg(&file).arg(case).arg("faults").output();
    std::fs::remove_file(file).expect("remove test-only target module");
    let output = output.expect("actual Node process");
    assert!(
        output.status.success(),
        "{case}/{provider}/{mode}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() <= 65_536);
    let receipt: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("actual runtime receipt");
    assert_eq!(receipt["case"], case);
    assert_eq!(receipt["pristineRetries"], true);
}
macro_rules! cases {($($ordinary:ident,$observed:ident,$case:literal;)*)=>{$(
    #[test] fn $ordinary(){actual($case,false);}
    #[test] fn $observed(){actual($case,true);}
)*};}
cases! {
    ordinary_nominal_shared_copy,observed_nominal_shared_copy,"nominal-shared-copy";
    ordinary_nominal_shared_clone,observed_nominal_shared_clone,"nominal-shared-clone";
    ordinary_nominal_exclusive_copy,observed_nominal_exclusive_copy,"nominal-exclusive-copy";
    ordinary_nominal_exclusive_replace,observed_nominal_exclusive_replace,"nominal-exclusive-replace";
    ordinary_option_shared_both,observed_option_shared_both,"option-shared-both";
    ordinary_result_shared_opaque,observed_result_shared_opaque,"result-shared-opaque-bound";
    ordinary_result_exclusive,observed_result_exclusive,"result-exclusive-both";
    ordinary_opaque_forward,observed_opaque_forward,"opaque-payload-forward";
    ordinary_nested_exclusive,observed_nested_exclusive,"nested-exclusive-result";
    ordinary_branch_cleanup,observed_branch_cleanup,"branch-arm-cleanup";
    ordinary_shared_reborrow,observed_shared_reborrow,"shared-reborrow-from-exclusive";
}
