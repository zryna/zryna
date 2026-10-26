//! Unchanged retained-read hostile literals stay source-owned before target admission.
use super::concat_source;
use zryna_source::UntrustedSpan;
fn rejected(case: &str, marker: &str, width: usize) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-string-concat/literals");
    let text = std::fs::read_to_string(base.join(format!("{case}.zry")))
        .expect("unchanged genuine hostile source");
    let start = text.rfind(marker).expect("frozen invalidating occurrence");
    for provider in concat_source::PROVIDERS {
        let (sources, syntax) = concat_source::project(case, provider);
        let span = sources
            .verify_span(UntrustedSpan {
                file: 0,
                start: u32::try_from(start).expect("small original"),
                end: u32::try_from(start + width).expect("small original"),
            })
            .expect("original source-owned span");
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("invalidated retained read must not issue permission")
        else {
            panic!("exact source failure, not infrastructure rejection");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7007");
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}
#[test]
fn second_operand_cannot_move_first_source() {
    rejected("moved-first", "root));", 4);
}
#[test]
fn second_operand_cannot_mutate_first_referent() {
    rejected("mutated-first", "change(loan)", 6);
}
