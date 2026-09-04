use super::*;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedCallArgument};
use zryna_syntax::v4::RawExpressionKind;

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

fn append_vec_callees(source: &mut String, raw: &mut RawProjectSyntaxSnapshot) {
    let (callee_source, callees) = private_vec_call_fixture("String");
    assert_eq!(callees.files[0].functions.len(), 3);
    assert_eq!(callees.files[0].type_syntax.len(), 16);
    let old_start = callees.files[0].functions[1].span.start;
    source.push(' ');
    let start = i32::try_from(source.len()).expect("fixture offset");
    let shifted = shift_snapshot_signed(
        callees,
        old_start,
        start.checked_sub(i32::try_from(old_start).expect("offset")).expect("bounded relocation"),
    );
    source.push_str(&callee_source[usize::try_from(old_start).expect("offset")..]);
    let base = u32::try_from(raw.files[0].type_syntax.len()).expect("type count");
    let remap = |id: u32| base + id.checked_sub(8).expect("callee syntax type");
    for original in &shifted.files[0].type_syntax[8..] {
        let mut ty = original.clone();
        match &mut ty.kind {
            RawTypeSyntaxKind::String { .. } => {}
            RawTypeSyntaxKind::Vec { argument, .. } => *argument = remap(*argument),
            _ => panic!("exact Vec<String> callee syntax"),
        }
        raw.files[0].type_syntax.push(ty);
    }
    for (ordinal, original) in shifted.files[0].functions[1..].iter().enumerate() {
        let mut function = original.clone();
        assert_eq!(function.name.text, if ordinal == 0 { "identity" } else { "producer" });
        assert_eq!(function.parameters.len(), usize::from(ordinal == 0));
        function.result_type = remap(function.result_type);
        for parameter in &mut function.parameters {
            parameter.type_syntax = remap(parameter.type_syntax);
        }
        for expression in &mut function.body.expressions {
            match &mut expression.kind {
                RawExpressionKind::Reference { .. } => assert_eq!(ordinal, 0),
                RawExpressionKind::VecConstruction { type_syntax, elements, .. } => {
                    assert_eq!(ordinal, 1);
                    assert!(elements.is_empty());
                    *type_syntax = remap(*type_syntax);
                }
                _ => panic!("unchanged identity or empty producer body"),
            }
        }
        raw.files[0].functions.push(function);
    }
}

pub(in crate::data_ownership_v1) fn mixed_vec_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_construction::mixed_fixture(false);
    let old = "Vec<String>([\"a\"])";
    let replacement = "identity(producer())";
    let start = source.find(old).expect("mixed Vec field constructor");
    source.replace_range(start..start + old.len(), replacement);
    raw = shift_snapshot_signed(
        raw,
        u32::try_from(start + old.len()).expect("offset"),
        i32::try_from(replacement.len()).expect("length")
            - i32::try_from(old.len()).expect("length"),
    );
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    // These two removed syntax nodes belonged only to the replaced Vec construction.
    raw.files[0].type_syntax.truncate(3);
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 3);
    body.expressions[0] = call(start + 9, "producer", start + replacement.len() - 1, Vec::new());
    body.expressions[1] = call(start, "identity", start + replacement.len(), vec![0]);
    let RawExpressionKind::StructConstruction { fields, .. } = &body.expressions[2].kind else {
        panic!("Parcel result");
    };
    assert_eq!(fields.len(), 1);
    assert!(matches!(
        fields[0].kind,
        zryna_syntax::v4::RawFieldInitializerKind::Explicit { value: 1, .. }
    ));
    append_vec_callees(&mut source, &mut raw);
    (source, raw)
}

#[test]
fn mixed_vec_string_producer_identity_field_transfers_exact_call_results() {
    let (source, raw) = mixed_vec_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("authenticated Parcel and actual Vec<String> callees");
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).expect("independent mixed Vec-call full IR");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("caller block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::StructConstruct,
            ]
        );
        assert_eq!(caller.places().count(), 3);
        assert_eq!(caller.cleanup_plans().count(), 3);
        assert_eq!(instructions[0].callee().expect("producer").declaration(), 2);
        assert_eq!(instructions[0].call_arguments().count(), 0);
        assert_eq!(instructions[1].callee().expect("identity").declaration(), 1);
        assert_eq!(
            instructions[1].call_arguments().collect::<Vec<_>>(),
            vec![VerifiedCallArgument::Value(instructions[0].result().expect("producer Vec"))]
        );
        for call in &instructions[..2] {
            let result = call.result().expect("Vec result");
            assert_eq!(caller.places().filter(|p| matches!(p.kind(), VerifiedPlaceKind::Temporary(value) if value == result)).count(), 1);
            let cleanup = caller
                .cleanup_plans()
                .find(|p| Some(p.id()) == call.cleanup())
                .expect("call cleanup");
            assert_eq!(cleanup.site().role(), VerifiedCleanupRole::CallTrap);
            assert_eq!(
                call.derived_drop_actions().count(),
                0,
                "transferred argument excluded from CallTrap"
            );
        }
        assert_ne!(instructions[0].cleanup(), instructions[1].cleanup());
        assert_eq!(
            instructions[2].value_operands().collect::<Vec<_>>(),
            vec![instructions[1].result().expect("identity Vec")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[2].result().expect("Parcel")]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        assert_eq!(functions[1].parameters().count(), 1);
        assert_eq!(
            functions[1].places().filter(|p| p.kind() == VerifiedPlaceKind::Parameter(0)).count(),
            1
        );
        let producer = functions[2].blocks().next().expect("producer block");
        let production = producer.instructions().collect::<Vec<_>>();
        assert_eq!(production.len(), 1);
        assert_eq!(production[0].kind(), VerifiedInstructionKind::VecConstruct);
        assert_eq!(
            production[0].value_operands().count(),
            0,
            "actual zero-element Vec<String> producer"
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.callee().map(zryna_ir::data_ownership_v1::FunctionIdentity::declaration),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}
