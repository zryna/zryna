use super::*;
use mixed_phase_fixtures::{PhaseChild, phase_fixture};
use zryna_syntax::v4::{
    RawExpressionKind, RawFieldInitializerKind, RawTypeSyntax, RawTypeSyntaxKind,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum ProjectedRead {
    Direct,
    Clone,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum InvalidRead {
    Missing,
    WrongType,
    MovedRoot,
    MovedLeaf,
}

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn reference(start: usize, name: &str) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + name.len()),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: name.into(), span: at(start, start + name.len()) },
        },
    }
}

fn field(start: usize, base: u32) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + 7),
        kind: RawExpressionKind::FieldAccess {
            base,
            dot_span: at(start + 1, start + 2),
            field: RawIdentifierSyntax { text: "first".into(), span: at(start + 2, start + 7) },
        },
    }
}

// Fixed lexical surgery over the existing real OwnedPair local/Vec fixture.
pub(in crate::data_ownership_v1) fn projected_fixture(
    mode: ProjectedRead,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = phase_fixture(PhaseChild::StringClone, false);
    let old = "clone(p.first)";
    let start = source.find(old).expect("original projected clone");
    let replacement = match mode {
        ProjectedRead::Direct => "concat(p.first, \"b\")",
        ProjectedRead::Clone => "concat(clone(p.first), \"b\")",
    };
    source.replace_range(start..start + old.len(), replacement);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + old.len()).expect("offset"),
        u32::try_from(replacement.len() - old.len()).expect("growth"),
    );
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 9);
    let mut expressions = body.expressions[..3].to_vec();
    let p = start + if matches!(mode, ProjectedRead::Clone) { 13 } else { 7 };
    expressions.push(reference(p, "p"));
    expressions.push(field(p, 3));
    let left = if matches!(mode, ProjectedRead::Clone) {
        expressions.push(RawExpressionSyntax {
            span: at(start + 7, p + 8),
            kind: RawExpressionKind::Clone {
                keyword_span: at(start + 7, start + 12),
                open_paren_span: at(start + 12, start + 13),
                value: 4,
                close_paren_span: at(p + 7, p + 8),
            },
        });
        5
    } else {
        4
    };
    let right = u32::try_from(expressions.len()).expect("bounded expression");
    let literal = start + replacement.find("\"b\"").expect("right literal");
    expressions.push(RawExpressionSyntax {
        span: at(literal, literal + 3),
        kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".into() },
    });
    let concat = u32::try_from(expressions.len()).expect("bounded expression");
    expressions.push(RawExpressionSyntax {
        span: at(start, start + replacement.len()),
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "concat".into(), span: at(start, start + 6) },
            open_paren_span: at(start + 6, start + 7),
            arguments: vec![left, right],
            close_paren_span: at(start + replacement.len() - 1, start + replacement.len()),
        },
    });
    expressions.push(body.expressions[6].clone());
    let mut structure = body.expressions[7].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("result OwnedPair");
    };
    for (field, value) in fields.iter_mut().zip([concat, concat + 1]) {
        let RawFieldInitializerKind::Explicit { value: slot, .. } = &mut field.kind else {
            panic!("explicit fields");
        };
        *slot = value;
    }
    expressions.push(structure);
    let mut vector = body.expressions[8].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("outer Vec");
    };
    *elements = vec![concat + 2];
    expressions.push(vector);
    body.expressions = expressions;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("final return");
    };
    *value = concat + 3;
    (source, raw)
}

fn offset_inputs(expression: &mut RawExpressionSyntax, amount: u32) {
    let bump = |id: &mut u32| {
        if *id >= 3 {
            *id += amount;
        }
    };
    match &mut expression.kind {
        RawExpressionKind::FieldAccess { base, .. } => bump(base),
        RawExpressionKind::Clone { value, .. } => bump(value),
        RawExpressionKind::Call { arguments, .. } => arguments.iter_mut().for_each(bump),
        RawExpressionKind::VecConstruction { elements, type_syntax, .. } => {
            elements.iter_mut().for_each(bump);
            assert_eq!(*type_syntax, 6);
            *type_syntax = 7;
        }
        RawExpressionKind::StructConstruction { fields, .. } => {
            for field in fields {
                let RawFieldInitializerKind::Explicit { value, .. } = &mut field.kind else {
                    panic!("explicit field");
                };
                bump(value);
            }
        }
        RawExpressionKind::Reference { .. }
        | RawExpressionKind::BoolLiteral { .. }
        | RawExpressionKind::StringLiteral { .. } => {}
        _ => panic!("fixed source vocabulary"),
    }
}

