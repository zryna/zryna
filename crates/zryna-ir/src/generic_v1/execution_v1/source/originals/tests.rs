//! Owning original controls reuse already authenticated frozen source/DTO pairs.
use super::*;
use zryna_source::{SourceFileInput, UntrustedSpan};
const FIRST: &str = "m7-generic-full-profile-first-vertical";
const CLONE: &str = "m7-generic-struct-clone-v5";
fn project(group: &str, case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests").join(group);
    let text = std::fs::read_to_string(directory.join("literals").join(format!("{case}.zry")))
        .expect("frozen literal");
    let packet = std::fs::read(directory.join(format!("{case}-{provider}.json")))
        .expect("genuine named provider packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("literal source map");
    let syntax =
        v5::verify_snapshot(v5::decode_snapshot(&packet).expect("strict complete DTO"), &sources)
            .expect("genuine source syntax authority");
    (sources, syntax)
}
fn check_case(group: &str, case: &str, provider: &str) -> Result<(), Failure> {
    let (sources, syntax) = project(group, case, provider);
    let mut spent = 0;
    let inventory = inventory(&syntax, &sources, &mut spent)?;
    let originals = authenticate(&inventory, &syntax, &mut spent)?;
    check(&originals, &sources, &mut spent)
}
#[test]
fn five_frozen_originals_check_before_any_generated_program_or_layout() {
    for case in [
        "scalar-i32-clone",
        "scalar-bool-clone",
        "nominal-enum-clone",
        "option-nominal-enum-clone",
        "fixed-array-identity",
    ] {
        for provider in ["native", "typescript"] {
            check_case(FIRST, case, provider).expect("new symbolic original checker");
        }
    }
}
#[test]
fn unused_opaque_clone_is_exact_original_m7002_without_generated_context() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = project(FIRST, "opaque-clone-unused", provider);
        let mut spent = 0;
        let inventory = inventory(&syntax, &sources, &mut spent).expect("inventory");
        let originals = authenticate(&inventory, &syntax, &mut spent).expect("original binder");
        let before = spent;
        let Failure::Diagnostics(errors) =
            check(&originals, &sources, &mut spent).expect_err("opaque original cannot gain Clone")
        else {
            panic!("source diagnostic");
        };
        assert_eq!(spent, before, "failed original phase publishes no completed debit");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7002");
        assert_eq!(
            errors[0].primary_span(),
            Some(
                sources
                    .verify_span(UntrustedSpan { file: 0, start: 65, end: 70 })
                    .expect("original clone keyword")
            )
        );
    }
}
#[test]
fn broader_original_nominal_copy_phantom_and_stored_clone_controls() {
    for case in ["copy-struct", "fixed-member", "phantom-opaque", "nested-prefix", "repeat-source"]
    {
        for provider in ["native", "typescript"] {
            check_case(CLONE, case, provider).expect("real original member capabilities");
        }
    }
    for case in ["opaque-direct", "opaque-stored", "opaque-inactive"] {
        for provider in ["native", "typescript"] {
            let Failure::Diagnostics(errors) =
                check_case(CLONE, case, provider).expect_err("opaque capability remains false")
            else {
                panic!("source diagnostic");
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M7002");
        }
    }
}
#[test]
fn moved_and_conflicting_original_sources_reject_with_owning_diagnostics() {
    for provider in ["native", "typescript"] {
        for (group, case) in
            [(CLONE, "moved-source"), ("m7-generic-execution-loans", "overlap-exclusive")]
        {
            let Failure::Diagnostics(errors) = check_case(group, case, provider)
                .expect_err("moved owner or exact overlapping exclusive borrow")
            else {
                panic!("owning source diagnostic");
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M7007");
            assert!(
                errors[0].primary_span().is_some(),
                "exact original operation has source authority"
            );
        }
        // This older frozen literal uses a const root. It remains invalid at the
        // source owner; support for lexical loans cannot grant mutation permission.
        let Failure::Diagnostics(errors) = check_case(CLONE, "exclusive-source", provider)
            .expect_err("const root cannot grant an exclusive mutable referent")
        else {
            panic!("owning source diagnostic, not an unfinished implementation gate");
        };
        assert_eq!(errors.len(), 1);
        assert!(errors[0].code().starts_with("ZRYNA-M"));
        assert!(errors[0].primary_span().is_some());
    }
}
#[test]
fn shared_original_loan_preserves_observation_and_clone_of_its_owner() {
    for provider in ["native", "typescript"] {
        check_case(CLONE, "shared-owner", provider)
            .expect("shared borrowing preserves explicit original source Clone");
    }
}
#[test]
fn original_budget_exact_first_extra_is_atomic_and_pristine() {
    let (sources, syntax) = project(FIRST, "fixed-array-identity", "native");
    let mut spent = 0;
    let inventory = inventory(&syntax, &sources, &mut spent).expect("inventory");
    let originals = authenticate(&inventory, &syntax, &mut spent).expect("binder");
    let mut cost = 0;
    check(&originals, &sources, &mut cost).expect("cold original work");
    let mut exact = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS - cost;
    check(&originals, &sources, &mut exact).expect("exact credit");
    assert_eq!(exact, crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS);
    let mut short = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS - cost + 1;
    let before = short;
    check(&originals, &sources, &mut short).expect_err("first extra original work");
    assert_eq!(short, before);
    check(&originals, &sources, &mut 0).expect("pristine retry");
}
#[test]
fn stored_original_roots_include_primitives_closed_array_and_no_opaque() {
    let (sources, syntax) = project(FIRST, "fixed-array-identity", "native");
    let mut spent = 0;
    let inventory = inventory(&syntax, &sources, &mut spent).expect("inventory");
    let originals = authenticate(&inventory, &syntax, &mut spent).expect("binder");
    let roots = stored_roots(&originals, &mut spent).expect("actual original stored forest");
    for primitive in 0..3 {
        assert!(roots.iter().any(|key| key.as_slice() == [primitive]));
    }
    assert!(roots.iter().any(|key| key.as_slice() == [0x20, 2, 0, 0, 0, 1, 0, 0, 0, 1]));
    for key in roots {
        crate::generic_v1::keys::decode(&key, crate::generic_v1::keys::Domain::Type)
            .expect("all roots genuinely closed");
    }
}
#[test]
fn foreign_same_byte_source_map_rejects_inventory_before_any_debit() {
    let (_, syntax) = project(FIRST, "scalar-i32-clone", "native");
    let (foreign, _) = project(FIRST, "scalar-i32-clone", "native");
    let mut spent = 17;
    inventory(&syntax, &foreign, &mut spent).err().expect("foreign syntax source authority");
    assert_eq!(spent, 17);
}
#[test]
fn heap_source_key_sort_counts_work_and_rejects_duplicates() {
    let mut values = vec![vec![0x41, 9], vec![0x40, 3], vec![0x41, 1], vec![0x40, 1]];
    let mut spent = 0;
    sort::keys(&mut values, &mut spent).expect("metered heap order");
    assert_eq!(values, [vec![0x40, 1], vec![0x40, 3], vec![0x41, 1], vec![0x41, 9]]);
    assert!(spent > 0);
    sort::keys(&mut [vec![1], vec![1]], &mut 0).expect_err("duplicate complete key");
}
