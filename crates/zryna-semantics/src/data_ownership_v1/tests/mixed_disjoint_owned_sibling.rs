use super::*;
use mixed_unknown_projected::{InvalidRead, invalid_fixture};
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity};
use zryna_syntax::v4::{RawExpressionKind, RawTypeSyntaxKind};

// Fixed source/DTO edits; both declared fields are owned String leaves.
pub(in crate::data_ownership_v1) fn sibling_fixture(
    disjoint: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut snapshot) = invalid_fixture(InvalidRead::MovedLeaf);
    let types = &snapshot.files[0].type_syntax;
    let booleans = types
        .iter()
        .enumerate()
        .filter_map(|(i, ty)| {
            matches!(&ty.kind, RawTypeSyntaxKind::Named { name } if name.text == "bool")
                .then_some(i)
        })
        .collect::<Vec<_>>();
    assert_eq!(booleans.len(), 1);
    let index = booleans[0];
    let old = types[index].span;
    let start = usize::try_from(old.start).expect("source offset");
    let end = usize::try_from(old.end).expect("source offset");
    assert_eq!(&source[start..end], "bool");
    source.replace_range(start..end, "String");
    snapshot = shift_snapshot(snapshot, old.end, 2);
    let ty = &mut snapshot.files[0].type_syntax[index];
    assert_eq!(ty.span.start, old.start);
    assert_eq!(ty.span.end, old.end + 2);
    ty.kind = RawTypeSyntaxKind::String { keyword_span: ty.span };
    let body = &mut snapshot.files[0].functions[0].body;
    let mut converted = 0;
    for expression in &mut body.expressions {
        if let RawExpressionKind::BoolLiteral { value } = expression.kind {
            assert!(value);
            let start = usize::try_from(expression.span.start).expect("literal offset");
            let end = usize::try_from(expression.span.end).expect("literal offset");
            assert_eq!(&source[start..end], "true");
            source.replace_range(start..end, "\"bb\"");
            expression.kind = RawExpressionKind::StringLiteral { spelling: "\"bb\"".into() };
            converted += 1;
        }
    }
    assert_eq!(converted, 2);
    if disjoint {
        let expression = &mut body.expressions[4];
        let RawExpressionKind::FieldAccess { field, .. } = &mut expression.kind else {
            panic!("earlier projected local initializer");
        };
        assert_eq!(field.text, "first");
        let start = usize::try_from(field.span.start).expect("field offset");
        let end = usize::try_from(field.span.end).expect("field offset");
        assert_eq!(&source[start..end], "first");
        source.replace_range(start..end, "flag ");
        field.text = "flag".into();
        field.span.end -= 1;
        expression.span.end -= 1;
    }
    assert!(source.contains("concat(p.first, \"b\")"));
    assert!(source.contains(if disjoint {
        "const taken: String = p.flag ;"
    } else {
        "const taken: String = p.first;"
    }));
    (source, snapshot)
}

fn expected_kinds() -> Vec<VerifiedInstructionKind> {
    vec![
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::StructConstruct,
        VerifiedInstructionKind::InitializePlace,
        VerifiedInstructionKind::MoveFromPlace,
        VerifiedInstructionKind::InitializePlace,
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::StringConcat,
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::StructConstruct,
        VerifiedInstructionKind::VecConstruct,
    ]
}

#[test]
fn mixed_disjoint_owned_sibling_read_preserves_partial_cleanup_and_exact_ir() {
    let (source, snapshot) = sibling_fixture(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated two-String Pair");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("independent complete IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(), expected_kinds());
        assert_eq!(function.places().count(), 13);
        assert_eq!(function.cleanup_plans().count(), 7);
        assert_eq!(
            instructions
                .iter()
                .filter_map(|i| i.result())
                .map(ValueIdentity::index)
                .collect::<Vec<_>>(),
            (0..9).collect::<Vec<_>>()
        );
        let places = |index: usize| {
            instructions[index].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>()
        };
        assert_eq!(places(3), vec![3]);
        assert_eq!(places(4), vec![4]);
        assert_eq!(places(5), vec![6]);
        assert_eq!(places(7), vec![7, 8]);
        let values = |index: usize| {
            instructions[index].value_operands().map(ValueIdentity::index).collect::<Vec<_>>()
        };
        assert_eq!(values(2), vec![0, 1]);
        assert_eq!(values(3), vec![2]);
        assert_eq!(values(5), vec![3]);
        assert_eq!(values(9), vec![5, 6]);
        assert_eq!(values(10), vec![7]);
        assert_eq!(
            instructions
                .iter()
                .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![
                vec![],
                vec![0],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![6, 3],
                vec![8, 6, 3],
                vec![9, 8, 6, 3],
                vec![],
                vec![11, 8, 6, 3],
            ]
        );
        let drops = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(drops.iter().map(|a| a.root().index()).collect::<Vec<_>>(), vec![8, 6, 3]);
        assert_eq!(
            drops[2].moved_projections().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(
            drops[2].initialized_projections().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            vec![8]
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    i.place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
                    i.derived_drop_actions()
                        .map(|a| {
                            (
                                a.root().index(),
                                a.moved_projections().map(PlaceIdentity::index).collect::<Vec<_>>(),
                            )
                        })
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
