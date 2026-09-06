use super::generic_static_fixture::{Case, Shape, fixture};
use super::*;

#[test]
fn handle_static_source_rejects_wrong_rhs_repeated_move_and_moved_target() {
    for case in [Case::WrongType, Case::RepeatedMove, Case::MovedTarget] {
        let (source, raw) = fixture(Shape::HandleStruct, case);
        let body = &raw.files[0].functions[0].body;
        let (at, code, message, guidance) = match case {
            Case::WrongType => (
                body.expressions.iter().rfind(|expression| matches!(&expression.kind,
                    zryna_syntax::v4::RawExpressionKind::Reference { name } if name.text == "wrong"
                )).expect("wrong RHS").span,
                "ZRYNA-M3016",
                "aggregate operand has the wrong exact type",
                "use the exact declared field, element, local, or result type",
            ),
            Case::RepeatedMove => (
                body.expressions.last().expect("second projection").span,
                "ZRYNA-M3014",
                "owned projection is unavailable or overlaps an already moved subobject",
                "move each owned field or fixed-array element at most once",
            ),
            Case::MovedTarget => (
                body.statements.iter().find(|statement| matches!(statement.kind,
                    RawStatementKind::Assignment { .. }
                )).expect("replacement").span,
                "ZRYNA-M3014",
                "static replacement target is immutable, unavailable, or consumed during preparation",
                "retain the exact complete mutable subobject until replacement commits",
            ),
            _ => unreachable!(),
        };
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated ownership negative");
        let first = lower(pair_input(&syntax, &sources)).expect_err("invalid static transfer");
        assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("repeat rejection"));
        assert_eq!(
            first,
            [zryna_diagnostics::Diagnostic::error_at(code, span(&sources, at), message, guidance,)],
            "{case:?}: {source}"
        );
        let (source, raw) = fixture(Shape::HandleStruct, Case::Move);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery");
        lower(pair_input(&syntax, &sources)).expect("valid transfer after rejected input");
    }
}
