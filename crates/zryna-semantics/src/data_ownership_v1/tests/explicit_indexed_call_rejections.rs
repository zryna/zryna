use super::explicit_indexed_fixture::{Container, call_access_fixture, call_fixture};
use super::generic_vec_fixture::Element;
use super::*;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn explicit_indexed_call_rejections_require_exact_live_alias_authority() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [
            Element::I32,
            Element::String,
            Element::Struct,
            Element::Enum,
            Element::Array,
            Element::Vec,
        ] {
            for wrong_access in [false, true] {
                let (mut source, mut raw) = if wrong_access {
                    call_access_fixture(container, &element, false, true)
                } else {
                    call_fixture(container, &element, true)
                };
                let body = &mut raw.files[0].functions[0].body;
                let first_argument = body
                    .expressions
                    .iter()
                    .find_map(|e| {
                        if let RawExpressionKind::Call { arguments, .. } = &e.kind {
                            Some(arguments[0])
                        } else {
                            None
                        }
                    })
                    .expect("borrow call argument");
                let expression = &mut body.expressions[first_argument as usize];
                let at = expression.span;
                if !wrong_access {
                    let RawExpressionKind::Reference { name } = &mut expression.kind else {
                        panic!("alias reference");
                    };
                    assert_eq!(name.text, "loan");
                    source.replace_range(at.start as usize..at.end as usize, "next");
                    name.text = "next".into();
                }
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated hostile borrowed call");
                let expected = vec![zryna_diagnostics::Diagnostic::error_at(
                    "ZRYNA-M3017",
                    span(&sources, at),
                    "call argument requires a live borrow alias with exact referent and access",
                    "pass a matching lexical or call-frame borrow without moving or reborrowing it",
                )];
                for _ in 0..2 {
                    assert_eq!(
                        lower(pair_input(&syntax, &sources)).expect_err("invalid call borrow"),
                        expected,
                        "{container:?} {element:?} wrong_access={wrong_access}\n{source}"
                    );
                }
            }
        }
    }
}
