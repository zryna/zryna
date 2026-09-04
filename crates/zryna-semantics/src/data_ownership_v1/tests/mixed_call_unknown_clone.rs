use super::*;
use mixed_string_calls::mixed_string_call_fixture;
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity, VerifiedCallArgument};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

pub(in crate::data_ownership_v1) fn unknown_call_clone_fixture()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_string_call_fixture();
    let old = "identity(producer())";
    let start = source.find(old).expect("original call child");
    source.replace_range(start..start + old.len(), "clone(identity(producer()))");
    raw = shift_snapshot(raw, u32::try_from(start + old.len()).expect("span"), 7);
    let at = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("span"),
        end: u32::try_from(end).expect("span"),
    };
    let make_call = |offset: usize, name: &str, end: usize, arguments| RawExpressionSyntax {
        span: at(offset, end),
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: name.into(),
                span: at(offset, offset + name.len()),
            },
            open_paren_span: at(offset + name.len(), offset + name.len() + 1),
            arguments,
            close_paren_span: at(end - 1, end),
        },
    };
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 5);
    let mut structure = body.expressions[3].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel");
    };
    let RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind else {
        panic!("String field");
    };
    *value = 3;
    let mut vector = body.expressions[4].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("Vec");
    };
    *elements = vec![4];
    body.expressions = vec![
        body.expressions[0].clone(),
        make_call(start + 15, "producer", start + 25, Vec::new()),
        make_call(start + 6, "identity", start + 26, vec![1]),
        RawExpressionSyntax {
            span: at(start, start + 27),
            kind: RawExpressionKind::Clone {
                keyword_span: at(start, start + 5),
                open_paren_span: at(start + 5, start + 6),
                value: 2,
                close_paren_span: at(start + 26, start + 27),
            },
        },
        structure,
        vector,
    ];
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("return");
    };
    *value = 5;
    (source, raw)
}

#[test]
fn mixed_clone_of_identity_call_preserves_unknown_source_owner_and_exact_cleanup() {
    let (source, raw) = unknown_call_clone_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated clone of actual String call");
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).expect("independent Unknown clone full IR");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::StringClone,
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(caller.places().count(), 7);
        assert_eq!(caller.cleanup_plans().count(), 6);
        assert_eq!(instructions[2].callee().expect("producer").declaration(), 2);
        assert_eq!(instructions[2].call_arguments().count(), 0);
        assert_eq!(instructions[3].callee().expect("identity").declaration(), 1);
        assert_eq!(
            instructions[3].call_arguments().collect::<Vec<_>>(),
            vec![VerifiedCallArgument::Value(instructions[2].result().expect("producer String"))]
        );
        assert_eq!(
            instructions[4].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            instructions
                .iter()
                .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![], vec![], vec![1], vec![1], vec![3, 1], vec![], vec![5, 3, 1]]
        );
        assert_eq!(
            instructions[5].value_operands().collect::<Vec<_>>(),
            vec![instructions[4].result().expect("clone result")]
        );
        assert_eq!(
            instructions[6].value_operands().collect::<Vec<_>>(),
            vec![instructions[5].result().expect("Parcel")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[6].result().expect("Vec")]
        );
        assert_eq!(
            block.terminator().derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            vec![3, 1]
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}
