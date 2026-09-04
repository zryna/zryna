use super::*;
#[path = "scalar_owned_lhs.rs"]
mod owned_lhs;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::ValueIdentity;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn identifier(name: &str, start: usize) -> RawIdentifierSyntax {
    RawIdentifierSyntax { text: name.into(), span: at(start, start + name.len()) }
}

fn insertion(
    source: &mut String,
    raw: RawProjectSyntaxSnapshot,
    start: usize,
    text: &str,
) -> RawProjectSyntaxSnapshot {
    source.insert_str(start, text);
    shift_snapshot(
        raw,
        u32::try_from(start).expect("offset"),
        u32::try_from(text.len()).expect("length"),
    )
}

// Two fixed expression shapes, not a test parser or a semantic/type estimator.
pub(in crate::data_ownership_v1) fn operator_fixture(
    bad_right: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (source, raw) = mixed_construction::mixed_fixture(false);
    let (mut source, mut raw) = scalar_fields(source, raw);
    let RawExpressionKind::StructConstruction { close_brace_span, .. } =
        raw.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("Parcel initializer")
    };
    let start = usize::try_from(close_brace_span.start).expect("span");
    let text = if bad_right {
        ", count: true - lost, flag: true === false"
    } else {
        ", count: 7 - 3, flag: true === false"
    };
    raw = insertion(&mut source, raw, start, text);
    let lhs = start + 9;
    let rhs = start + text.find(if bad_right { "lost" } else { "3" }).expect("right operand");
    let minus = start + text.find('-').expect("subtraction token");
    let flag = start + text.find("flag:").expect("Bool field");
    let bool_left = flag + 6;
    let equal = start + text.find("===").expect("equality token");
    let bool_right = start + text.find("false").expect("false operand");
    let body = &mut raw.files[0].functions[0].body;
    let mut structure = body.expressions.pop().expect("last Parcel expression");
    assert_eq!(body.expressions.len(), 2);
    body.expressions.extend([
        RawExpressionSyntax {
            span: at(lhs, lhs + if bad_right { 4 } else { 1 }),
            kind: if bad_right {
                RawExpressionKind::BoolLiteral { value: true }
            } else {
                RawExpressionKind::I32Literal { spelling: "7".into() }
            },
        },
        RawExpressionSyntax {
            span: at(rhs, rhs + if bad_right { 4 } else { 1 }),
            kind: if bad_right {
                RawExpressionKind::Reference { name: identifier("lost", rhs) }
            } else {
                RawExpressionKind::I32Literal { spelling: "3".into() }
            },
        },
        RawExpressionSyntax {
            span: at(lhs, rhs + if bad_right { 4 } else { 1 }),
            kind: RawExpressionKind::Subtraction {
                operator_span: at(minus, minus + 1),
                lhs: 2,
                rhs: 3,
            },
        },
        RawExpressionSyntax {
            span: at(bool_left, bool_left + 4),
            kind: RawExpressionKind::BoolLiteral { value: true },
        },
        RawExpressionSyntax {
            span: at(bool_right, bool_right + 5),
            kind: RawExpressionKind::BoolLiteral { value: false },
        },
        RawExpressionSyntax {
            span: at(bool_left, bool_right + 5),
            kind: RawExpressionKind::Equal { operator_span: at(equal, equal + 3), lhs: 5, rhs: 6 },
        },
    ]);
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel fields")
    };
    for (name, start, end, colon, value) in [
        ("count", start + 2, rhs + if bad_right { 4 } else { 1 }, 5, 4),
        ("flag", flag, bool_right + 5, 4, 7),
    ] {
        fields.push(RawFieldInitializer {
            span: at(start, end),
            kind: RawFieldInitializerKind::Explicit {
                name: identifier(name, start),
                colon_span: at(start + colon, start + colon + 1),
                value,
            },
        });
    }
    body.expressions.push(structure);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("constructor return")
    };
    *value = 8;
    (source, raw)
}

#[test]
fn mixed_owned_copy_children_preserve_subtraction_and_bool_equality_operands() {
    let (source, snapshot) = operator_fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated mixed Copy operators");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("mixed Copy operators full IR");
        let module = program.modules().next().expect("module");
        let function = module.functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::I32Sub,
                VerifiedInstructionKind::BoolLiteral,
                VerifiedInstructionKind::BoolLiteral,
                VerifiedInstructionKind::Eq,
                VerifiedInstructionKind::StructConstruct,
            ]
        );
        assert_eq!(instructions[2].i32_literal(), Some(7));
        assert_eq!(instructions[3].i32_literal(), Some(3));
        assert_eq!(instructions[5].bool_literal(), Some(true));
        assert_eq!(instructions[6].bool_literal(), Some(false));
        for (index, operands) in
            [(1, vec![0]), (4, vec![2, 3]), (7, vec![5, 6]), (8, vec![1, 4, 7])]
        {
            assert_eq!(
                instructions[index].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                operands
            );
        }
        assert_eq!(function.places().count(), 3);
        assert_eq!(function.cleanup_plans().count(), 3);
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            [8]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(observed[1].3, [0]);
        assert!(observed.iter().enumerate().all(|(i, value)| i == 1 || value.3.is_empty()));
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

#[test]
fn mixed_copy_wrong_left_type_does_not_hide_undeclared_right_operand() {
    let (source, snapshot) = operator_fixture(true);
    let sources = sources_for(&source);
    let missing = snapshot.files[0].functions[0].body.expressions[3].span;
    let syntax =
        verify_snapshot(snapshot, &sources).expect("authenticated competing operand errors");
    let expected = [Diagnostic::error_at(
        "ZRYNA-M3002",
        span(&sources, missing),
        "name 'lost' is not declared",
        "reference one exact parameter, local, or match payload binding",
    )];
    for _ in 0..2 {
        let actual = lower(pair_input(&syntax, &sources))
            .expect_err("right resolution precedes left type check");
        assert_eq!(actual, expected);
    }
}

fn scalar_fields(
    mut source: String,
    mut raw: RawProjectSyntaxSnapshot,
) -> (String, RawProjectSyntaxSnapshot) {
    let declaration_end =
        usize::try_from(raw.files[0].data_declarations[0].span.end).expect("span") - 1;
    raw = insertion(&mut source, raw, declaration_end, " count: i32; flag: bool;");
    let count_decl = declaration_end + 1;
    let flag_decl = declaration_end + 13;
    assert_eq!(&source[count_decl..count_decl + 11], "count: i32;");
    assert_eq!(&source[flag_decl..flag_decl + 11], "flag: bool;");
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    raw.files[0].type_syntax.extend([
        RawTypeSyntax {
            span: at(count_decl + 7, count_decl + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".into(),
                    span: at(count_decl + 7, count_decl + 10),
                },
            },
        },
        RawTypeSyntax {
            span: at(flag_decl + 6, flag_decl + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "bool".into(),
                    span: at(flag_decl + 6, flag_decl + 10),
                },
            },
        },
    ]);
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Parcel declaration")
    };
    for (name, start, colon, ty) in [("count", count_decl, 5, 5), ("flag", flag_decl, 4, 6)] {
        fields.push(RawDataField {
            span: at(start, start + 11),
            name: identifier(name, start),
            colon_span: at(start + colon, start + colon + 1),
            type_syntax: ty,
            semicolon_span: at(start + 10, start + 11),
        });
    }
    (source, raw)
}
