use super::generic_vec_fixture::{Element, Operation, fixture};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

fn assert_rejected(
    source: &str,
    raw: RawProjectSyntaxSnapshot,
    code: &str,
    at: zryna_source::UntrustedSpan,
    message: &str,
    guidance: &str,
) {
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected Vec operation");
    let check =
        || lower(pair_input(&syntax, &sources)).expect_err("invalid ordinary Vec operation");
    let first = check();
    assert_eq!(
        first,
        vec![zryna_diagnostics::Diagnostic::error_at(code, span(&sources, at), message, guidance)],
        "{source}"
    );
    assert_eq!(first, check());
}

#[test]
fn generic_vec_source_owned_bare_index_never_moves_an_element_or_leaves_a_hole() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let (source, raw) = fixture(&element, Operation::Read, None);
        let at = raw.files[0].functions[0].body.expressions.last().expect("Index").span;
        assert_rejected(
            &source,
            raw,
            "ZRYNA-M3013",
            at,
            "Vec observation requires its exact Copy element or an explicit owned clone",
            "read the exact Copy element type or explicitly clone the owned indexed element",
        );
    }
}

#[test]
fn generic_vec_source_replacement_blocks_same_container_clone_before_nested_index_evaluation() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let (source, raw) = fixture(&element, Operation::ReplaceSelfClone, None);
        let body = &raw.files[0].functions[0].body;
        let rhs = body
            .statements
            .iter()
            .find_map(|statement| match statement.kind {
                RawStatementKind::Assignment { value, .. } => Some(value),
                _ => None,
            })
            .expect("RHS clone");
        let at = body.expressions[rhs as usize].span;
        assert_rejected(
            &source,
            raw,
            "ZRYNA-M3014",
            at,
            "Vec operation conflicts with an active whole-container access",
            "finish the indexed operation before accessing or consuming its container",
        );
    }
}

#[test]
fn generic_vec_source_replacement_rejects_immutable_wrong_type_and_missing_rhs() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let (source, raw) = fixture(&element, Operation::Replace, None);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw.clone(), &sources).expect("authenticated replacement control");
        lower(pair_input(&syntax, &sources)).expect("valid exact owned replacement control");

        let mut immutable_source = source.clone();
        let RawStatementKind::LocalDeclaration { keyword_span, .. } =
            raw.files[0].functions[0].body.statements[0].kind
        else {
            panic!("items local");
        };
        immutable_source
            .replace_range(keyword_span.start as usize..keyword_span.end as usize, "const");
        let mut immutable = shift_snapshot(raw.clone(), keyword_span.end, 2);
        let RawStatementKind::LocalDeclaration { mutable, .. } =
            &mut immutable.files[0].functions[0].body.statements[0].kind
        else {
            panic!("items local");
        };
        *mutable = false;
        let target = immutable.files[0].functions[0]
            .body
            .statements
            .iter()
            .find_map(|statement| match statement.kind {
                RawStatementKind::Assignment { target, .. } => Some(target),
                _ => None,
            })
            .expect("target");
        let target_at = immutable.files[0].functions[0].body.expressions[target as usize].span;
        assert_rejected(
            &immutable_source,
            immutable,
            "ZRYNA-M3014",
            target_at,
            "indexed Vec is immutable for mutation, unavailable, or partially moved",
            "use one complete initialized Vec with exclusive mutation access",
        );

        let rhs = raw.files[0].functions[0]
            .body
            .statements
            .iter()
            .find_map(|statement| match statement.kind {
                RawStatementKind::Assignment { value, .. } => Some(value),
                _ => None,
            })
            .expect("replacement RHS");
        let at = raw.files[0].functions[0].body.expressions[rhs as usize].span;
        for (spelling, code) in [("index", "ZRYNA-M3016"), ("lost", "ZRYNA-M3002")] {
            let mut source = source.clone();
            source.replace_range(at.start as usize..at.end as usize, spelling);
            let mut rejected = if spelling.len() == 5 {
                shift_snapshot(raw.clone(), at.end, 1)
            } else {
                raw.clone()
            };
            let RawExpressionKind::Reference { name } =
                &mut rejected.files[0].functions[0].body.expressions[rhs as usize].kind
            else {
                panic!("reference RHS");
            };
            name.text = spelling.into();
            let at = rejected.files[0].functions[0].body.expressions[rhs as usize].span;
            let (message, guidance) = if spelling == "index" {
                (
                    "aggregate operand has the wrong exact type",
                    "use the exact declared field, element, local, or result type",
                )
            } else {
                (
                    "aggregate value 'lost' is not declared",
                    "reference one exact preceding local using its declared spelling",
                )
            };
            assert_rejected(&source, rejected, code, at, message, guidance);
        }
    }
}
