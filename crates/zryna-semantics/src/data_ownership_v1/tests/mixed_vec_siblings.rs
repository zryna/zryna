use super::*;
use mixed_vec_calls::mixed_vec_call_fixture;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedCallArgument};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn add_vec_type(raw: &mut RawProjectSyntaxSnapshot, start: usize) -> u32 {
    let element = u32::try_from(raw.files[0].type_syntax.len()).expect("type ID");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(start + 4, start + 10),
        kind: RawTypeSyntaxKind::String { keyword_span: at(start + 4, start + 10) },
    });
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(start, start + 11),
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: at(start, start + 3),
            less_than_span: at(start + 3, start + 4),
            argument: element,
            greater_than_span: at(start + 10, start + 11),
        },
    });
    element + 1
}

fn add_field(source: &mut String, raw: &mut RawProjectSyntaxSnapshot) {
    let insertion = usize::try_from(raw.files[0].data_declarations[0].span.end).expect("span") - 1;
    let text = " other: Vec<String>;";
    source.insert_str(insertion, text);
    *raw = shift_snapshot(
        raw.clone(),
        u32::try_from(insertion).expect("span"),
        u32::try_from(text.len()).expect("length"),
    );
    let type_syntax = add_vec_type(raw, insertion + 8);
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Parcel");
    };
    assert_eq!(fields.len(), 1);
    fields.push(RawDataField {
        span: at(insertion + 1, insertion + text.len()),
        name: RawIdentifierSyntax { text: "other".into(), span: at(insertion + 1, insertion + 6) },
        colon_span: at(insertion + 6, insertion + 7),
        type_syntax,
        semicolon_span: at(insertion + text.len() - 1, insertion + text.len()),
    });
}

fn reference(start: usize) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span: at(start, start + 5),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "items".into(), span: at(start, start + 5) },
        },
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

pub(in crate::data_ownership_v1) fn vec_sibling_fixture(
    duplicate: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_vec_call_fixture();
    add_field(&mut source, &mut raw);
    let local = source.find("return ").expect("caller return");
    let prefix = "const items: Vec<String> = producer(); ";
    source.insert_str(local, prefix);
    raw = shift_snapshot(
        raw,
        u32::try_from(local).expect("span"),
        u32::try_from(prefix.len()).expect("length"),
    );
    let local_type = add_vec_type(&mut raw, local + 13);
    let first = source.find("identity(producer())").expect("first child");
    source.replace_range(first..first + 20, "identity(items)");
    raw = shift_snapshot_signed(raw, u32::try_from(first + 20).expect("span"), -5);
    let first_end = first + 15;
    let suffix = if duplicate { ", other: items" } else { ", other: producer()" };
    source.insert_str(first_end, suffix);
    raw = shift_snapshot(
        raw,
        u32::try_from(first_end).expect("span"),
        u32::try_from(suffix.len()).expect("length"),
    );
    let second = first_end + 9;
    let second_end = first_end + suffix.len();
    let mut structure = raw.files[0].functions[0].body.expressions[2].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel constructor");
    };
    assert_eq!(fields.len(), 1);
    fields[0].span.end = u32::try_from(first_end).expect("span");
    let RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind else {
        panic!("first field");
    };
    *value = 2;
    fields.push(RawFieldInitializer {
        span: at(first_end + 2, second_end),
        kind: RawFieldInitializerKind::Explicit {
            name: RawIdentifierSyntax {
                text: "other".into(),
                span: at(first_end + 2, first_end + 7),
            },
            colon_span: at(first_end + 7, first_end + 8),
            value: 3,
        },
    });
    let producer = local + prefix.find("producer()").expect("actual preceding producer");
    let semi = local + prefix.len() - 2;
    let body = &mut raw.files[0].functions[0].body;
    body.expressions = vec![
        call(producer, "producer", producer + 10, Vec::new()),
        reference(first + 9),
        call(first, "identity", first_end, vec![1]),
        if duplicate {
            reference(second)
        } else {
            call(second, "producer", second_end, Vec::new())
        },
        structure,
    ];
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return");
    };
    *value = 4;
    body.statements.insert(
        0,
        RawStatementSyntax {
            span: at(local, semi + 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(local, local + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "items".into(), span: at(local + 6, local + 11) },
                type_syntax: local_type,
                equals_span: at(producer - 2, producer - 1),
                initializer: 0,
                semicolon_span: at(semi, semi + 1),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1];
    (source, raw)
}

#[test]
fn mixed_vec_call_later_sibling_rejects_actual_preceding_local_reuse() {
    let (source, raw) = vec_sibling_fixture(true);
    let bad = raw.files[0].functions[0].body.expressions[3].span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("authenticated two Vec fields and actual items local");
    let expected = vec![Diagnostic::error_at(
        "ZRYNA-M3014",
        span(&sources, bad),
        "aggregate value 'items' is moved or only partially available",
        "move a whole owned aggregate only before moving any of its projections",
    )];
    for _ in 0..2 {
        assert_eq!(
            lower(pair_input(&syntax, &sources)).expect_err("later sibling reuses moved Vec"),
            expected
        );
    }
}

#[test]
fn mixed_vec_call_independent_second_producer_preserves_first_result_cleanup() {
    let (source, raw) = vec_sibling_fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated independent Vec siblings");
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).expect("independent full sibling-call IR");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 3);
        let caller = &functions[0];
        let block = caller.blocks().next().expect("caller block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::StructConstruct,
            ]
        );
        assert_eq!(caller.places().count(), 6);
        assert_eq!(caller.cleanup_plans().count(), 4);
        assert_eq!(instructions[0].callee().expect("local producer").declaration(), 2);
        assert_eq!(instructions[3].callee().expect("identity").declaration(), 1);
        assert_eq!(instructions[4].callee().expect("second producer").declaration(), 2);
        assert_eq!(instructions[0].call_arguments().count(), 0);
        assert_eq!(instructions[4].call_arguments().count(), 0);
        assert_eq!(
            instructions[3].call_arguments().collect::<Vec<_>>(),
            vec![VerifiedCallArgument::Value(
                instructions[2].result().expect("actual moved items result")
            )]
        );
        assert_eq!(
            instructions[2]
                .place_operands()
                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            instructions
                .iter()
                .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![], vec![], vec![], vec![], vec![3], vec![]]
        );
        for index in [0, 3, 4] {
            let cleanup = caller
                .cleanup_plans()
                .find(|plan| Some(plan.id()) == instructions[index].cleanup())
                .expect("call cleanup");
            assert_eq!(cleanup.site().role(), VerifiedCleanupRole::CallTrap);
        }
        assert_eq!(
            instructions[5].value_operands().collect::<Vec<_>>(),
            vec![
                instructions[3].result().expect("first Vec"),
                instructions[4].result().expect("second Vec")
            ]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[5].result().expect("Parcel")]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
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
