use super::generic_vec_fixture::{Element, Operation, fixture};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

fn elements() -> [Element; 6] {
    [
        Element::Shared,
        Element::Weak,
        Element::HandleStruct,
        Element::HandleEnum,
        Element::HandleArray,
        Element::HandleVec,
    ]
}

fn reject(
    source: &str,
    raw: RawProjectSyntaxSnapshot,
    code: &str,
    at: zryna_source::UntrustedSpan,
    message: &str,
    guidance: &str,
) {
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected handle Vec source");
    let expected =
        zryna_diagnostics::Diagnostic::error_at(code, span(&sources, at), message, guidance);
    for _ in 0..2 {
        assert_eq!(
            lower(pair_input(&syntax, &sources)).expect_err("rejected handle Vec"),
            std::slice::from_ref(&expected)
        );
    }
}

#[test]
fn generic_vec_handle_bare_observation_rejects_implicit_move_and_recovers() {
    for element in elements() {
        let (source, raw) = fixture(&element, Operation::Read, None);
        let at = raw.files[0].functions[0]
            .body
            .expressions
            .last()
            .expect("verified generic Vec handle shape")
            .span;
        reject(
            &source,
            raw,
            "ZRYNA-M3013",
            at,
            "Vec observation requires its exact Copy element or an explicit owned clone",
            "read the exact Copy element type or explicitly clone the owned indexed element",
        );
        let (source, raw) = fixture(&element, Operation::Clone, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery");
        lower(pair_input(&syntax, &sources)).expect("explicit handle clone recovers");
    }
}

#[test]
fn generic_vec_handle_replacement_rejects_overlap_before_nested_index_effects() {
    for element in elements() {
        let (source, raw) = fixture(&element, Operation::ReplaceSelfClone, None);
        let body = &raw.files[0].functions[0].body;
        let rhs = body
            .statements
            .iter()
            .find_map(|statement| match statement.kind {
                RawStatementKind::Assignment { value, .. } => Some(value),
                _ => None,
            })
            .expect("verified generic Vec handle shape");
        let at = body.expressions[rhs as usize].span;
        reject(
            &source,
            raw,
            "ZRYNA-M3014",
            at,
            "owner access conflicts with an active borrow",
            "finish the indexed operation before accessing or consuming its container",
        );
    }
}

#[test]
fn generic_vec_handle_replacement_rejects_wrong_exact_owned_type_and_recovers() {
    for element in elements() {
        let (source, raw) = fixture(&element, Operation::ReplaceString, None);
        let body = &raw.files[0].functions[0].body;
        let rhs = body
            .statements
            .iter()
            .find_map(|statement| match statement.kind {
                RawStatementKind::Assignment { value, .. } => Some(value),
                _ => None,
            })
            .expect("verified generic Vec handle shape");
        let RawExpressionKind::Reference { .. } = body.expressions[rhs as usize].kind else {
            panic!("reference")
        };
        let at = body.expressions[rhs as usize].span;
        reject(
            &source,
            raw,
            "ZRYNA-M3016",
            at,
            "aggregate operand has the wrong exact type",
            "use the exact declared field, element, local, or result type",
        );
        let (source, raw) = fixture(&element, Operation::Replace, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated exact recovery");
        lower(pair_input(&syntax, &sources)).expect("exact handle replacement recovers");
    }
}
