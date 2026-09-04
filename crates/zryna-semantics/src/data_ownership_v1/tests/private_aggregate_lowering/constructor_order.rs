use super::*;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

fn invalid_initializer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = OWNED_TRIO_SOURCE.to_owned();
    let mut raw = response_snapshot(OWNED_TRIO_RESPONSE);
    for expression in &mut raw.files[0].functions[0].body.expressions[..3] {
        let at = expression.span;
        source.replace_range(at.start as usize..at.end as usize, "bad");
        expression.kind = RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "bad".to_owned(), span: at },
        };
    }
    (source, raw)
}

#[test]
fn complete_owned_field_mapping_precedes_initializer_diagnostics() {
    for (replacement, message) in
        [("z", "struct 'Trio' has no field 'z'"), ("c", "field 'c' is initialized more than once")]
    {
        let (mut source, mut raw) = invalid_initializer_snapshot();
        source.replace_range(118..119, replacement);
        let RawExpressionKind::StructConstruction { fields, .. } =
            &mut raw.files[0].functions[0].body.expressions[3].kind
        else {
            panic!("Trio constructor")
        };
        let RawFieldInitializerKind::Explicit { name, .. } = &mut fields[1].kind else {
            panic!("explicit field")
        };
        name.text = replacement.to_owned();
        let sources = sources_for(&source);
        if replacement == "c" {
            let errors = verify_snapshot(raw, &sources).expect_err("duplicate syntax fails first");
            assert_eq!(errors[0].code(), "ZRYNA-Y4002");
            assert_eq!(errors[0].message(), "duplicate initializer name");
            continue;
        }
        let syntax = verify_snapshot(raw, &sources).expect("authenticated malformed mapping");
        for _ in 0..2 {
            let errors =
                lower(pair_input(&syntax, &sources)).expect_err("mapping before evaluation");
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M3016");
            assert_eq!(errors[0].message(), message);
            assert_eq!(errors[0].primary_span().map(|s| (s.start(), s.end())), Some((118, 124)));
        }
    }

    let (mut source, mut raw) = invalid_initializer_snapshot();
    source.replace_range(124..132, "");
    let body = &mut raw.files[0].functions[0].body;
    let RawExpressionKind::StructConstruction { fields, .. } = &mut body.expressions[3].kind else {
        panic!("Trio constructor")
    };
    fields.pop();
    body.expressions.remove(2);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 2;
    let raw = shift_snapshot_signed(raw, 132, -8);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated missing field");
    let errors = lower(pair_input(&syntax, &sources)).expect_err("missing before evaluation");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3016");
    assert_eq!(errors[0].message(), "field 'a' is not initialized");
    assert_eq!(errors[0].primary_span().map(|s| (s.start(), s.end())), Some((103, 127)));
}

#[test]
fn owned_initializer_errors_follow_declarations_not_source_order() {
    let (source, raw) = invalid_initializer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid initializer names");
    for _ in 0..2 {
        let errors = lower(pair_input(&syntax, &sources)).expect_err("first declaration fails");
        assert_eq!(errors[0].code(), "ZRYNA-M3002");
        assert_eq!(errors[0].primary_span().map(|s| (s.start(), s.end())), Some((129, 132)));
    }
}

#[test]
fn constructor_evaluation_order_does_not_permit_reordered_syntax_fields() {
    let sources = sources_for(OWNED_TRIO_SOURCE);
    let mut raw = response_snapshot(OWNED_TRIO_RESPONSE);
    let RawExpressionKind::StructConstruction { fields, .. } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("Trio constructor")
    };
    fields.reverse();
    assert!(verify_snapshot(raw, &sources).is_err());
}

#[test]
fn owned_constructor_moves_field_before_later_copy_read_without_dropping_it() {
    let (source, raw) = owned_pair_projected_return_snapshot("first");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated reversed moved field");
    let program = lower(pair_input(&syntax, &sources)).expect("disjoint remaining Copy field");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let tail = &instructions[instructions.len() - 3..];
    assert_eq!(
        tail.iter().map(|i| i.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::CopyFromPlace,
            VerifiedInstructionKind::StructConstruct,
        ]
    );
    assert!(tail.iter().all(|i| i.cleanup().is_none()));
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].moved_projections().count(), 1);
    assert_eq!(cleanup[0].initialized_projections().count(), 1);
}
