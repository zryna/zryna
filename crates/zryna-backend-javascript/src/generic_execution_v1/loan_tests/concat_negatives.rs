//! Exact owning original diagnostics for invalidated retained String reads.
use sha2::{Digest, Sha256};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};
fn rejected(case: &str, marker: &str, width: usize) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-string-concat");
    let text = std::fs::read(base.join("literals").join(format!("{case}.zry")))
        .expect("pre-provider literal");
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(base.join("SOURCE_EXPECTED_FREEZE.json")).expect("pre-provider oracle"),
    )
    .expect("frozen source oracle");
    let row = expected["cases"]
        .as_array()
        .expect("source rows")
        .iter()
        .find(|r| r["case"] == case)
        .expect("frozen hostile");
    assert_eq!(format!("{:x}", Sha256::digest(&text)), row["sha256"]);
    let text = String::from_utf8(text).expect("original UTF8");
    let start = text.rfind(marker).expect("exact ownership-invalidating operand");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("source identity");
    let span = sources
        .verify_span(UntrustedSpan {
            file: 0,
            start: u32::try_from(start).expect("small exact original"),
            end: u32::try_from(start + width).expect("small exact original"),
        })
        .expect("original offending operation span");
    for provider in ["native", "typescript"] {
        let packet = std::fs::read(base.join(format!("{case}-{provider}.json")))
            .expect("actual fresh provider packet");
        let syntax =
            verify_snapshot(decode_snapshot(&packet).expect("complete provider DTO"), &sources)
                .expect("exact syntax/source authentication");
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("invalidating retained read cannot issue original permission")
        else {
            panic!("owning diagnostic, not infrastructure/unfinished rejection");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7007", "owning read-availability permission");
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}
#[test]
fn second_operand_cannot_move_the_retained_first_source() {
    rejected("moved-first", "root));", 4);
}
#[test]
fn second_operand_cannot_pass_retained_first_referent_to_exclusive_call() {
    rejected("mutated-first", "change(loan)", 6);
}
