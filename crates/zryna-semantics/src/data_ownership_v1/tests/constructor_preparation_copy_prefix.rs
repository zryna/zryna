use super::*;

#[test]
fn constructor_preparation_controls_cached_copy_prefix_is_not_created_twice() {
    let (source, snapshot) = fixtures::snapshot(Fixture::Projection);
    replay(&source, snapshot.clone(), VerifiedInstructionKind::StructConstruct, 2, 0, 1, false);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty));
        let flag = lowerer.function.body.expressions.iter().position(|expression|
            matches!(&expression.kind, RawExpressionKind::FieldAccess { field, .. } if field.text == "flag"))
            .and_then(|id| u32::try_from(id).ok()).expect("actual Copy projection");
        let cached = lowerer.owned_place(flag).expect("first canonical Copy prefix");
        assert!(cached.ty.is_copy());
        let original_places = lowerer.places.clone();
        let original_span = original_places[cached.place.0 as usize].span;
        let original_projections = lowerer.projections.clone();
        let owners = lowerer.owners.clone();
        assert_eq!(owners.pending().len(), 1, "one genuine preceding Pair root");
        let before = lowerer.preparation_checkpoint();
        let id = root_value(lowerer, 1);
        let prepared = PreparedValue::prepare(lowerer, id, ty).expect("projected constructor");
        assert_eq!(prepared.lowerer.preparation_checkpoint(), before);
        assert_eq!(prepared.lowerer.places, original_places);
        assert_eq!(prepared.lowerer.projections, original_projections);
        assert_eq!(prepared.lowerer.owners, owners);
        assert_eq!(prepared.plan.visits, 3);
        assert_eq!(prepared.plan.steps.iter().filter(|step| step.value.is_some()).count(), 3);
        let prefixes = prepared
            .plan
            .steps
            .iter()
            .filter_map(|step| {
                if let Operation::Prefix { id, .. } = &step.operation { Some(*id) } else { None }
            })
            .collect::<Vec<_>>();
        assert_eq!(prefixes.len(), 1, "only the uncached moved String field needs a prefix");
        assert_ne!(prefixes[0], cached.place);
        assert_eq!(prefixes[0].0 as usize, original_places.len());
        assert_eq!(prepared.plan.projections.len(), original_projections.len() + 1);
        let result = prepared.plan.result;
        assert_eq!(prepared.consume(), result);
        assert_eq!(lowerer.owned_place(flag).expect("same cached Copy prefix").place, cached.place);
        assert_eq!(lowerer.places[cached.place.0 as usize].span, original_span);
        assert!(lowerer.owners.contains(owners.pending()[0]));
        assert_eq!(lowerer.moved_projections.len(), 1, "only String sibling was moved");
        assert!(lowerer.partial_roots.contains(&owners.pending()[0]));
    });
    assert!(errors.is_empty());
}
