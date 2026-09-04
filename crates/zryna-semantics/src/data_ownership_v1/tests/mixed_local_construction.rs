use super::*;
use zryna_syntax::v4::{RawExpressionKind, RawTypeSyntax, RawTypeSyntaxKind};

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn vec_type(start: usize, end: usize, argument: u32) -> RawTypeSyntax {
    RawTypeSyntax {
        span: at(start, end),
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: at(start, start + 3),
            less_than_span: at(start + 3, start + 4),
            argument,
            greater_than_span: at(end - 1, end),
        },
    }
}

// Fixed edits to an existing source fixture, not a parser or semantic oracle.
fn local_vec_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_construction::mixed_fixture(true);
    let original = raw.files[0].type_syntax[2].span;
    source.insert(original.end as usize, '>');
    raw = shift_snapshot(raw, original.end, 1);
    raw.files[0].type_syntax[2].span.end = original.end;
    let RawTypeSyntaxKind::Vec { greater_than_span, .. } = &mut raw.files[0].type_syntax[2].kind
    else {
        panic!("original Vec result");
    };
    greater_than_span.end = original.end;
    source.insert_str(original.start as usize, "Vec<");
    raw = shift_snapshot(raw, original.start, 4);
    raw.files[0].type_syntax.push(vec_type(original.start as usize, original.end as usize + 5, 2));
    raw.files[0].functions[0].result_type = 5;

    let start = source.find("return ").expect("return");
    let prefix = "const items: Vec<Parcel> = ";
    source.replace_range(start..start + 7, prefix);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + 7).expect("fixed fixture structure"),
        u32::try_from(prefix.len() - 7).expect("bounded fixture offset"),
    );
    let insertion = source.rfind('}').expect("body close");
    let suffix = "return Vec<Vec<Parcel>>([items]); ";
    source.insert_str(insertion, suffix);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("bounded fixture offset"),
        u32::try_from(suffix.len()).expect("bounded fixture offset"),
    );
    let local_type = start + prefix.find("Vec<Parcel>").expect("fixed fixture structure");
    let named_start = local_type + 4;
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(named_start, named_start + 6),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Parcel".into(),
                span: at(named_start, named_start + 6),
            },
        },
    });
    raw.files[0].type_syntax.push(vec_type(local_type, local_type + 11, 6));
    let root = insertion + 7;
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(root + 8, root + 14),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Parcel".into(), span: at(root + 8, root + 14) },
        },
    });
    raw.files[0].type_syntax.push(vec_type(root + 4, root + 15, 8));
    raw.files[0].type_syntax.push(vec_type(root, root + 16, 9));
    let body = &mut raw.files[0].functions[0].body;
    let old_semicolon =
        source[start..insertion].find(';').expect("fixed fixture structure") + start;
    body.statements[0] = RawStatementSyntax {
        span: at(start, old_semicolon + 1),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: at(start, start + 5),
            mutable: false,
            name: RawIdentifierSyntax { text: "items".into(), span: at(start + 6, start + 11) },
            type_syntax: 7,
            equals_span: at(start + prefix.len() - 2, start + prefix.len() - 1),
            initializer: 2,
            semicolon_span: at(old_semicolon, old_semicolon + 1),
        },
    };
    body.expressions.push(RawExpressionSyntax {
        span: at(root + 18, root + 23),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "items".into(), span: at(root + 18, root + 23) },
        },
    });
    body.expressions.push(RawExpressionSyntax {
        span: at(root, root + 25),
        kind: RawExpressionKind::VecConstruction {
            type_syntax: 10,
            open_paren_span: at(root + 16, root + 17),
            open_bracket_span: at(root + 17, root + 18),
            elements: vec![3],
            close_bracket_span: at(root + 23, root + 24),
            close_paren_span: at(root + 24, root + 25),
        },
    });
    body.statements.push(RawStatementSyntax {
        span: at(insertion, root + 26),
        kind: RawStatementKind::Return {
            keyword_span: at(insertion, insertion + 6),
            value: 4,
            semicolon_span: at(root + 25, root + 26),
        },
    });
    body.blocks[0].statements = vec![0, 1];
    (source, raw)
}

