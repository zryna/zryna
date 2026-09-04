use super::super::owned_aggregate_lowering::projection_resolution_checks::compare;
use super::*;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawExpressionKind;

fn projection_ids(syntax: &zryna_syntax::v4::ProjectSyntaxSnapshot) -> Vec<u32> {
    syntax.files()[0].functions()[0]
        .body
        .expressions
        .iter()
        .enumerate()
        .filter(|(_, expression)| {
            matches!(
                expression.kind,
                RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. }
            )
        })
        .map(|(id, _)| u32::try_from(id).expect("bounded expression"))
        .collect()
}

#[test]
fn projection_descriptors_match_materialization_and_reuse_first_source_spans() {
    for (source, snapshot) in [
        owned_pair_projected_return_snapshot("first"),
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Disjoint),
        nested_owned_partial_local_transfer_snapshot(),
    ] {
        let sources = sources_for(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("authenticated projection source");
        let input = pair_input(&syntax, &sources);
        let ids = projection_ids(&syntax);
        assert!(!ids.is_empty());
        let first = compare(input, &ids, 1, None);
        assert!(first.diagnostics.is_empty());
        let repeated = ids.iter().chain(&ids).copied().collect::<Vec<_>>();
        let second = compare(input, &repeated, 1, None);
        assert!(second.diagnostics.is_empty());
        assert_eq!(first.places, second.places, "cache retains original prefix spans");
        assert_eq!(first.resolved, second.resolved[..ids.len()]);
        assert_eq!(first.resolved, second.resolved[ids.len()..]);
        assert!(lower(input).is_ok(), "complete source still passes independent IR verification");
    }
}

#[test]
fn projection_descriptors_preserve_prefix_capacity_before_later_field_failure() {
    let (mut source, mut snapshot) = nested_owned_partial_local_transfer_snapshot();
    let start = source.find("o.inner.text").expect("nested projection");
    let start_id = u32::try_from(start).expect("bounded source offset");
    source.replace_range(start + 8..start + 12, "nope");
    let expression = snapshot.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find(|expression| {
            expression.span.start as usize == start && expression.span.end as usize == start + 12
        })
        .expect("nested projection expression");
    let RawExpressionKind::FieldAccess { field, .. } = &mut expression.kind else {
        panic!("field")
    };
    field.text = "nope".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated unknown final field");
    let ids = projection_ids(&syntax);
    let final_id = *ids.last().expect("nested final field");
    let input = pair_input(&syntax, &sources);
    for initial in [1, ir::MAX_PLACES_PER_FUNCTION - 1, ir::MAX_PLACES_PER_FUNCTION] {
        let result = compare(input, &[final_id], initial, None);
        assert_eq!(result.resolved, vec![None]);
        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        if initial == ir::MAX_PLACES_PER_FUNCTION {
            assert!(result.places.is_empty());
            assert_eq!(diagnostic.code(), "ZRYNA-M3201");
            assert_eq!(
                diagnostic.message(),
                "derived owned projection places exceed the per-function M3 limit"
            );
            assert_eq!(
                diagnostic.primary_span().map(|at| (at.start(), at.end())),
                Some((start_id, start_id + 7))
            );
        } else {
            assert_eq!(result.places.len(), 1, "earlier prefix remains materialized on failure");
            assert_eq!(diagnostic.code(), "ZRYNA-M3006");
            assert_eq!(diagnostic.message(), "struct 'Inner' has no field 'nope'");
            assert_eq!(
                diagnostic.primary_span().map(|at| (at.start(), at.end())),
                Some((start_id + 8, start_id + 12))
            );
        }
    }
    let errors = lower(input).expect_err("same authenticated bad field through source lowering");
    assert_eq!(errors[0].code(), "ZRYNA-M3006");
    assert_eq!(errors[0].message(), "struct 'Inner' has no field 'nope'");
}

#[test]
fn projection_descriptors_cached_paths_survive_full_capacity_and_first_extra_rejects() {
    let (source, snapshot) = nested_owned_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated nested projection");
    let id = *projection_ids(&syntax).last().expect("nested final field");
    let input = pair_input(&syntax, &sources);
    let exact = compare(input, &[id, id], ir::MAX_PLACES_PER_FUNCTION - 2, None);
    assert!(exact.diagnostics.is_empty());
    assert_eq!(exact.places.len(), 2);
    assert_eq!(exact.resolved[0], exact.resolved[1]);
    let extra = compare(input, &[id], ir::MAX_PLACES_PER_FUNCTION - 1, None);
    assert_eq!(extra.places.len(), 1, "retain successful first prefix");
    assert_eq!(extra.diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(
        extra.diagnostics[0].primary_span(),
        Some(span(&sources, syntax.files()[0].functions()[0].body.expressions[id as usize].span,))
    );
    assert!(lower(input).is_ok(), "valid replay after synthetic capacity checks");
}

#[test]
fn projection_descriptors_preserve_index_and_base_diagnostics() {
    for (source, snapshot, message) in [
        {
            let (s, r) = struct_index_wrong_base_snapshot();
            (s, r, "owned indexing currently admits only fixed-array projections")
        },
        {
            let (s, r) = fixed_array_field_wrong_base_snapshot();
            (s, r, "owned field projection requires an exact struct place")
        },
        {
            let (s, r) = owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Dynamic);
            (s, r, "owned fixed-array indices must be compile-time i32 literals")
        },
        {
            let (s, r) = owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Negative);
            (s, r, "owned fixed-array indices must be compile-time i32 literals")
        },
        {
            let (s, r) =
                owned_array_projected_return_snapshot(OwnedArrayProjectionCase::OutOfBounds);
            (s, r, "owned fixed-array index 2 is outside length 2")
        },
    ] {
        let sources = sources_for(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("authenticated invalid projection");
        let ids = projection_ids(&syntax);
        let input = pair_input(&syntax, &sources);
        let result = compare(input, &ids, 1, None);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|error| error.code() == "ZRYNA-M3006" && error.message() == message),
            "expected {message:?} for {source:?}; actual {:?}",
            result.diagnostics,
        );
        let errors = lower(input).expect_err("unchanged full source admission gate");
        assert!(
            errors.iter().any(|error| error.code() == "ZRYNA-M3006" && error.message() == message),
            "expected {message:?} for {source:?}; actual {errors:?}",
        );
    }
}

#[test]
fn projection_descriptors_retain_field_semantic_type_fallback_diagnostic() {
    let (source, snapshot) = nested_owned_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated nested nominal source");
    let input = pair_input(&syntax, &sources);
    let id = *projection_ids(&syntax).last().expect("nested final field");
    // Deliberately incomplete semantic lookup after successful source/layout authentication.
    let result = compare(input, &[id], 1, Some("Inner"));
    assert!(result.places.is_empty());
    assert_eq!(result.resolved, vec![None]);
    assert_eq!(result.diagnostics.len(), 2);
    assert_eq!(result.diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(result.diagnostics[1].code(), "ZRYNA-M3006");
    assert_eq!(result.diagnostics[1].message(), "struct 'Outer' has no field 'inner'");
    assert!(lower(input).is_ok(), "normal full authority remains usable");
}
