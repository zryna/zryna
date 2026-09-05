use super::nested_mixed_construction::root_replacement::{
    ReplacementCase, ReplacementRoot, replacement_fixture,
};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

fn shift_clone_spans(value: &mut serde_json::Value, start: u32, end: u32) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                shift_clone_spans(value, start, end);
            }
        }
        serde_json::Value::Object(values) => {
            if values.contains_key("file")
                && values.contains_key("start")
                && values.contains_key("end")
            {
                for key in ["start", "end"] {
                    let offset = values[key].as_u64().expect("span offset");
                    let after = if key == "start" {
                        offset >= u64::from(end)
                    } else {
                        offset > u64::from(end)
                    };
                    let within = if key == "start" {
                        offset >= u64::from(start)
                    } else {
                        offset > u64::from(start)
                    };
                    let delta = if after {
                        7
                    } else if within {
                        6
                    } else {
                        0
                    };
                    values.insert(key.into(), (offset + delta).into());
                }
            } else {
                for value in values.values_mut() {
                    shift_clone_spans(value, start, end);
                }
            }
        }
        _ => {}
    }
}

pub(in crate::data_ownership_v1) fn fixture(
    root: ReplacementRoot,
    assignment: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = replacement_fixture(
        root,
        if assignment { ReplacementCase::SelfDirect } else { ReplacementCase::Constructor },
    );
    let body = &raw.files[0].functions[0].body;
    let selected = body
        .statements
        .iter()
        .find_map(|statement| match statement.kind {
            RawStatementKind::Assignment { value, .. } if assignment => Some(value),
            RawStatementKind::Return { value, .. } if !assignment => Some(value),
            _ => None,
        })
        .expect("clone operand");
    let old = body.expressions[selected as usize].span;
    assert!(matches!(
        body.expressions[selected as usize].kind,
        RawExpressionKind::Reference { .. }
    ));
    source.insert(old.end as usize, ')');
    source.insert_str(old.start as usize, "clone(");
    let mut json = serde_json::to_value(raw).expect("fixture serialization");
    shift_clone_spans(&mut json, old.start, old.end);
    let mut raw: RawProjectSyntaxSnapshot = serde_json::from_value(json).expect("shifted fixture");
    let body = &mut raw.files[0].functions[0].body;
    let at = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let clone = RawExpressionSyntax {
        span: at(old.start, old.end + 7),
        kind: RawExpressionKind::Clone {
            keyword_span: at(old.start, old.start + 5),
            open_paren_span: at(old.start + 5, old.start + 6),
            value: selected,
            close_paren_span: at(old.end + 6, old.end + 7),
        },
    };
    body.expressions.insert(selected as usize + 1, clone);
    for statement in &mut body.statements {
        match &mut statement.kind {
            RawStatementKind::Assignment { value, .. } if assignment && *value == selected => {
                *value += 1;
            }
            RawStatementKind::Return { value, .. } if *value >= selected => *value += 1,
            _ => {}
        }
    }
    (source, raw)
}

#[test]
fn generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for assignment in [false, true] {
            let (source, raw) = fixture(root, assignment);
            let sources = sources_for(&source);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated generic clone source");
            let mut previous = None;
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                    panic!("{root:?} assignment={assignment}: {errors:?}")
                });
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let clone = instructions
                    .iter()
                    .find(|i| i.kind() == VerifiedInstructionKind::GenericClonePlace)
                    .expect("generic clone opcode");
                let source = clone.place_operands().next().expect("retained source");
                let result = clone.result().expect("distinct result");
                let owner = function
                    .places()
                    .find(|place| place.kind() == VerifiedPlaceKind::Temporary(result))
                    .expect("result owner");
                assert_ne!(owner.id(), source);
                assert!(clone.derived_drop_actions().any(|drop| drop.root() == source));
                let prefix = clone.generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
                assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
                assert_eq!(prefix[0].root(), owner.id());
                assert_eq!(
                    &prefix[1..],
                    clone.derived_drop_actions().collect::<Vec<_>>().as_slice()
                );
                let trace = instructions
                    .iter()
                    .map(|instruction| format!("{:?}", instruction.kind()))
                    .collect::<Vec<_>>();
                if let Some(previous) = previous.replace(trace.clone()) {
                    assert_eq!(previous, trace);
                }
                if assignment {
                    let commit = instructions
                        .iter()
                        .rfind(|i| i.kind() == VerifiedInstructionKind::ReplacePlace)
                        .expect("clone replacement commit");
                    assert_eq!(commit.place_operands().collect::<Vec<_>>(), [source]);
                    assert_eq!(
                        commit.derived_drop_actions().next().expect("old value").root(),
                        source
                    );
                } else {
                    assert!(
                        block.terminator().derived_drop_actions().any(|drop| drop.root() == source)
                    );
                }
            }
        }
    }
}
