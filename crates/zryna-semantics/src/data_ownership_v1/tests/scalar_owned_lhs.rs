use super::{at, identifier, operator_fixture};
use crate::data_ownership_v1::tests::*;
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

#[path = "scalar_owned_lhs_fixtures.rs"]
mod fixtures;

#[derive(Clone, Copy)]
enum Left {
    Concat,
    Producer,
    Identity,
    AggregateClone,
    MissingNominal,
    WrongCaseNominal,
}

fn call(start: usize, text: &str, end: usize, arguments: Vec<u32>) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, end),
        kind: RawExpressionKind::Call {
            callee: identifier(text, start),
            open_paren_span: at(start + text.len(), start + text.len() + 1),
            arguments,
            close_paren_span: at(end - 1, end),
        },
    }
}

fn left_nodes(left: Left, start: usize, text: &str) -> Vec<RawExpressionSyntax> {
    match left {
        Left::Concat => vec![
            RawExpressionSyntax {
                span: at(start + 7, start + 10),
                kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
            },
            RawExpressionSyntax {
                span: at(start + 12, start + 15),
                kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".into() },
            },
            call(start, "concat", start + text.len(), vec![2, 3]),
        ],
        Left::Producer => vec![call(start, "producer", start + text.len(), vec![])],
        Left::Identity => vec![
            call(start + 9, "producer", start + text.len() - 1, vec![]),
            call(start, "identity", start + text.len(), vec![2]),
        ],
        Left::AggregateClone => vec![
            RawExpressionSyntax {
                span: at(start + 6, start + 7),
                kind: RawExpressionKind::Reference { name: identifier("p", start + 6) },
            },
            RawExpressionSyntax {
                span: at(start, start + text.len()),
                kind: RawExpressionKind::Clone {
                    keyword_span: at(start, start + 5),
                    open_paren_span: at(start + 5, start + 6),
                    value: 2,
                    close_paren_span: at(start + text.len() - 1, start + text.len()),
                },
            },
        ],
        Left::MissingNominal | Left::WrongCaseNominal => {
            let name = if matches!(left, Left::MissingNominal) { "Absent" } else { "parcel" };
            vec![RawExpressionSyntax {
                span: at(start, start + text.len()),
                kind: RawExpressionKind::StructConstruction {
                    type_name: identifier(name, start),
                    open_paren_span: at(start + 6, start + 7),
                    open_brace_span: at(start + 7, start + 8),
                    fields: vec![],
                    close_brace_span: at(start + 8, start + 9),
                    close_paren_span: at(start + 9, start + 10),
                },
            }]
        }
    }
}

fn fixture(left: Left) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = operator_fixture(true);
    let original = raw.files[0].functions[0].body.expressions[2].span;
    let start = usize::try_from(original.start).expect("left offset");
    let end = usize::try_from(original.end).expect("left end");
    assert_eq!(&source[start..end], "true");
    let text = match left {
        Left::Concat => "concat(\"a\", \"b\")",
        Left::Producer => "producer()",
        Left::Identity => "identity(producer())",
        Left::AggregateClone => "clone(p)",
        Left::MissingNominal => "Absent({})",
        Left::WrongCaseNominal => "parcel({})",
    };
    source.replace_range(start..end, text);
    raw =
        shift_snapshot(raw, original.end, u32::try_from(text.len() - 4).expect("positive growth"));
    let nodes = left_nodes(left, start, text);
    let extra = u32::try_from(nodes.len() - 1).expect("small expression packet");
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 9);
    for expression in &mut body.expressions[3..] {
        fixtures::offset_inputs(expression, 3, extra);
    }
    body.expressions.splice(2..3, nodes);
    let RawExpressionKind::Subtraction { lhs, .. } =
        &mut body.expressions[usize::try_from(4 + extra).expect("subtract")].kind
    else {
        panic!("subtraction")
    };
    *lhs = 2 + extra;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value += extra;
    if matches!(left, Left::Producer | Left::Identity) {
        mixed_string_calls::append_string_callees(&mut source, &mut raw);
    }
    if matches!(left, Left::AggregateClone) {
        fixtures::prepend_pair_local(&mut source, &mut raw);
    }
    (source, raw)
}

#[test]
fn mixed_inferred_owned_left_does_not_hide_missing_scalar_right() {
    for left in [Left::Concat, Left::Producer, Left::Identity, Left::AggregateClone] {
        let (source, raw) = fixture(left);
        let missing = raw.files[0].functions[0]
            .body
            .expressions
            .iter()
            .find_map(|e| {
                if let RawExpressionKind::Reference { name } = &e.kind {
                    (name.text == "lost").then_some(name.span)
                } else {
                    None
                }
            })
            .expect("exact missing RHS");
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("authenticated owned left and missing right");
        let expected = vec![Diagnostic::error_at(
            "ZRYNA-M3002",
            span(&sources, missing),
            "name 'lost' is not declared",
            "reference one exact parameter, local, or match payload binding",
        )];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources))
                    .expect_err("RHS resolution precedes operand mismatch"),
                expected
            );
        }
    }
}

#[test]
fn mixed_inferred_nominal_left_keeps_exact_name_diagnostic_before_missing_right() {
    for left in [Left::MissingNominal, Left::WrongCaseNominal] {
        let (source, raw) = fixture(left);
        let RawExpressionKind::StructConstruction { type_name, .. } =
            &raw.files[0].functions[0].body.expressions[2].kind
        else {
            panic!("nominal LHS")
        };
        let (name, at) = (type_name.text.clone(), type_name.span);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated nominal failure source");
        let expected = vec![Diagnostic::error_at(
            "ZRYNA-M3016",
            span(&sources, at),
            format!("'{name}' is not an exact module-local owned struct"),
            "construct one exact supported struct type",
        )];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("LHS name authority first"),
                expected
            );
        }
    }
}
#[test]
fn mixed_inferred_owned_left_is_rejected_after_valid_scalar_right() {
    for left in [Left::Concat, Left::Producer, Left::Identity, Left::AggregateClone] {
        let (mut source, mut raw) = fixture(left);
        let body = &mut raw.files[0].functions[0].body;
        let missing = body
            .expressions
            .iter_mut()
            .find(|expression| {
                matches!(&expression.kind, RawExpressionKind::Reference { name } if name.text == "lost")
            })
            .expect("one fixed missing right operand");
        let start = usize::try_from(missing.span.start).expect("right offset");
        let end = usize::try_from(missing.span.end).expect("right end");
        assert_eq!(&source[start..end], "lost");
        source.replace_range(start..end, "1234");
        missing.kind = RawExpressionKind::I32Literal { spelling: "1234".into() };
        let operation_span = body
            .expressions
            .iter()
            .find_map(|expression| {
                matches!(expression.kind, RawExpressionKind::Subtraction { .. })
                    .then_some(expression.span)
            })
            .expect("fixed subtraction");
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources)
            .expect("authenticated owned left and valid integer right");
        let expected = vec![Diagnostic::error_at(
            "ZRYNA-M3007",
            span(&sources, operation_span),
            "left operand has a different exact aggregate type",
            "use a value with the exact declared type",
        )];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources))
                    .expect_err("owned left cannot be used as an integer operand"),
                expected
            );
        }
    }
}
