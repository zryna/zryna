//! Genuine nominal enum sources retain declaration, substituted payload and ownership authority.
use super::tests::with_claim;
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, resolve_declarations,
    tests::body_fixtures::project,
};
use zryna_ir::generic_v1::{Failure, owned_v2, raw};

mod acceptance;
mod negative;
mod packet;

const VALUES: &str =
    include_str!("../../../../../../tests/m7-generic-owned-nominal-enum/values.zry");
const MAIN: &str = include_str!("../../../../../../tests/m7-generic-owned-nominal-enum/main.zry");
const IMPORTS: &str =
    include_str!("../../../../../../tests/m7-generic-owned-nominal-enum/imports.txt");

fn with_packet(cross: bool, check: impl FnOnce(&[(&str, &str)])) {
    let text =
        if cross { format!("{}\n{MAIN}", IMPORTS.trim_end()) } else { format!("{VALUES}\n{MAIN}") };
    let files = if cross {
        vec![("main.zry", text.as_str()), ("values.zry", VALUES)]
    } else {
        vec![("main.zry", text.as_str())]
    };
    check(&files);
}

fn assert_original_types(files: &[(&str, &str)]) {
    let p = project(files);
    let entry = p.sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&p.syntax, &p.sources, entry).expect("source-bound input"),
    )
    .expect("exact original declarations before ownership admission");
    check_body_types(&declarations).expect("well-typed original bodies before ownership admission");
}

fn original_ownership_errors(source: &str) -> Vec<zryna_diagnostics::Diagnostic> {
    assert_original_types(&[("main.zry", source)]);
    let p = project(&[("main.zry", source)]);
    match owned_v2::check_original_bodies(&p.syntax, &p.sources) {
        Err(Failure::Diagnostics(errors)) => errors,
        Err(failure) => panic!("expected original ownership diagnostics: {failure:?}"),
        Ok(()) => panic!("invalid original ownership was admitted"),
    }
}
