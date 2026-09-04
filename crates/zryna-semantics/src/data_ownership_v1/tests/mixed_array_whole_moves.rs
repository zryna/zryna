use super::super::{at, snapshot, string_type, vector_type};
use super::{construction, nested_type};
use crate::data_ownership_v1::tests::*;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity};
use zryna_syntax::v4::RawExpressionKind;

fn fixture(duplicate: bool) -> (String, RawProjectSyntaxSnapshot) {
    let array = "FixedArray<Vec<String>, 1>";
    let vector = "Vec<FixedArray<Vec<String>, 1>>";
    let elements = if duplicate { "item, item" } else { "item" };
    let source = format!(
        "function make(): {vector} {{ const item: {array} = {array}([Vec<String>([\"a\"])]); return {vector}([{elements}]); }}"
    );
    let result_start = source.find(vector).expect("result");
    let local_start = source.find("const item").expect("local");
    let local_type = local_start + "const item: ".len();
    let array_start = source.find(" = ").expect("initializer") + 3;
    let inner_start = source.find("Vec<String>([").expect("inner Vec");
    let literal = source.find("\"a\"").expect("literal");
    let returned = source.rfind(vector).expect("returned Vec");
    let reference = returned + vector.len() + 2;
    let mut types = Vec::new();
    let child = nested_type(&mut types, result_start + 4, true);
    let result = vector_type(&mut types, result_start, result_start + vector.len(), child);
    let local_ty = nested_type(&mut types, local_type, true);
    let array_ty = nested_type(&mut types, array_start, true);
    let string = string_type(&mut types, inner_start + 4);
    let inner_ty = vector_type(&mut types, inner_start, inner_start + 11, string);
    let child = nested_type(&mut types, returned + 4, true);
    let returned_ty = vector_type(&mut types, returned, returned + vector.len(), child);
    let reference_expression = |start| RawExpressionSyntax {
        span: at(start, start + 4),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "item".into(), span: at(start, start + 4) },
        },
    };
    let mut expressions = vec![
        RawExpressionSyntax {
            span: at(literal, literal + 3),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        construction(inner_start, literal + 5, inner_start + 11, inner_ty, 0, false),
        construction(array_start, literal + 7, array_start + array.len(), array_ty, 1, true),
        reference_expression(reference),
    ];
    let mut outer = construction(
        returned,
        reference + elements.len() + 2,
        returned + vector.len(),
        returned_ty,
        3,
        false,
    );
    if duplicate {
        expressions.push(reference_expression(reference + 6));
        let RawExpressionKind::VecConstruction { elements, .. } = &mut outer.kind else {
            panic!("Vec")
        };
        elements.push(4);
    }
    expressions.push(outer);
    let mut raw = snapshot(&source, types, vec![], expressions, result);
    let semi = source.find(';').expect("local terminator");
    let body = &mut raw.files[0].functions[0].body;
    body.statements.insert(
        0,
        RawStatementSyntax {
            span: at(local_start, semi + 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(local_start, local_start + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "item".into(),
                    span: at(local_start + 6, local_start + 10),
                },
                type_syntax: local_ty,
                equals_span: at(array_start - 2, array_start - 1),
                initializer: 2,
                semicolon_span: at(semi, semi + 1),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1];
    (source, raw)
}

#[test]
fn mixed_owned_array_local_moves_once_into_outer_vec_with_exact_cleanup() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated actual array local");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("full owned array move IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::FixedArrayConstruct,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(function.places().count(), 6);
        assert_eq!(function.cleanup_plans().count(), 4);
        assert_eq!(
            instructions[4].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            instructions[5].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            vec![3]
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![
                (Some(0), vec![], vec![]),
                (Some(1), vec![0], vec![0]),
                (Some(2), vec![1], vec![]),
                (None, vec![2], vec![]),
                (Some(3), vec![], vec![]),
                (Some(4), vec![3], vec![4]),
            ]
        );
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

#[test]
fn mixed_owned_array_second_whole_move_reports_exact_source_unavailable() {
    let (source, raw) = fixture(true);
    let bad = raw.files[0].functions[0].body.expressions[4].span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated duplicate array reference");
    let expected = vec![Diagnostic::error_at(
        "ZRYNA-M3014",
        span(&sources, bad),
        "aggregate value 'item' is moved or only partially available",
        "move a whole owned aggregate only before moving any of its projections",
    )];
    for _ in 0..2 {
        assert_eq!(
            lower(pair_input(&syntax, &sources)).expect_err("one unique source owner"),
            expected
        );
    }
}
