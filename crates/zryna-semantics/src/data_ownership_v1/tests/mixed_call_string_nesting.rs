use super::*;
use mixed_string_calls::mixed_string_call_fixture;
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity, VerifiedCallArgument};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum CallStringNesting {
    StringArgument,
    StringRead,
}

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

fn literal(start: usize) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + 3),
        kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".into() },
    }
}

// Fixed vocabulary and explicit DTO ordering; no semantic/layout inference.
fn argument_expressions(start: usize, replacement: &str) -> Vec<RawExpressionSyntax> {
    let text = start + replacement.find("text").expect("original local read");
    let clone = start + replacement.find("clone(").expect("clone");
    let concat = start + replacement.find("concat(").expect("concat");
    let right = start + replacement.find("\"b\"").expect("right literal");
    vec![
        RawExpressionSyntax {
            span: at(text, text + 4),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "text".into(), span: at(text, text + 4) },
            },
        },
        RawExpressionSyntax {
            span: at(clone, text + 5),
            kind: RawExpressionKind::Clone {
                keyword_span: at(clone, clone + 5),
                open_paren_span: at(clone + 5, clone + 6),
                value: 1,
                close_paren_span: at(text + 4, text + 5),
            },
        },
        literal(right),
        call(concat, "concat", right + 4, vec![2, 3]),
        call(start, "identity", start + replacement.len(), vec![4]),
    ]
}

fn read_expressions(start: usize, replacement: &str) -> Vec<RawExpressionSyntax> {
    let identity = start + replacement.find("identity(").expect("identity");
    let producer = start + replacement.find("producer(").expect("producer");
    let right = start + replacement.find("\"b\"").expect("right literal");
    vec![
        call(producer, "producer", producer + "producer()".len(), Vec::new()),
        call(identity, "identity", producer + "producer())".len(), vec![1]),
        literal(right),
        call(start, "concat", start + replacement.len(), vec![2, 3]),
    ]
}

pub(in crate::data_ownership_v1) fn nested_call_fixture(
    mode: CallStringNesting,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_string_call_fixture();
    let old = "identity(producer())";
    let replacement = match mode {
        CallStringNesting::StringArgument => "identity(concat(clone(text), \"b\"))",
        CallStringNesting::StringRead => "concat(identity(producer()), \"b\")",
    };
    let start = source.find(old).expect("mixed child call");
    source.replace_range(start..start + old.len(), replacement);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + old.len()).expect("offset"),
        u32::try_from(replacement.len() - old.len()).expect("growth"),
    );
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 5);
    let mut expressions = vec![body.expressions[0].clone()];
    expressions.extend(match mode {
        CallStringNesting::StringArgument => argument_expressions(start, replacement),
        CallStringNesting::StringRead => read_expressions(start, replacement),
    });
    let result = u32::try_from(expressions.len() - 1).expect("result expression");
    let mut structure = body.expressions[3].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel");
    };
    assert_eq!(fields.len(), 1);
    let RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind else {
        panic!("explicit String field");
    };
    *value = result;
    expressions.push(structure);
    let mut vector = body.expressions[4].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("Vec");
    };
    *elements = vec![result + 1];
    expressions.push(vector);
    body.expressions = expressions;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("return");
    };
    *value = result + 2;
    assert_eq!(raw.files[0].functions.len(), 3);
    (source, raw)
}

fn expected_kinds(argument: bool) -> Vec<VerifiedInstructionKind> {
    use VerifiedInstructionKind::{
        DirectCall, InitializePlace, StringClone, StringConcat, StringFromUtf8, StructConstruct,
        VecConstruct,
    };
    if argument {
        vec![
            StringFromUtf8,
            InitializePlace,
            StringClone,
            StringFromUtf8,
            StringConcat,
            DirectCall,
            StructConstruct,
            VecConstruct,
        ]
    } else {
        vec![
            StringFromUtf8,
            InitializePlace,
            DirectCall,
            DirectCall,
            StringFromUtf8,
            StringConcat,
            StructConstruct,
            VecConstruct,
        ]
    }
}

fn expected_cleanup(argument: bool) -> Vec<Vec<u32>> {
    if argument {
        vec![
            vec![],
            vec![],
            vec![1],
            vec![2, 1],
            vec![3, 2, 1],
            vec![3, 2, 1],
            vec![],
            vec![6, 3, 2, 1],
        ]
    } else {
        vec![vec![], vec![], vec![1], vec![1], vec![3, 1], vec![4, 3, 1], vec![], vec![6, 4, 3, 1]]
    }
}

fn assert_call_roles(
    instructions: &[zryna_ir::data_ownership_v1::VerifiedInstruction<'_>],
    argument: bool,
) {
    let identity = if argument { 5 } else { 3 };
    assert_eq!(instructions[identity].callee().expect("identity").declaration(), 1);
    assert_eq!(
        instructions[identity].call_arguments().collect::<Vec<_>>(),
        vec![VerifiedCallArgument::Value(
            instructions[if argument { 4 } else { 2 }].result().expect("exact argument result")
        )]
    );
    if !argument {
        assert_eq!(instructions[2].callee().expect("producer").declaration(), 2);
        assert_eq!(instructions[2].call_arguments().count(), 0);
    }
    let concat = if argument { 4 } else { 5 };
    assert_eq!(
        instructions[concat].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
        if argument { vec![2, 3] } else { vec![3, 4] }
    );
    if argument {
        assert_eq!(
            instructions[2].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![1]
        );
    }
}

fn assert_nested_ir(mode: CallStringNesting) {
    let (source, raw) = nested_call_fixture(mode);
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("authenticated caller and both real callees");
    let argument = matches!(mode, CallStringNesting::StringArgument);
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).expect("independent nested-scope full IR");
        let module = program.modules().next().expect("module");
        let functions = module.functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("caller block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            expected_kinds(argument)
        );
        assert_eq!(caller.places().count(), 8);
        assert_eq!(caller.cleanup_plans().count(), 7);
        assert_call_roles(&instructions, argument);
        for instruction in
            instructions.iter().filter(|i| i.kind() == VerifiedInstructionKind::DirectCall)
        {
            let cleanup = caller
                .cleanup_plans()
                .find(|plan| Some(plan.id()) == instruction.cleanup())
                .expect("CallTrap");
            assert_eq!(cleanup.site().role(), VerifiedCleanupRole::CallTrap);
        }
        assert_eq!(
            instructions
                .iter()
                .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            expected_cleanup(argument)
        );
        assert_eq!(
            instructions[6].value_operands().collect::<Vec<_>>(),
            vec![instructions[5].result().expect("final String only")]
        );
        assert_eq!(
            instructions[7].value_operands().collect::<Vec<_>>(),
            vec![instructions[6].result().expect("Parcel only")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[7].result().expect("Vec only")]
        );
        assert_eq!(
            block.terminator().derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            if argument { vec![3, 2, 1] } else { vec![4, 3, 1] }
        );
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
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

#[test]
fn mixed_identity_compound_string_argument_retains_read_temporaries_not_transferred_result() {
    assert_nested_ir(CallStringNesting::StringArgument);
}

#[test]
fn mixed_concat_reads_nested_identity_call_result_without_forwarding_intermediate_producer() {
    assert_nested_ir(CallStringNesting::StringRead);
}
