use super::*;
use crate::data_ownership_v1::tests::constructor_envelope_fixtures::{self, Fixture};

// Remap only the closed DTO vocabulary in the fixed scalar fixture; no semantic dispatch.
pub(super) fn offset_inputs(expression: &mut RawExpressionSyntax, minimum: u32, amount: u32) {
    let bump = |id: &mut u32| {
        if *id >= minimum {
            *id += amount;
        }
    };
    match &mut expression.kind {
        RawExpressionKind::Subtraction { lhs, rhs, .. }
        | RawExpressionKind::Equal { lhs, rhs, .. } => {
            bump(lhs);
            bump(rhs);
        }
        RawExpressionKind::VecConstruction { elements, .. } => elements.iter_mut().for_each(bump),
        RawExpressionKind::StructConstruction { fields, .. } => {
            for field in fields {
                let RawFieldInitializerKind::Explicit { value, .. } = &mut field.kind else {
                    panic!("explicit field")
                };
                bump(value);
            }
        }
        RawExpressionKind::Call { arguments, .. } => arguments.iter_mut().for_each(bump),
        RawExpressionKind::Clone { value, .. } => bump(value),
        RawExpressionKind::StringLiteral { .. }
        | RawExpressionKind::BoolLiteral { .. }
        | RawExpressionKind::Reference { .. } => {}
        _ => panic!("unexpected fixed fixture expression"),
    }
}

pub(super) fn prepend_pair_local(source: &mut String, raw: &mut RawProjectSyntaxSnapshot) {
    let (pair_source, pair) = constructor_envelope_fixtures::snapshot(Fixture::Pair);
    assert_eq!(pair.files[0].data_declarations.len(), 1);
    assert_eq!(pair.files[0].type_syntax.len(), 4);
    let declaration = &pair.files[0].data_declarations[0];
    let start = raw.files[0].functions[0].span.start;
    assert_eq!(declaration.span.start, 0);
    let declaration_text =
        &pair_source[..usize::try_from(declaration.span.end).expect("declaration end")];
    let prefix = format!("{declaration_text} ");
    source.insert_str(usize::try_from(start).expect("function offset"), &prefix);
    *raw = shift_snapshot(raw.clone(), start, u32::try_from(prefix.len()).expect("prefix length"));
    let shifted = shift_snapshot(pair.clone(), 0, start);
    let base = u32::try_from(raw.files[0].type_syntax.len()).expect("type base");
    raw.files[0].type_syntax.extend_from_slice(&shifted.files[0].type_syntax[..2]);
    let mut declaration = shifted.files[0].data_declarations[0].clone();
    let RawDataDeclarationKind::Struct { fields, .. } = &mut declaration.kind else {
        panic!("OwnedPair declaration")
    };
    for field in fields {
        field.type_syntax += base;
    }
    raw.files[0].data_declarations.push(declaration);

    let local = &pair.files[0].functions[0].body.statements[0];
    let old_start = local.span.start;
    let inserted = raw.files[0].functions[0].body.statements[0].span.start;
    let text = &pair_source[usize::try_from(old_start).expect("local start")
        ..usize::try_from(local.span.end).expect("local end")];
    let prefix = format!("{text} ");
    source.insert_str(usize::try_from(inserted).expect("return offset"), &prefix);
    *raw =
        shift_snapshot(raw.clone(), inserted, u32::try_from(prefix.len()).expect("local length"));
    let shifted =
        shift_snapshot(pair, old_start, inserted.checked_sub(old_start).expect("longer caller"));
    let mut local = shifted.files[0].functions[0].body.statements[0].clone();
    let ty = u32::try_from(raw.files[0].type_syntax.len()).expect("local type ID");
    raw.files[0].type_syntax.push(shifted.files[0].type_syntax[3].clone());
    let RawStatementKind::LocalDeclaration { name, type_syntax, initializer, .. } = &mut local.kind
    else {
        panic!("real Pair local")
    };
    assert_eq!(name.text, "p");
    assert_eq!(*initializer, 2);
    *type_syntax = ty;
    let body = &mut raw.files[0].functions[0].body;
    for expression in &mut body.expressions {
        offset_inputs(expression, 0, 3);
    }
    body.expressions
        .splice(0..0, shifted.files[0].functions[0].body.expressions[..3].iter().cloned());
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("original return")
    };
    *value += 3;
    body.statements.insert(0, local);
    body.blocks[0].statements = vec![0, 1];
}
