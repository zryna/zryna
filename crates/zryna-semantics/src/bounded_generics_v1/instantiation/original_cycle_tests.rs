//! Fixed original layout rejections and intentional private-v4 original-validation phase order.
use super::{BodyTypeContext, InstantiationFailure};
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, resolve_declarations,
    tests::body_fixtures::project,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::Failure;
mod controls;
mod literals;
mod phases;

const NODE: &str = "interface Node<T extends ZrynaValue> extends ZrynaStruct { next: Node<T>; }";
const NEST: &str =
    "interface Nest<T extends ZrynaValue> extends ZrynaStruct { next: Vec<Nest<Vec<T>>>; }";
const MOVE: &str =
    "function consumeTwice<U extends ZrynaValue>(value:U):U {const once:U=value;return value;}";

fn with_bodies(source: &str, test: impl FnOnce(&BodyTypeContext<'_, '_>)) {
    let input = project(&[("main.zry", source)]);
    let declarations = resolve_declarations(
        SemanticInput::try_new(
            &input.syntax,
            &input.sources,
            input.sources.verify_file_id(0).expect("entry"),
        )
        .expect("exact original source authority"),
    )
    .expect("original declarations and names");
    let bodies = check_body_types(&declarations).expect("all original opaque bodies typed first");
    test(&bodies);
}

fn plain_errors(bodies: &BodyTypeContext<'_, '_>) -> Vec<Diagnostic> {
    let InstantiationFailure::Diagnostics(errors) = super::discover(bodies)
        .expect_err("original phase rejection must not issue an instance context")
    else {
        panic!("owning diagnostics required")
    };
    errors
}
fn v4_errors(bodies: &BodyTypeContext<'_, '_>) -> Vec<Diagnostic> {
    let Failure::Diagnostics(errors) = super::owned_v4::discover(bodies)
        .expect_err("original rejection must not issue a private v4 context")
    else {
        panic!("owning diagnostics required")
    };
    errors
}
fn observed(errors: &[Diagnostic]) -> Vec<(String, u32, u32)> {
    errors
        .iter()
        .map(|error| {
            let span = error.primary_span().expect("authenticated original phase witness");
            assert_eq!(span.file().index(), 0);
            (error.code().into(), span.start(), span.end())
        })
        .collect()
}
fn one(errors: &[Diagnostic], code: &str, start: u32, end: u32) {
    assert_eq!(observed(errors), [(code.into(), start, end)]);
}
