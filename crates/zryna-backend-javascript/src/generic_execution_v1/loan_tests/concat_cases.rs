use super::{concat_source, run};
use zryna_ir::generic_v1::execution_v1::raw;

#[test]
fn genuine_concat_originals_retain_exact_operation_and_outer_failure() {
    for case in ["local-observation", "same-source", "nested-temporaries", "formal-observation"] {
        for provider in ["native", "typescript"] {
            concat_source::with_program(case, provider, |program, _wire| {
                let operations: Vec<_> = program
                    .operations()
                    .filter(|operation| {
                        matches!(operation.kind(), raw::Operation::StringConcat { .. })
                    })
                    .collect();
                assert_eq!(operations.len(), if case == "nested-temporaries" { 2 } else { 1 });
                for operation in operations {
                    assert!(
                        operation.failure().is_some(),
                        "unpublished concat destination excluded"
                    );
                }
            });
        }
    }
}
macro_rules! cases {
    ($($ordinary:ident, $observed:ident, $faults:ident, $case:literal;)*) => {$(
        #[test]
        fn $ordinary() { run::concat($case, false, false); }
        #[test]
        fn $observed() { run::concat($case, true, false); }
        #[test]
        fn $faults() { run::concat($case, true, true); }
    )*};
}
cases! {
    ordinary_local_observation, observed_local_observation, faults_local_observation, "local-observation";
    ordinary_same_source, observed_same_source, faults_same_source, "same-source";
    ordinary_nested_temporaries, observed_nested_temporaries, faults_nested_temporaries, "nested-temporaries";
    ordinary_formal_observation, observed_formal_observation, faults_formal_observation, "formal-observation";
}
