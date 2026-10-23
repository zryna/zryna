//! Keep future nested amplification inputs; current exclusion cannot qualify native budgets.
use super::{SourceFileInput, SourceMap};
use zryna_ir::generic_v1::Failure;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, instantiation::owned_v4, resolve_declarations,
};

#[test]
fn future_nested_wrapped_fixture_rejects_before_native_authority() {
    const FUTURE_MINIMUM: u64 = 24 * 1024 * (2 * 16 + 3 + 6);
    assert_eq!(FUTURE_MINIMUM, 1_007_616, "unchanged future enum-only lower bound");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/generic_owned_v2/struct_tests/fixtures");
    for drops in [1, 24] {
        let source = std::fs::read_to_string(root.join(format!("wrapped-{drops}.zry")))
            .expect("unchanged future source");
        let bytes = std::fs::read(root.join(format!("wrapped-{drops}.json")))
            .expect("unchanged future raw DTO");
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
                .expect("source authority");
        let syntax = zryna_syntax::v5::verify_snapshot(
            zryna_syntax::v5::decode_snapshot(&bytes).expect("closed schema"),
            &sources,
        )
        .expect("complete original source authentication");
        let entry = sources.verify_file_id(0).expect("entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("semantic source binding"),
        )
        .expect("original declarations");
        let bodies = check_body_types(&declarations).expect("opaque original body types");
        let Failure::Diagnostics(errors) = owned_v4::discover(&bodies)
            .expect_err("nested nominal argument needs its separate full-contract proof")
        else {
            panic!("owning source exclusion required");
        };
        assert_eq!(errors[0].code(), "ZRYNA-M3008");
        // Deliberately never call layouts/verify_v4/MIR: this records only the current
        // private boundary. The preserved future native expectation remains unqualified.
    }
}
