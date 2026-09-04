use super::*;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedCallArgument, VerifiedInstruction};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn call(start: usize, name: &str, end: usize, arguments: Vec<u32>) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, end),
        kind: RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: name.into(), span: at(start, start + name.len()) },
            open_paren_span: at(start + name.len(), start + name.len() + 1),
            arguments,
            close_paren_span: at(end - 1, end),
        },
    }
}

// Reuse the actual sealed identity/producer declarations, relocating syntax only.
fn append_string_callees(source: &mut String, raw: &mut RawProjectSyntaxSnapshot) {
    let (callee_source, callees) = private_string_call_fixture();
    assert_eq!(callees.files[0].functions.len(), 3);
    assert_eq!(callees.files[0].type_syntax.len(), 6);
    let old_start = callees.files[0].functions[1].span.start;
    source.push(' ');
    let start = u32::try_from(source.len()).expect("fixture offset");
    let shifted = shift_snapshot(
        callees,
        old_start,
        start.checked_sub(old_start).expect("mixed caller is longer than original caller"),
    );
    source.push_str(&callee_source[usize::try_from(old_start).expect("offset")..]);
    let base = u32::try_from(raw.files[0].type_syntax.len()).expect("type count");
    let types = &shifted.files[0].type_syntax[3..];
    assert!(types.iter().all(|ty| matches!(ty.kind, RawTypeSyntaxKind::String { .. })));
    raw.files[0].type_syntax.extend_from_slice(types);
    for (ordinal, original) in shifted.files[0].functions[1..].iter().enumerate() {
        let mut function = original.clone();
        assert_eq!(function.name.text, if ordinal == 0 { "identity" } else { "producer" });
        assert_eq!(function.parameters.len(), usize::from(ordinal == 0));
        function.result_type = base + function.result_type.checked_sub(3).expect("callee type");
        for parameter in &mut function.parameters {
            parameter.type_syntax =
                base + parameter.type_syntax.checked_sub(3).expect("parameter type");
        }
        raw.files[0].functions.push(function);
    }
}

pub(in crate::data_ownership_v1) fn mixed_string_call_fixture() -> (String, RawProjectSyntaxSnapshot)
{
    let (mut source, mut raw) = mixed_local_construction::string_clone_fixture();
    let old = "clone(text)";
    let replacement = "identity(producer())";
    let start = source.find(old).expect("mixed String child");
    source.replace_range(start..start + old.len(), replacement);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + old.len()).expect("offset"),
        u32::try_from(replacement.len() - old.len()).expect("growth"),
    );
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 5);
    assert_eq!(body.statements.len(), 2);
    assert!(matches!(body.expressions[0].kind, RawExpressionKind::StringLiteral { .. }));
    assert!(matches!(body.expressions[1].kind, RawExpressionKind::Reference { .. }));
    assert!(matches!(body.expressions[2].kind, RawExpressionKind::Clone { .. }));
    // Keep all arena IDs for the outer constructors: replace ref+clone with producer+identity.
    body.expressions[1] = call(start + 9, "producer", start + replacement.len() - 1, Vec::new());
    body.expressions[2] = call(start, "identity", start + replacement.len(), vec![1]);
    let RawExpressionKind::StructConstruction { fields, .. } = &body.expressions[3].kind else {
        panic!("Parcel constructor");
    };
    assert_eq!(fields.len(), 1);
    assert!(matches!(fields[0].kind, RawFieldInitializerKind::Explicit { value: 2, .. }));
    let RawExpressionKind::VecConstruction { elements, .. } = &body.expressions[4].kind else {
        panic!("outer Vec");
    };
    assert_eq!(elements, &[3]);
    append_string_callees(&mut source, &mut raw);
    (source, raw)
}

#[test]
fn mixed_string_producer_identity_child_binds_callee_transfer_and_cleanup() {
    let (source, raw) = mixed_string_call_fixture();
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("authenticated mixed caller and real callees");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("independent full mixed-call IR");
        let module = program.modules().next().expect("module");
        let functions = module.functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("caller block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(caller.places().count(), 6);
        assert_eq!(caller.cleanup_plans().count(), 5);
        assert_eq!(instructions[2].callee().expect("producer").declaration(), 2);
        assert_eq!(instructions[3].callee().expect("identity").declaration(), 1);
        assert_eq!(instructions[2].call_arguments().count(), 0);
        assert_eq!(
            instructions[3].call_arguments().collect::<Vec<_>>(),
            vec![VerifiedCallArgument::Value(instructions[2].result().expect("producer String"))]
        );
        for instruction in &instructions[2..4] {
            let cleanup = caller
                .cleanup_plans()
                .find(|plan| Some(plan.id()) == instruction.cleanup())
                .expect("call cleanup");
            assert_eq!(cleanup.site().role(), VerifiedCleanupRole::CallTrap);
        }
        assert_eq!(
            instructions
                .iter()
                .map(|instruction| instruction
                    .derived_drop_actions()
                    .map(|action| action.root().index())
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![], vec![], vec![1], vec![1], vec![], vec![4, 1]],
            "identity argument owner is transferred before CallTrap; original text survives"
        );
        assert_eq!(
            instructions[4].value_operands().collect::<Vec<_>>(),
            vec![instructions[3].result().expect("identity String")]
        );
        assert_eq!(
            instructions[5].value_operands().collect::<Vec<_>>(),
            vec![instructions[4].result().expect("Parcel")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[5].result().expect("Vec")]
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_callees(&functions);
        let observed = instructions
            .iter()
            .map(|instruction| {
                (
                    instruction.kind(),
                    instruction.result().map(ValueIdentity::index),
                    instruction
                        .callee()
                        .map(zryna_ir::data_ownership_v1::FunctionIdentity::declaration),
                    instruction.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

fn assert_callees(functions: &[VerifiedFunction<'_>]) {
    let identity = &functions[1];
    let producer = &functions[2];
    assert_eq!(
        identity.places().filter(|place| place.kind() == VerifiedPlaceKind::Parameter(0)).count(),
        1
    );
    let block = identity.blocks().next().expect("identity block");
    assert_eq!(
        block.instructions().map(VerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::MoveFromPlace]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    let block = producer.blocks().next().expect("producer block");
    assert_eq!(
        block.instructions().map(VerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::StringFromUtf8]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}
