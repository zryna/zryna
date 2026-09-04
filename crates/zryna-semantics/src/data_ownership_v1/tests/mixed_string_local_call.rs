use super::mixed_string_calls::mixed_string_call_fixture;
use super::*;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedCallArgument};
use zryna_syntax::v4::RawExpressionKind;

fn call(at: zryna_source::UntrustedSpan) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at,
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "producer".into(),
                span: zryna_source::UntrustedSpan { end: at.start + 8, ..at },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                start: at.start + 8,
                end: at.start + 9,
                ..at
            },
            arguments: Vec::new(),
            close_paren_span: zryna_source::UntrustedSpan { start: at.start + 9, ..at },
        },
    }
}

#[test]
fn mixed_string_producer_local_moves_through_identity_into_constructor() {
    let (mut source, mut snapshot) = mixed_string_call_fixture();
    let original = snapshot.files[0].functions[0].body.expressions[0].span;
    assert_eq!(&source[original.start as usize..original.end as usize], "\"a\"");
    source.replace_range(original.start as usize..original.end as usize, "producer()");
    snapshot = shift_snapshot(snapshot, original.end, 7);
    snapshot.files[0].functions[0].body.expressions[0] =
        call(zryna_source::UntrustedSpan { end: original.start + 10, ..original });
    let inner = snapshot.files[0].functions[0].body.expressions[1].span;
    assert_eq!(&source[inner.start as usize..inner.end as usize], "producer()");
    source.replace_range(inner.start as usize..inner.end as usize, "text");
    snapshot = shift_snapshot_signed(snapshot, inner.end, -6);
    let at = zryna_source::UntrustedSpan { end: inner.start + 4, ..inner };
    snapshot.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: at,
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "text".into(), span: at },
        },
    };
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated String producer local");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("String local call full IR");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|value| value.kind()).collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(caller.places().count(), 6);
        assert_eq!(caller.cleanup_plans().count(), 4);
        assert_eq!(instructions[0].callee().expect("producer").declaration(), 2);
        assert_eq!(instructions[3].callee().expect("identity").declaration(), 1);
        assert_eq!(instructions[0].call_arguments().count(), 0);
        assert_eq!(
            instructions[3].call_arguments().collect::<Vec<_>>(),
            [VerifiedCallArgument::Value(instructions[2].result().expect("moved local"))]
        );
        assert_eq!(
            instructions[2]
                .place_operands()
                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            [1]
        );
        assert_eq!(
            instructions[4].value_operands().collect::<Vec<_>>(),
            [instructions[3].result().expect("identity result")]
        );
        assert_eq!(
            instructions[5].value_operands().collect::<Vec<_>>(),
            [instructions[4].result().expect("Parcel")]
        );
        assert_eq!(
            instructions
                .iter()
                .map(|value| value
                    .derived_drop_actions()
                    .map(|action| action.root().index())
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [vec![], vec![], vec![], vec![], vec![], vec![4]]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let observed = instructions
            .iter()
            .map(|value| {
                (
                    value.kind(),
                    value.result().map(ValueIdentity::index),
                    value.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}
