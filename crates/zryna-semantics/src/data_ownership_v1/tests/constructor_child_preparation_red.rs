use super::*;
use zryna_ir::data_ownership_v1::raw;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::data_ownership_v1::owned_aggregate_lowering) struct PreparationState {
    arenas: String,
    constructor_types: String,
    bindings: String,
    owners: OwnerState,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved_projections: BTreeSet<raw::PlaceId>,
    partial_roots: BTreeSet<raw::PlaceId>,
    counters: [usize; 6],
    held: [usize; 4],
    next_local: u32,
    sites: [usize; 3],
}

pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn state(
    lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>,
) -> PreparationState {
    PreparationState {
        arenas: format!(
            "{:?}\n{:?}\n{:?}",
            lowerer.instructions, lowerer.places, lowerer.cleanup_plans
        ),
        // Derived Debug includes both private dense types and scanned_instructions; no
        // production visibility or cache observation is needed to capture either field.
        constructor_types: format!("{:?}", lowerer.constructor_types),
        bindings: format!("{:?}", lowerer.bindings),
        owners: lowerer.owners.clone(),
        projections: lowerer.projections.clone(),
        moved_projections: lowerer.moved_projections.clone(),
        partial_roots: lowerer.partial_roots.clone(),
        counters: [
            lowerer.next_value as usize,
            lowerer.places.len(),
            lowerer.instructions.len(),
            lowerer.aggregate_operands,
            lowerer.cleanup_plans.len(),
            lowerer.cleanup_actions,
        ],
        held: credits(lowerer),
        next_local: lowerer.next_local,
        sites: [
            lowerer.aggregate_subobject_moves,
            lowerer.projected_aggregate_clones,
            lowerer.projected_aggregate_assignments,
        ],
    }
}

pub(super) fn replace_literal_with_reference(
    source: &mut String,
    expression: &mut zryna_syntax::v4::RawExpressionSyntax,
    name: &str,
) -> zryna_source::UntrustedSpan {
    let at = expression.span;
    assert_eq!(at.end - at.start, u32::try_from(name.len()).expect("short name"));
    source.replace_range(at.start as usize..at.end as usize, name);
    expression.kind = RawExpressionKind::Reference {
        name: zryna_syntax::v4::RawIdentifierSyntax { text: name.to_owned(), span: at },
    };
    at
}

pub(super) fn unresolved(at: Span, name: &str) -> Diagnostic {
    Diagnostic::error_at(
        "ZRYNA-M3002",
        at,
        format!("aggregate value '{name}' is not declared"),
        "reference one exact preceding local using its declared spelling",
    )
}

#[test]
fn constructor_child_preparation_red_later_invalid_after_literal_preserves_real_state() {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Pair);
    let bad = replace_literal_with_reference(
        &mut source,
        &mut snapshot.files[0].functions[0].body.expressions[0],
        "lost",
    );
    let (mut before, mut after, mut expected) = (None, None, None);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        assert!(lowerer.errors.is_empty());
        before = Some(state(lowerer));
        expected = Some(unresolved(span(lowerer.input.sources(), bad), "lost"));
        assert!(lowerer.value(root_value(lowerer, 0), result).is_none());
        after = Some(state(lowerer));
    });
    assert_eq!(errors, [expected.expect("exact source-bound diagnostic")]);
    // Red on the current materializer: the earlier String child remains emitted. The
    // diagnostic assertion above must pass before this required C2 invariant is tested.
    assert_eq!(after, before, "C2 rejected child preparation must preserve complete real state");
}

#[test]
fn constructor_child_preparation_red_later_invalid_after_whole_move_preserves_real_state() {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::NestedPartialTransfer);
    let body = &mut snapshot.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { name, .. } = &mut body.statements[1].kind else {
        panic!("String local")
    };
    assert_eq!(name.text, "text");
    source.replace_range(name.span.start as usize..name.span.end as usize, "txt ");
    name.text = "txt".to_owned();
    name.span.end -= 1;
    let moved = body.expressions.iter_mut().find(|expression| matches!(&expression.kind, RawExpressionKind::StringLiteral { spelling } if spelling == "\"d\"")).expect("final Inner String child");
    replace_literal_with_reference(&mut source, moved, "txt");
    let invalid = body.expressions.iter_mut().find(|expression| matches!(&expression.kind, RawExpressionKind::StringLiteral { spelling } if spelling == "\"c\"")).expect("final Outer tail");
    let bad = replace_literal_with_reference(&mut source, invalid, "bad");
    let (mut before, mut after, mut expected) = (None, None, None);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        // Real preceding statements establish the String owner, partial-root topology,
        // transferred aggregate owner and emission-observed constructor cache.
        for statement in 0..3 {
            assert!(run_statement(lowerer, statement, result));
        }
        let binding = lowerer.bindings.get("txt").expect("preceding String local");
        assert_eq!(binding.ty.category, zryna_layout::TypeCategory::String);
        assert!(lowerer.owners.contains(binding.place));
        assert!(lowerer.errors.is_empty());
        before = Some(state(lowerer));
        expected = Some(unresolved(span(lowerer.input.sources(), bad), "bad"));
        assert!(lowerer.value(root_value(lowerer, 3), result).is_none());
        after = Some(state(lowerer));
    });
    assert_eq!(errors, [expected.expect("exact source-bound diagnostic")]);
    // Declared inner-before-tail evaluation moves the whole String local into Inner,
    // then rejects the unresolved tail. The future planner must leave neither effect.
    assert_eq!(after, before, "C2 rejected child preparation must preserve complete real state");
}
