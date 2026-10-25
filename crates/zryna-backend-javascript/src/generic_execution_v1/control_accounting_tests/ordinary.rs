//! Public ordinary modules are unchanged test subjects, with no seeded counters or ambient runtime.
use super::*;
use crate::generic_execution_v1::{self as target, accounting_tests};
#[test]
fn control_birth_public_zero_small_and_same_module_reset() {
    actual("control-birth-call-loop");
}
#[test]
fn all_live_count_roles_public_zero_small_and_same_module_reset() {
    actual("control-live-count-call-loop");
}
#[test]
fn expired_branch_public_zero_small_and_same_module_reset() {
    actual("control-expired-call-loop");
}
fn actual(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let artifact =
                target::emit(program).expect("actual ordinary NEW control source target");
            assert_eq!(
                artifact,
                target::emit(program).expect("deterministic audited ordinary bytes")
            );
            for forbidden in [
                "globalThis",
                "Symbol.for",
                "import ",
                "import(",
                "require(",
                "eval(",
                "new Function",
            ] {
                assert!(
                    !artifact.source.contains(forbidden),
                    "private ordinary authority {forbidden}"
                );
            }
            let file = std::env::temp_dir()
                .join(format!("zryna-control-public-{}-{case}-{provider}.mjs", std::process::id()));
            std::fs::write(&file, &artifact.source).expect("retain actual ordinary artifact");
            let script = format!(
                "const $artifactPath={},$record={};\n{}",
                serde_json::to_string(&file.to_string_lossy()).expect("bounded test path"),
                row(case),
                include_str!("ordinary.mjs")
            );
            let receipt = accounting_tests::private_boundary(
                &format!("control-public-{case}-{provider}"),
                &script,
            );
            assert_eq!(receipt["exports"], serde_json::json!(["entry"]));
            let observations =
                receipt["observations"].as_array().expect("actual observations before checks");
            assert_eq!(observations.len(), 3);
            let expected = row(case);
            let rows = expected["public_rows"].as_array().expect("frozen public rows");
            let mut index = 0;
            for row in rows {
                for _ in 0..row["repeats"].as_u64().expect("finite repeat") {
                    assert_eq!(observations[index]["error"], serde_json::Value::Null);
                    assert_eq!(observations[index]["value"], row["value"]);
                    index += 1;
                }
            }
        });
    }
}
