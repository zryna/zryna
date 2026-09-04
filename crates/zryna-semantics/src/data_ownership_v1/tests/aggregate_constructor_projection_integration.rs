use super::*;
use zryna_ir::data_ownership_v1::raw;

fn field_expression(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, name: &str) -> u32 {
    lowerer
        .function
        .body
        .expressions
        .iter()
        .enumerate()
        .find(|(_, expression)| {
            matches!(&expression.kind,
            RawExpressionKind::FieldAccess { field, .. } if field.text == name)
        })
        .map(|(index, _)| u32::try_from(index).expect("fixture expression"))
        .expect("authenticated field expression")
}

#[test]
fn constructor_envelope_direct_projection_adapter_preserves_credit_ids_cache_and_release() {
    let errors = with_fixture(Fixture::Projection, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let first_expression = field_expression(lowerer, "first");
        let flag_expression = field_expression(lowerer, "flag");
        let first = lowerer.owned_place_preflight(first_expression).expect("String field type");
        let flag = lowerer.owned_place_preflight(flag_expression).expect("bool field type");
        assert_eq!(first.place.root, flag.place.root);
        assert_eq!(first.place.ty.category, zryna_layout::TypeCategory::String);
        assert_eq!(flag.place.ty.category, zryna_layout::TypeCategory::Bool);
        let base = first.place.root;
        let before = lowerer.places.len();
        // Injected surrounding credits model a frontier, not dense maximum source IR.
        let held = [0, 0, 0, LIMITS[3] - before - 2];
        set_credits(lowerer, held);
        let at = span(
            lowerer.input.sources(),
            lowerer.function.body.expressions[first_expression as usize].span,
        );
        let later = span(lowerer.input.sources(), lowerer.function.span);
        let parent = lowerer.reserve_constructor_commit(result, 2, at).expect("parent fits");
        let pending = lowerer.owners.pending().to_vec();
        let instructions = lowerer.instructions.len();
        let first_place = lowerer
            .push_projection(
                first.place.ty,
                at,
                (base.0, 0, 0),
                raw::PlaceKind::StructField { base, ordinal: 0 },
            )
            .expect("one unreserved place");
        assert_eq!(first_place.0 as usize, before, "committed ID excludes held credits");
        assert_eq!(lowerer.budget_places(), LIMITS[3]);
        assert_eq!(
            lowerer.push_projection(
                first.place.ty,
                later,
                (base.0, 0, 0),
                raw::PlaceKind::StructField { base, ordinal: 0 }
            ),
            Some(first_place)
        );
        assert_eq!(lowerer.places[first_place.0 as usize].span, at, "first span retained");
        assert!(
            lowerer
                .push_projection(
                    flag.place.ty,
                    later,
                    (base.0, 0, 1),
                    raw::PlaceKind::StructField { base, ordinal: 1 }
                )
                .is_none()
        );
        assert_eq!(lowerer.places.len(), before + 1);
        assert_eq!(lowerer.projections.len(), 1);
        parent.release(lowerer);
        assert_eq!(credits(lowerer), held);
        let flag_place = lowerer
            .push_projection(
                flag.place.ty,
                later,
                (base.0, 0, 1),
                raw::PlaceKind::StructField { base, ordinal: 1 },
            )
            .expect("released parent slot");
        assert_eq!(flag_place.0 as usize, before + 1);
        assert_eq!(lowerer.budget_places(), LIMITS[3]);
        assert_eq!(lowerer.owners.pending(), pending);
        assert_eq!(lowerer.instructions.len(), instructions);
    });
    assert_diagnostic(&errors, "derived owned projection places exceed the per-function M3 limit");
}

#[test]
fn constructor_envelope_nested_projection_prefix_precedence_includes_held_parent_places() {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::NestedPartialTransfer);
    let start = source.find("o.inner.text").expect("nested source projection");
    source.replace_range(start + 8..start + 12, "nope");
    let (id, expression) = snapshot.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .enumerate()
        .find(|(_, expression)| {
            expression.span.start as usize == start && expression.span.end as usize == start + 12
        })
        .expect("nested projection");
    let RawExpressionKind::FieldAccess { field, .. } = &mut expression.kind else {
        panic!("field")
    };
    field.text = "nope".to_owned();
    let id = u32::try_from(id).expect("bounded expression");
    for free_prefixes in 0..2 {
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            assert!(lowerer.projections.is_empty());
            let before = lowerer.places.len();
            let instructions = lowerer.instructions.len();
            let pending = lowerer.owners.pending().to_vec();
            // Injected surrounding credits model a frontier, not dense maximum source IR.
            let held = [0, 0, 0, LIMITS[3] - before - 1 - free_prefixes];
            set_credits(lowerer, held);
            let at =
                span(lowerer.input.sources(), lowerer.function.body.expressions[id as usize].span);
            let parent = lowerer.reserve_constructor_commit(result, 2, at).expect("parent fits");
            assert!(lowerer.owned_place(id).is_none());
            assert_eq!(lowerer.places.len(), before + free_prefixes);
            assert_eq!(lowerer.projections.len(), free_prefixes);
            if free_prefixes == 1 {
                let prefix = &lowerer.places[before];
                assert_eq!(prefix.id.0 as usize, before);
                assert_eq!(
                    (prefix.span.start(), prefix.span.end()),
                    (u32::try_from(start).expect("span"), u32::try_from(start + 7).expect("span"))
                );
            }
            assert_eq!(lowerer.instructions.len(), instructions);
            assert_eq!(lowerer.owners.pending(), pending);
            parent.release(lowerer);
            assert_eq!(credits(lowerer), held);
        });
        assert_eq!(errors.len(), 1);
        let (code, message, offset, length) = if free_prefixes == 0 {
            (
                "ZRYNA-M3201",
                "derived owned projection places exceed the per-function M3 limit",
                0,
                7,
            )
        } else {
            ("ZRYNA-M3006", "struct 'Inner' has no field 'nope'", 8, 4)
        };
        assert_eq!(errors[0].code(), code);
        assert_eq!(errors[0].message(), message);
        assert_eq!(
            errors[0].primary_span().map(|at| (at.start() as usize, at.end() as usize)),
            Some((start + offset, start + offset + length))
        );
    }
    let (source, snapshot) = fixtures::snapshot(Fixture::NestedPartialTransfer);
    let sources = fixtures::sources(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("valid source replay");
    assert!(
        ownership::lower(fixtures::input(&syntax, &sources)).is_ok(),
        "independent full IR replay"
    );
}