pub(super) fn string_clone_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_construction::mixed_fixture(true);
    let start = source.find("return ").expect("fixed fixture structure");
    let prefix = "const text: String = \"a\"; ";
    source.insert_str(start, prefix);
    raw = shift_snapshot(
        raw,
        u32::try_from(start).expect("bounded fixture offset"),
        u32::try_from(prefix.len()).expect("bounded fixture offset"),
    );
    let literal = source.rfind("\"a\"").expect("fixed fixture structure");
    source.replace_range(literal..literal + 3, "clone(text)");
    raw = shift_snapshot(raw, u32::try_from(literal + 3).expect("bounded fixture offset"), 8);
    let local_type = start + prefix.find("String").expect("fixed fixture structure");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(local_type, local_type + 6),
        kind: RawTypeSyntaxKind::String { keyword_span: at(local_type, local_type + 6) },
    });
    let body = &mut raw.files[0].functions[0].body;
    let string_start = start + prefix.find("\"a\"").expect("fixed fixture structure");
    let mut structure = body.expressions[1].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("struct")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind
    else {
        panic!("field")
    };
    *value = 2;
    let mut vector = body.expressions[2].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("Vec")
    };
    *elements = vec![3];
    body.expressions = vec![
        RawExpressionSyntax {
            span: at(string_start, string_start + 3),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        RawExpressionSyntax {
            span: at(literal + 6, literal + 10),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "text".into(),
                    span: at(literal + 6, literal + 10),
                },
            },
        },
        RawExpressionSyntax {
            span: at(literal, literal + 11),
            kind: RawExpressionKind::Clone {
                keyword_span: at(literal, literal + 5),
                open_paren_span: at(literal + 5, literal + 6),
                value: 1,
                close_paren_span: at(literal + 10, literal + 11),
            },
        },
        structure,
        vector,
    ];
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 4;
    body.statements.insert(
        0,
        RawStatementSyntax {
            span: at(start, start + prefix.len() - 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(start, start + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "text".into(), span: at(start + 6, start + 10) },
                type_syntax: 5,
                equals_span: at(string_start - 2, string_start - 1),
                initializer: 0,
                semicolon_span: at(string_start + 3, string_start + 4),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1];
    (source, raw)
}

fn expected_instructions(clone: bool) -> Vec<VerifiedInstructionKind> {
    if clone {
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringClone,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    } else {
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::VecConstruct,
        ]
    }
}

fn assert_local_program(clone: bool) {
    let (source, raw) = if clone { string_clone_fixture() } else { local_vec_fixture() };
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated local mixed source");
    let input = pair_input(&syntax, &sources);
    let expected = expected_instructions(clone);
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(input).expect("local mixed program reaches independent IR verification");
        let function = program
            .modules()
            .next()
            .expect("fixed fixture structure")
            .functions()
            .next()
            .expect("fixed fixture structure");
        let block = function.blocks().next().expect("fixed fixture structure");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(), expected);
        assert_eq!(function.places().count(), if clone { 5 } else { 6 });
        assert!(function.places().all(|place| !place.is_copy()));
        let cleanup = instructions
            .iter()
            .map(|instruction| {
                instruction
                    .derived_drop_actions()
                    .map(|action| action.root().index())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            cleanup,
            if clone {
                vec![vec![], vec![], vec![1], vec![], vec![3, 1]]
            } else {
                vec![vec![], vec![], vec![1], vec![], vec![], vec![4]]
            }
        );
        assert_eq!(function.cleanup_plans().count(), 4);
        let read = &instructions[if clone { 2 } else { 4 }];
        assert_eq!(
            read.place_operands()
                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            vec![if clone { 1 } else { 3 }]
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            if clone { vec![1] } else { vec![] }
        );
        let returned = instructions
            .last()
            .expect("fixed fixture structure")
            .result()
            .expect("fixed fixture structure");
        assert_eq!(block.terminator().value_operands().collect::<Vec<_>>(), vec![returned]);
        assert_eq!(block.terminator().derived_drop_actions().count(), usize::from(clone));
        let child = instructions[instructions.len() - 2].result().expect("fixed fixture structure");
        assert_eq!(
            instructions
                .last()
                .expect("fixed fixture structure")
                .value_operands()
                .collect::<Vec<_>>(),
            vec![child]
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index),
                    i.value_operands()
                        .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

#[test]
fn mixed_local_vec_moves_into_outer_vec_with_no_residual_drop() {
    assert_local_program(false);
}

#[test]
fn mixed_local_string_clone_retains_original_cleanup() {
    assert_local_program(true);
}
