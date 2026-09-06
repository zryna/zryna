use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::indexed_handle_source::elements;
use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawExpressionKind;

fn expected(action: Action) -> (&'static str, &'static str, &'static str) {
    match action {
        Action::Read => (
            "ZRYNA-M3017",
            "borrowed element cannot be moved, implicitly copied, or read with another type",
            "read a Copy element or explicitly clone its exact owned referent",
        ),
        Action::Replace => (
            "ZRYNA-M3017",
            "indexed replacement requires active exclusive authority",
            "assign through a live BorrowMut alias",
        ),
        _ => (
            "ZRYNA-M3014",
            "owner access conflicts with an active borrow",
            "finish the indexed operation before accessing or consuming its container",
        ),
    }
}

#[test]
fn indexed_handle_rejections_pin_exact_diagnostics_and_valid_recovery() {
    for container in [Container::Array(2), Container::Vec] {
        for element in elements() {
            for action in [
                Action::Read,
                Action::Replace,
                Action::OwnerMove,
                Action::OwnerReplace,
                Action::Conflict,
            ] {
                let (source, raw) = fixture(container, &element, false, action, None);
                let body = &raw.files[0].functions[0].body;
                let statement = &body.statements[body.blocks[1].statements[1] as usize];
                let id = match statement.kind {
                    RawStatementKind::LocalDeclaration { initializer, .. } => initializer,
                    RawStatementKind::Assignment { value, .. } => value,
                    _ => panic!("rejected statement"),
                };
                let id = match body.expressions[id as usize].kind {
                    RawExpressionKind::BorrowMut { value, .. } => value,
                    _ => id,
                };
                let at = body.expressions[id as usize].span;
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated rejection");
                let (code, message, guidance) = expected(action);
                let expected =
                    vec![Diagnostic::error_at(code, span(&sources, at), message, guidance)];
                for _ in 0..2 {
                    assert_eq!(
                        lower(pair_input(&syntax, &sources)).expect_err("handle exclusion"),
                        expected,
                        "{container:?} {element:?} {action:?}\n{source}"
                    );
                }
                let (source, raw) = fixture(container, &element, false, Action::Clone, None);
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery");
                lower(pair_input(&syntax, &sources)).expect("valid compile after rejection");
            }
        }
    }
}

#[test]
fn indexed_handle_rejections_wrong_rhs_type_is_source_authenticated() {
    for container in [Container::Array(2), Container::Vec] {
        for element in elements() {
            let (mut source, mut raw) = fixture(container, &element, true, Action::Replace, None);
            let body = &mut raw.files[0].functions[0].body;
            let RawStatementKind::Assignment { value, .. } =
                body.statements[body.blocks[1].statements[1] as usize].kind
            else {
                panic!("replacement");
            };
            let expression = &mut body.expressions[value as usize];
            let at = expression.span;
            assert_eq!(&source[at.start as usize..at.end as usize], "next");
            source.replace_range(at.start as usize..at.end as usize, "true");
            expression.kind = RawExpressionKind::BoolLiteral { value: true };
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("real Bool RHS, not forged type");
            let errors = lower(pair_input(&syntax, &sources)).expect_err("wrong RHS type");
            assert_eq!(
                errors,
                vec![Diagnostic::error_at(
                    "ZRYNA-M3016",
                    span(&sources, at),
                    "expression is outside private owned Struct/Enum/FixedArray lowering",
                    "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors"
                )]
            );
            assert_eq!(
                errors,
                lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection")
            );
        }
    }
}

#[test]
fn indexed_handle_rejections_lowerable_owned_rhs_requires_exact_referent_type() {
    for container in [Container::Array(2), Container::Vec] {
        let (source, raw) =
            fixture(container, &Element::String, true, Action::WrongOwnedType, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated String RHS control");
        lower(pair_input(&syntax, &sources))
            .expect("identical owned RHS lowers for String referent");
        for element in elements() {
            let (source, raw) = fixture(container, &element, true, Action::WrongOwnedType, None);
            let body = &raw.files[0].functions[0].body;
            let RawStatementKind::Assignment { value, .. } =
                body.statements[body.blocks[1].statements[1] as usize].kind
            else {
                panic!("owned replacement");
            };
            let at = body.expressions[value as usize].span;
            assert_eq!(&source[at.start as usize..at.end as usize], "next");
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated owned type mismatch");
            let expected = vec![Diagnostic::error_at(
                "ZRYNA-M3016",
                span(&sources, at),
                "aggregate operand has the wrong exact type",
                "use the exact declared field, element, local, or result type",
            )];
            for _ in 0..2 {
                assert_eq!(
                    lower(pair_input(&syntax, &sources)).expect_err("wrong exact referent"),
                    expected
                );
            }
            let (source, raw) = fixture(container, &element, true, Action::Replace, None);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated exact-type recovery");
            lower(pair_input(&syntax, &sources))
                .expect("valid same-type replacement after rejection");
        }
    }
}