fn insert_move(source: &mut String, raw: &mut RawProjectSyntaxSnapshot, leaf: bool) {
    let start = source.find("return ").expect("return");
    let prefix =
        if leaf { "const taken: String = p.first; " } else { "const moved: OwnedPair = p; " };
    source.insert_str(start, prefix);
    *raw = shift_snapshot(
        raw.clone(),
        u32::try_from(start).expect("offset"),
        u32::try_from(prefix.len()).expect("growth"),
    );
    let spelling = if leaf { "String" } else { "OwnedPair" };
    let ty = start + prefix.find(spelling).expect("type");
    raw.files[0].type_syntax.insert(
        5,
        RawTypeSyntax {
            span: at(ty, ty + spelling.len()),
            kind: if leaf {
                RawTypeSyntaxKind::String { keyword_span: at(ty, ty + 6) }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax {
                        text: spelling.into(),
                        span: at(ty, ty + spelling.len()),
                    },
                }
            },
        },
    );
    // The former Vec type is now index7 and refers to shifted named type6.
    let RawTypeSyntaxKind::Vec { argument, .. } = &mut raw.files[0].type_syntax[7].kind else {
        panic!("final Vec syntax");
    };
    *argument = 6;
    let body = &mut raw.files[0].functions[0].body;
    let count = if leaf { 2 } else { 1 };
    for expression in &mut body.expressions {
        offset_inputs(expression, count);
    }
    let p = start + prefix.find("= p").expect("initializer") + 2;
    let mut inserted = vec![reference(p, "p")];
    if leaf {
        inserted.push(field(p, 3));
    }
    body.expressions.splice(3..3, inserted);
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("return");
    };
    *value += count;
    let name = if leaf { "taken" } else { "moved" };
    let semi = start + prefix.len() - 2;
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: at(start, semi + 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(start, start + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: name.into(), span: at(start + 6, start + 11) },
                type_syntax: 5,
                equals_span: at(p - 2, p - 1),
                initializer: 2 + count,
                semicolon_span: at(semi, semi + 1),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
}

pub(in crate::data_ownership_v1) fn invalid_fixture(
    mode: InvalidRead,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = projected_fixture(ProjectedRead::Direct);
    match mode {
        InvalidRead::MovedRoot | InvalidRead::MovedLeaf => {
            insert_move(&mut source, &mut raw, matches!(mode, InvalidRead::MovedLeaf));
        }
        InvalidRead::Missing => {
            let expression = &mut raw.files[0].functions[0].body.expressions[3];
            let RawExpressionKind::Reference { name } = &mut expression.kind else {
                panic!("projected base");
            };
            source.replace_range(name.span.start as usize..name.span.end as usize, "q");
            name.text = "q".into();
        }
        InvalidRead::WrongType => {
            let expression = &mut raw.files[0].functions[0].body.expressions[4];
            let RawExpressionKind::FieldAccess { field, .. } = &mut expression.kind else {
                panic!("projected field");
            };
            source.replace_range(field.span.start as usize..field.span.end as usize, "flag ");
            field.text = "flag".into();
            field.span.end -= 1;
            expression.span.end -= 1;
        }
    }
    (source, raw)
}

fn expected_kinds(cloned: bool) -> Vec<VerifiedInstructionKind> {
    let mut kinds = vec![
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::BoolLiteral,
        VerifiedInstructionKind::StructConstruct,
        VerifiedInstructionKind::InitializePlace,
    ];
    if cloned {
        kinds.push(VerifiedInstructionKind::StringClone);
    }
    kinds.extend([
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::StringConcat,
        VerifiedInstructionKind::BoolLiteral,
        VerifiedInstructionKind::StructConstruct,
        VerifiedInstructionKind::VecConstruct,
    ]);
    kinds
}

fn expected_cleanup(cloned: bool) -> Vec<Vec<u32>> {
    if cloned {
        vec![
            vec![],
            vec![],
            vec![],
            vec![],
            vec![2],
            vec![4, 2],
            vec![5, 4, 2],
            vec![],
            vec![],
            vec![7, 5, 4, 2],
        ]
    } else {
        vec![vec![], vec![], vec![], vec![], vec![2], vec![4, 2], vec![], vec![], vec![6, 4, 2]]
    }
}

fn assert_projected_ir(mode: ProjectedRead) {
    let (source, raw) = projected_fixture(mode);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated projected concat");
    let cloned = matches!(mode, ProjectedRead::Clone);
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("independent full IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            expected_kinds(cloned)
        );
        assert_eq!(function.places().count(), if cloned { 9 } else { 8 });
        assert_eq!(function.cleanup_plans().count(), if cloned { 6 } else { 5 });
        let cleanup = instructions
            .iter()
            .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(cleanup, expected_cleanup(cloned));
        let concat = if cloned { 6 } else { 5 };
        assert_eq!(
            instructions[concat]
                .place_operands()
                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            if cloned { vec![4, 5] } else { vec![3, 4] }
        );
        if cloned {
            assert_eq!(
                instructions[4]
                    .place_operands()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
                vec![3]
            );
        }
        assert_eq!(
            block.terminator().derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            if cloned { vec![5, 4, 2] } else { vec![4, 2] }
        );
        let structure = instructions.len() - 2;
        assert_eq!(
            instructions[structure].value_operands().collect::<Vec<_>>(),
            vec![
                instructions[concat].result().expect("String"),
                instructions[concat + 1].result().expect("Bool")
            ]
        );
        assert_eq!(
            instructions[structure + 1].value_operands().collect::<Vec<_>>(),
            vec![instructions[structure].result().expect("Pair")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[structure + 1].result().expect("Vec")]
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
fn mixed_unknown_projected_concat_preserves_original_aggregate_and_read_cleanup() {
    assert_projected_ir(ProjectedRead::Direct);
}

#[test]
fn mixed_unknown_projected_clone_concat_preserves_intermediate_owner_cleanup() {
    assert_projected_ir(ProjectedRead::Clone);
}
