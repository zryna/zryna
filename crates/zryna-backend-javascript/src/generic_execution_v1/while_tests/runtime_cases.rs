//! Same frozen original argument rows, not a fitted interpretation of target output.
const CASES: [&str; 9] = [
    "copy-assignment",
    "zero-multiple",
    "nested-lexical",
    "early-return",
    "body-loan-cleanup",
    "opaque-retained",
    "owned-root-replacement",
    "branch-copy-update",
    "match-condition",
];
#[test]
fn ordinary_while_and_assignments_execute_without_ambient_injection() {
    for case in CASES {
        super::run::case(case, false, false);
    }
}
#[test]
fn observed_while_and_assignments_preserve_dynamic_iteration_and_release_order() {
    for case in CASES {
        super::run::case(case, true, false);
    }
}
#[test]
fn dynamic_acquire_and_clone_faults_unwind_then_retry_pristine() {
    for case in CASES {
        super::run::case(case, true, true);
    }
}
