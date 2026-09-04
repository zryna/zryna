#[path = "mixed_array_whole_moves.rs"]
mod whole_moves;

use super::{at, snapshot, string_type, vector_type};
use crate::data_ownership_v1::tests::*;
use zryna_ir::data_ownership_v1::VerifiedPlaceKind;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;

fn array_type(types: &mut Vec<RawTypeSyntax>, start: usize, end: usize, element: u32) -> u32 {
    let id = u32::try_from(types.len()).expect("type ID");
    types.push(RawTypeSyntax {
        span: at(start, end + 4),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: at(start, start + 10),
            less_than_span: at(start + 10, start + 11),
            element,
            comma_span: at(end, end + 1),
            length_span: at(end + 2, end + 3),
            length: 1,
            length_spelling: "1".into(),
            greater_than_span: at(end + 3, end + 4),
        },
    });
    id
}

fn nested_type(types: &mut Vec<RawTypeSyntax>, start: usize, array_outer: bool) -> u32 {
    let string = string_type(types, start + 15);
    if array_outer {
        let inner = vector_type(types, start + 11, start + 22, string);
        array_type(types, start, start + 22, inner)
    } else {
        let inner = array_type(types, start + 4, start + 21, string);
        vector_type(types, start, start + 26, inner)
    }
}

fn construction(
    start: usize,
    end: usize,
    type_end: usize,
    ty: u32,
    child: u32,
    array: bool,
) -> RawExpressionSyntax {
    let open_paren_span = at(type_end, type_end + 1);
    let open_bracket_span = at(type_end + 1, type_end + 2);
    let close_bracket_span = at(end - 2, end - 1);
    let close_paren_span = at(end - 1, end);
    let elements = vec![child];
    let kind = if array {
        RawExpressionKind::FixedArrayConstruction {
            type_syntax: ty,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        }
    } else {
        RawExpressionKind::VecConstruction {
            type_syntax: ty,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        }
    };
    RawExpressionSyntax { span: at(start, end), kind }
}

// Fixed three-expression source fixtures; no parsing, type checking or layout model.
fn fixture(array_outer: bool) -> (String, RawProjectSyntaxSnapshot) {
    let outer =
        if array_outer { "FixedArray<Vec<String>, 1>" } else { "Vec<FixedArray<String, 1>>" };
    let inner = if array_outer { "Vec<String>" } else { "FixedArray<String, 1>" };
    let source = format!("function make(): {outer} {{ return {outer}([{inner}([\"a\"])]); }}");
    let result_start = source.find(outer).expect("result type");
    let outer_start = source.rfind(outer).expect("outer construction");
    let inner_start = source.rfind(inner).expect("inner construction");
    let literal_start = source.find("\"a\"").expect("literal");
    let mut types = Vec::new();
    let result = nested_type(&mut types, result_start, array_outer);
    let outer_ty = nested_type(&mut types, outer_start, array_outer);
    let leaf = string_type(&mut types, inner_start + if array_outer { 4 } else { 11 });
    let inner_ty = if array_outer {
        vector_type(&mut types, inner_start, inner_start + inner.len(), leaf)
    } else {
        array_type(&mut types, inner_start, inner_start + 17, leaf)
    };
    let inner_end = literal_start + 5;
    let expressions = vec![
        RawExpressionSyntax {
            span: at(literal_start, literal_start + 3),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        construction(inner_start, inner_end, inner_start + inner.len(), inner_ty, 0, !array_outer),
        construction(
            outer_start,
            inner_end + 2,
            outer_start + outer.len(),
            outer_ty,
            1,
            array_outer,
        ),
    ];
    let raw = snapshot(&source, types, vec![], expressions, result);
    (source, raw)
}

#[test]
fn mixed_positive_array_vec_both_directions_bind_exact_types_and_owner_transfers() {
    for array_outer in [true, false] {
        let (source, raw) = fixture(array_outer);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("authenticated nonempty mixed array source");
        let mut previous = None;
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources)).expect("independent full mixed IR");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let inner_kind = if array_outer {
                VerifiedInstructionKind::VecConstruct
            } else {
                VerifiedInstructionKind::FixedArrayConstruct
            };
            let outer_kind = if array_outer {
                VerifiedInstructionKind::FixedArrayConstruct
            } else {
                VerifiedInstructionKind::VecConstruct
            };
            assert_eq!(
                instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
                vec![VerifiedInstructionKind::StringFromUtf8, inner_kind, outer_kind]
            );
            let values =
                instructions.iter().map(|i| i.result().expect("result")).collect::<Vec<_>>();
            assert_eq!(values.iter().map(|v| v.index()).collect::<Vec<_>>(), vec![0, 1, 2]);
            assert_eq!(instructions[1].value_operands().collect::<Vec<_>>(), vec![values[0]]);
            assert_eq!(instructions[2].value_operands().collect::<Vec<_>>(), vec![values[1]]);
            let mut types = Vec::new();
            for layouts in [
                program.verified_ir().linear32_layouts(),
                program.verified_ir().linux_x86_64_layouts(),
            ] {
                let inner = layouts
                    .type_by_id(instructions[1].result_type().expect("inner type"))
                    .expect("sealed inner");
                let outer = layouts
                    .type_by_id(instructions[2].result_type().expect("outer type"))
                    .expect("sealed outer");
                assert_eq!(
                    inner.category(),
                    if array_outer { TypeCategory::Vec } else { TypeCategory::FixedArray }
                );
                assert_eq!(
                    outer.category(),
                    if array_outer { TypeCategory::FixedArray } else { TypeCategory::Vec }
                );
                assert_eq!(inner.referenced_type(), instructions[0].result_type());
                assert_eq!(outer.referenced_type(), Some(inner.id()));
                let array = if array_outer { outer } else { inner };
                assert_eq!(array.array_length(), Some(1));
                assert!(array.size() > 0, "Vec elements must have positive storage stride");
                types.push((inner.id(), outer.id()));
            }
            assert_eq!(function.places().count(), 3);
            assert_eq!(function.cleanup_plans().count(), 3);
            for (i, place) in function.places().enumerate() {
                assert_eq!(place.kind(), VerifiedPlaceKind::Temporary(values[i]));
                assert!(!place.is_copy());
            }
            let vec_index = if array_outer { 1 } else { 2 };
            assert_eq!(
                instructions[vec_index]
                    .derived_drop_actions()
                    .map(|a| a.root().index())
                    .collect::<Vec<_>>(),
                vec![u32::try_from(vec_index - 1).expect("child root")]
            );
            assert_eq!(block.terminator().value_operands().collect::<Vec<_>>(), vec![values[2]]);
            assert_eq!(block.terminator().derived_drop_actions().count(), 0);
            let observed = (types, values.iter().map(|v| v.index()).collect::<Vec<_>>());
            if let Some(previous) = &previous {
                assert_eq!(&observed, previous);
            }
            previous = Some(observed);
        }
    }
}
