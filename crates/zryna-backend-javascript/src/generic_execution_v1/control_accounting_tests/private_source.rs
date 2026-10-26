//! A test-only projection calls the real retained private source step; it is not a production export.
use super::*;
use crate::generic_execution_v1::{self as target, accounting_tests};
use zryna_ir::generic_v1::execution_v1::raw;
#[test]
fn birth_exact_and_first_extra_have_real_source_cleanup_and_pristine_retry() {
    actual("control-birth-call-loop");
}
#[test]
fn live_count_denial_leaves_parent_funded_for_real_source_cleanup() {
    actual("control-live-count-call-loop");
}
#[test]
fn expired_known_failure_fits_exact_source_frontier_without_prospective_owner_debt() {
    actual("control-expired-call-loop");
}
fn actual(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, _| {
            let candidates = program
                .functions()
                .filter(|f| {
                    f.parameters().len() == 0
                        && f.result_type() == raw::ValueType::Stored(1)
                        && !program.export_functions().contains(&f.index())
                })
                .collect::<Vec<_>>();
            assert_eq!(candidates.len(), 1, "unique actual sealed zero-argument i32 step");
            let step = candidates[0].index();
            let artifact =
                target::emit(program).expect("ordinary actual source before private projection");
            let record = row(case);
            let script = format!(
                "{}\nconst $record={},$sourceStep=$g{step};\n{}",
                artifact.source,
                record,
                include_str!("private_source.mjs")
            );
            let receipt = accounting_tests::private_boundary(
                &format!("control-source-frontier-{case}-{provider}"),
                &script,
            );
            assert_eq!(receipt["seededPrivateSourceEvidence"], true);
            let actual = receipt["observations"].as_array().expect("actual source observations");
            let expected =
                record["private_source_rows"].as_array().expect("pre-output source frontiers");
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(actual["value"], expected["value"]);
                assert_eq!(actual["error"], expected["error"]);
                assert_eq!(actual["after"], expected["after"]);
                assert_eq!(actual["exitError"], serde_json::Value::Null);
            }
            let retry = receipt["retry"].clone();
            assert_eq!(retry["value"], record["private_source_rows"][0]["value"]);
            assert_eq!(retry["error"], serde_json::Value::Null);
            assert_eq!(retry["exitError"], serde_json::Value::Null);
            let per_step = 4_194_304
                - record["private_source_rows"][0]["pre"][2]
                    .as_u64()
                    .expect("hand-derived exact seed");
            assert_eq!(retry["after"], serde_json::json!([1, 0, per_step, 0]));
        });
    }
}
