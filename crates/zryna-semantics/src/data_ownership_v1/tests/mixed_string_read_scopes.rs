use super::*;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum ReadCase {
    LiteralClone,
    LocalConcat,
    NestedConcat,
}

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn literal(start: usize, spelling: &str) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + spelling.len()),
        kind: RawExpressionKind::StringLiteral { spelling: spelling.into() },
    }
}

fn reference(start: usize) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + 4),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "text".into(), span: at(start, start + 4) },
        },
    }
}

fn clone_expression(start: usize, close: usize, value: u32) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, close + 1),
        kind: RawExpressionKind::Clone {
            keyword_span: at(start, start + 5),
            open_paren_span: at(start + 5, start + 6),
            value,
            close_paren_span: at(close, close + 1),
        },
    }
}

fn concat(start: usize, close: usize, arguments: Vec<u32>) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, close + 1),
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "concat".into(), span: at(start, start + 6) },
            open_paren_span: at(start + 6, start + 7),
            arguments,
            close_paren_span: at(close, close + 1),
        },
    }
}

// Fixed lexical edits to the existing initialized-local/Parcel fixture. The DTO
// still passes source authentication before any layout or lowering assertion.
pub(in crate::data_ownership_v1) fn read_fixture(
    case: ReadCase,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_local_construction::string_clone_fixture();
    let start = source.find("clone(text)").expect("fixed original read");
    let replacement = match case {
        ReadCase::LiteralClone => "clone(\"a\" )",
        ReadCase::LocalConcat => "concat(text, \"b\")",
        ReadCase::NestedConcat => "concat(clone(text), \"b\")",
    };
    source.replace_range(start..start + 11, replacement);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + 11).expect("fixture offset"),
        u32::try_from(replacement.len() - 11).expect("nonnegative fixed edit"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let mut expressions = vec![body.expressions[0].clone()];
    match case {
        ReadCase::LiteralClone => {
            expressions.push(literal(start + 6, "\"a\""));
            expressions.push(clone_expression(start, start + 10, 1));
        }
        ReadCase::LocalConcat => {
            expressions.push(reference(start + 7));
            expressions.push(literal(start + 13, "\"b\""));
            expressions.push(concat(start, start + replacement.len() - 1, vec![1, 2]));
        }
        ReadCase::NestedConcat => {
            expressions.push(reference(start + 13));
            expressions.push(clone_expression(start + 7, start + 17, 1));
            expressions.push(literal(start + 20, "\"b\""));
            expressions.push(concat(start, start + replacement.len() - 1, vec![2, 3]));
        }
    }
    let string = u32::try_from(expressions.len() - 1).expect("fixture expression index");
    let mut structure = body.expressions[3].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("fixed Parcel constructor");
    };
    let RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind else {
        panic!("fixed text field");
    };
    *value = string;
    expressions.push(structure);
    let mut vector = body.expressions[4].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("fixed outer Vec constructor");
    };
    *elements = vec![string + 1];
    expressions.push(vector);
    body.expressions = expressions;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("fixed final return");
    };
    *value = string + 2;
    (source, raw)
}

fn expected_kinds(case: ReadCase) -> Vec<VerifiedInstructionKind> {
    if matches!(case, ReadCase::NestedConcat) {
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringClone,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringConcat,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    } else {
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            if matches!(case, ReadCase::LiteralClone) {
                VerifiedInstructionKind::StringClone
            } else {
                VerifiedInstructionKind::StringConcat
            },
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    }
}

fn expected_cleanup(nested: bool) -> Vec<Vec<u32>> {
    if nested {
        vec![vec![], vec![], vec![1], vec![2, 1], vec![3, 2, 1], vec![], vec![5, 3, 2, 1]]
    } else {
        vec![vec![], vec![], vec![1], vec![2, 1], vec![], vec![4, 2, 1]]
    }
}

fn assert_read_places(case: ReadCase, places: &[Vec<u32>]) {
    if matches!(case, ReadCase::NestedConcat) {
        assert_eq!(places[2], vec![1], "clone reads the original local");
        assert_eq!(places[4], vec![2, 3], "concat reads retained temporaries in order");
    } else {
        assert_eq!(
            places[3],
            if matches!(case, ReadCase::LiteralClone) { vec![2] } else { vec![1, 2] }
        );
    }
}

fn assert_read_program(case: ReadCase) {
    let (source, snapshot) = read_fixture(case);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated String read source");
    let input = pair_input(&syntax, &sources);
    let nested = matches!(case, ReadCase::NestedConcat);
    let retained = if nested { vec![3, 2, 1] } else { vec![2, 1] };
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(input).expect("real source reaches independent full IR verification");
        assert_eq!(program.modules().count(), 1);
        let module = program.modules().next().expect("module");
        assert_eq!(module.functions().count(), 1);
        let function = module.functions().next().expect("function");
        assert_eq!(function.blocks().count(), 1);
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(), expected_kinds(case));
        assert_eq!(function.places().count(), if nested { 7 } else { 6 });
        assert!(function.places().all(|place| !place.is_copy()));
        assert_eq!(function.cleanup_plans().count(), if nested { 6 } else { 5 });
        let cleanup = instructions
            .iter()
            .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(cleanup, expected_cleanup(nested));
        let places = instructions
            .iter()
            .map(|i| {
                i.place_operands()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_read_places(case, &places);
        let structure = instructions.len() - 2;
        let string = instructions[structure - 1].result().expect("final String result");
        assert_eq!(instructions[structure].value_operands().collect::<Vec<_>>(), vec![string]);
        let parcel = instructions[structure].result().expect("Parcel result");
        assert_eq!(instructions[structure + 1].value_operands().collect::<Vec<_>>(), vec![parcel]);
        let vector = instructions[structure + 1].result().expect("Vec result");
        assert_eq!(block.terminator().value_operands().collect::<Vec<_>>(), vec![vector]);
        assert_eq!(
            block.terminator().derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            retained
        );
        // Exact instruction kinds above exclude MoveFromPlace and eager DropPlace.
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
fn mixed_string_literal_clone_retains_read_temporary_through_return() {
    assert_read_program(ReadCase::LiteralClone);
}

#[test]
fn mixed_string_concat_retains_local_and_literal_read_owners() {
    assert_read_program(ReadCase::LocalConcat);
}

#[test]
fn mixed_string_nested_concat_retains_intermediate_read_owners_in_order() {
    assert_read_program(ReadCase::NestedConcat);
}
