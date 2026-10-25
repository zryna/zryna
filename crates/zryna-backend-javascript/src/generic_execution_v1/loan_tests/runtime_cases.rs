use super::run;

macro_rules! cases {
    ($($ordinary:ident, $observed:ident, $case:literal;)*) => {$(
        #[test]
        fn $ordinary() { run::loan($case, false, false); }
        #[test]
        fn $observed() { run::loan($case, true, false); }
    )*};
}
cases! {
    ordinary_formal_copy, observed_formal_copy, "formal-copy";
    ordinary_opaque_forward, observed_opaque_forward, "opaque-forward";
    ordinary_shared_reborrow_clone, observed_shared_reborrow_clone, "shared-reborrow-clone";
    ordinary_exclusive_reborrow_push, observed_exclusive_reborrow_push, "exclusive-reborrow-push";
    ordinary_generic_replace, observed_generic_replace, "generic-replace";
    ordinary_static_disjoint, observed_static_disjoint, "static-disjoint";
    ordinary_branch_cleanup, observed_branch_cleanup, "branch-cleanup";
    ordinary_fallible_cleanup, observed_fallible_cleanup, "fallible-cleanup";
    ordinary_referent_catalogue, observed_referent_catalogue, "referent-catalogue";
}
#[test]
fn observed_generic_replace_all_original_faults_and_retry() {
    run::loan("generic-replace", true, true);
}
#[test]
fn observed_static_disjoint_has_no_allocator_effects() {
    run::loan("static-disjoint", true, true);
}
#[test]
fn observed_fallible_cleanup_all_original_faults_and_retry() {
    run::loan("fallible-cleanup", true, true);
}
