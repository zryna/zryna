use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::expression_decisions::{
    ExpressionDecisions, ExpressionKind,
};
use zryna_ir::data_ownership_v1::raw;

fn decisions<'a, 'f, 'e>(
    lowerer: &'e mut PrivateOwnedAggregateLowerer<'a, 'f, '_>,
) -> ExpressionDecisions<'a, 'f, 'e> {
    ExpressionDecisions {
        input: lowerer.input,
        file: lowerer.file,
        function: lowerer.function,
        module: lowerer.module,
        declarations: lowerer.declarations,
        graph: lowerer.graph,
        node_types: lowerer.node_types,
        layouts: lowerer.layouts,
        errors: lowerer.errors,
    }
}

fn assert_no_materialization(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) {
    assert!(lowerer.instructions.is_empty());
    assert!(lowerer.places.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    assert_eq!(lowerer.owners, OwnerState::default());
    assert!(lowerer.bindings.is_empty());
    assert!(lowerer.projections.is_empty());
    assert!(lowerer.moved_projections.is_empty());
    assert!(lowerer.partial_roots.is_empty());
    assert_eq!((lowerer.next_value, lowerer.next_local, lowerer.aggregate_operands), (0, 0, 0));
}

#[test]
fn ordered_expression_decisions_map_struct_fields_without_resolving_children_or_emitting() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        assert_no_materialization(lowerer);
        assert_eq!(credits(lowerer), [0; 4]);
        assert_eq!(lowerer.cleanup_actions, 0);
        let expression = root_value(lowerer, 0);
        let decision = decisions(lowerer).classify(expression, result).expect("outer decision");
        let ExpressionKind::Struct(shape) = decision.kind else { panic!("struct decision") };
        assert_eq!(shape.children.len(), 2);
        let types = shape
            .children
            .iter()
            .map(|(syntax, _)| {
                decisions(lowerer).child_type(*syntax).expect("exact child type").category
            })
            .collect::<Vec<_>>();
        assert_eq!(types, [zryna_layout::TypeCategory::String, zryna_layout::TypeCategory::Bool]);
        let expression_ids = shape.children.iter().map(|(_, id)| *id).collect::<Vec<_>>();
        assert_eq!(expression_ids, [1, 0], "declaration order differs from source spelling");
        assert!(lowerer.instructions.is_empty());
        assert!(lowerer.places.is_empty());
        assert!(lowerer.owners.pending().is_empty());
        assert!(lowerer.cleanup_plans.is_empty());
        assert!(lowerer.constructor_storage_is_clear());
        assert_no_materialization(lowerer);
        assert_eq!(credits(lowerer), [0; 4]);
        let value = lowerer.value(expression, result).expect("materialized constructor");
        assert!(matches!(
            lowerer.instructions[0].kind,
            raw::InstructionKind::StringFromUtf8 { .. }
        ));
        assert!(matches!(lowerer.instructions[1].kind, raw::InstructionKind::BoolLiteral(true)));
        assert_eq!(value, raw::ValueId(2));
        assert_eq!((lowerer.instructions.len(), lowerer.places.len()), (3, 2));
        assert_eq!(lowerer.owners.pending(), [raw::PlaceId(1)]);
        assert_eq!(lowerer.owners.owner(value), Some(raw::PlaceId(1)));
        assert_eq!((lowerer.next_value, lowerer.aggregate_operands), (3, 2));
        assert_eq!((lowerer.cleanup_plans.len(), lowerer.cleanup_actions), (1, 0));
        assert_eq!(credits(lowerer), [0; 4]);
    });
    assert!(errors.is_empty());
}

#[test]
fn ordered_expression_decisions_preserve_later_failure_artifacts_and_earlier_cleanup_precedence() {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Pair);
    let expression = &mut snapshot.files[0].functions[0].body.expressions[0];
    let at = expression.span;
    source.replace_range(at.start as usize..at.end as usize, "lost");
    expression.kind = RawExpressionKind::Reference {
        name: zryna_syntax::v4::RawIdentifierSyntax { text: "lost".to_owned(), span: at },
    };
    for exhausted_cleanup in [false, true] {
        for _ in 0..2 {
            let mut expected = Vec::new();
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
                assert_no_materialization(lowerer);
                assert_eq!(credits(lowerer), [0; 4]);
                assert_eq!(lowerer.cleanup_actions, 0);
                let string_at =
                    span(lowerer.input.sources(), lowerer.function.body.expressions[1].span);
                expected.push(if exhausted_cleanup {
                    Diagnostic::error_at(
                        "ZRYNA-M3201",
                        string_at,
                        format!(
                            "derived cleanup actions exceed the per-function M3 limit of {}",
                            ir::MAX_DROP_ACTIONS_PER_FUNCTION
                        ),
                        "reduce simultaneously live owned aggregates and String leaves",
                    )
                } else {
                    Diagnostic::error_at(
                        "ZRYNA-M3002",
                        span(lowerer.input.sources(), at),
                        "aggregate value 'lost' is not declared",
                        "reference one exact preceding local using its declared spelling",
                    )
                });
                if exhausted_cleanup {
                    // Counter-only frontier, not a claim that this is complete valid raw IR.
                    lowerer.cleanup_actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION + 1;
                }
                assert!(lowerer.value(root_value(lowerer, 0), result).is_none());
                assert!(lowerer.constructor_storage_is_clear());
                assert_eq!(lowerer.reserved_transitions, 0);
                assert_eq!(lowerer.instructions.len(), usize::from(!exhausted_cleanup));
                assert_eq!(lowerer.places.len(), usize::from(!exhausted_cleanup));
                assert_eq!(lowerer.owners.pending().len(), usize::from(!exhausted_cleanup));
                assert_eq!(lowerer.cleanup_plans.len(), usize::from(!exhausted_cleanup));
                assert_eq!(lowerer.aggregate_operands, 0);
                assert_eq!(lowerer.next_value, u32::from(!exhausted_cleanup));
                assert_eq!(lowerer.next_local, 0);
                assert_eq!(
                    lowerer.cleanup_actions,
                    if exhausted_cleanup { ir::MAX_DROP_ACTIONS_PER_FUNCTION + 1 } else { 0 }
                );
                if exhausted_cleanup {
                    assert_no_materialization(lowerer);
                } else {
                    assert!(matches!(
                        lowerer.instructions[0].kind,
                        raw::InstructionKind::StringFromUtf8 { .. }
                    ));
                    assert_eq!(lowerer.instructions[0].span, string_at);
                    assert_eq!(lowerer.owners.pending(), [raw::PlaceId(0)]);
                    assert_eq!(lowerer.owners.owner(raw::ValueId(0)), Some(raw::PlaceId(0)));
                    assert_eq!(lowerer.cleanup_plans[0].span, string_at);
                    assert!(lowerer.cleanup_plans[0].actions.is_empty());
                }
            });
            assert_eq!(errors, expected, "exact ordered diagnostic including source authority");
            assert_eq!(errors.len(), 1);
            if exhausted_cleanup {
                assert_eq!(errors[0].code(), "ZRYNA-M3201");
            } else {
                assert_eq!(errors[0].code(), "ZRYNA-M3002");
                assert_eq!(errors[0].message(), "aggregate value 'lost' is not declared");
                assert_eq!(
                    errors[0].primary_span().map(|at| (at.start(), at.end())),
                    Some((at.start, at.end))
                );
            }
        }
    }
}

#[test]
fn ordered_expression_decisions_array_and_enum_types_precede_reservation_without_children() {
    for fixture in [Fixture::Array, Fixture::Enum, Fixture::EmptyEnum] {
        let mut expected = Vec::new();
        let errors = with_fixture(fixture, |lowerer, result| {
            assert_no_materialization(lowerer);
            assert_eq!(credits(lowerer), [0; 4]);
            let statement = usize::from(matches!(fixture, Fixture::Enum));
            let expression = root_value(lowerer, statement);
            let at = span(
                lowerer.input.sources(),
                lowerer.function.body.expressions[expression as usize].span,
            );
            let (message, guidance) = if matches!(fixture, Fixture::EmptyEnum) {
                (
                    format!(
                        "derived ownership transitions exceed the per-function M3 limit of {}",
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    ),
                    "reduce private aggregate expressions and assignments",
                )
            } else {
                (
                    format!(
                        "derived aggregate operands exceed the M3 limit of {}",
                        ir::MAX_AGGREGATE_OPERANDS
                    ),
                    "reduce Struct fields and fixed-array elements",
                )
            };
            expected.push(Diagnostic::error_at("ZRYNA-M3201", at, message, guidance));
            // Synthetic held counters pin diagnostic precedence, not a full raw-IR boundary.
            let held = LIMITS;
            set_credits(lowerer, held);
            let decision =
                decisions(lowerer).classify(expression, result).expect("complete outer shape");
            match decision.kind {
                ExpressionKind::Array(shape) => {
                    assert_eq!(shape.element.category, zryna_layout::TypeCategory::String);
                    assert_eq!(shape.elements.len(), 2);
                }
                ExpressionKind::Enum(shape) => {
                    if let Some((_, ty)) = shape.payload_input {
                        assert_eq!(ty.category, zryna_layout::TypeCategory::String);
                    }
                }
                _ => panic!("array or enum decision"),
            }
            assert_eq!(credits(lowerer), held);
            assert!(lowerer.instructions.is_empty());
            assert!(lowerer.cleanup_plans.is_empty());
            assert_no_materialization(lowerer);
            assert_eq!(lowerer.cleanup_actions, 0);
            assert!(
                lowerer.value(expression, result).is_none(),
                "materialization still reserves first"
            );
            assert_eq!(credits(lowerer), held);
            assert_no_materialization(lowerer);
            assert_eq!(lowerer.cleanup_actions, 0);
        });
        assert_eq!(errors, expected, "constructor-span capacity diagnostic precedes children");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M3201");
        let message = if matches!(fixture, Fixture::EmptyEnum) {
            format!(
                "derived ownership transitions exceed the per-function M3 limit of {}",
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            )
        } else {
            format!(
                "derived aggregate operands exceed the M3 limit of {}",
                ir::MAX_AGGREGATE_OPERANDS
            )
        };
        assert_eq!(errors[0].message(), message);
    }
}

#[test]
fn ordered_expression_decisions_defer_struct_child_type_lookup_until_requested() {
    let mut expected = Vec::new();
    let errors = with_fixture(Fixture::Nested, |lowerer, result| {
        assert_no_materialization(lowerer);
        assert_eq!(credits(lowerer), [0; 4]);
        let expression = root_value(lowerer, 0);
        let outer = lowerer
            .declarations
            .iter()
            .find(|decl| decl.name == "Outer")
            .expect("outer declaration");
        let mut context = decisions(lowerer);
        context.declarations = std::slice::from_ref(outer);
        let decision =
            context.classify(expression, result).expect("outer mapping does not resolve Inner");
        let ExpressionKind::Struct(shape) = decision.kind else { panic!("struct") };
        assert_eq!(shape.children.len(), 2);
        let syntax = &context.file.type_syntax()[shape.children[0].0 as usize];
        let zryna_syntax::v4::RawTypeSyntaxKind::Named { name } = &syntax.kind else {
            panic!("fixture's first declared child type is Inner")
        };
        assert_eq!(name.text, "Inner");
        expected.push(Diagnostic::error_at(
            "ZRYNA-M3002",
            span(context.input.sources(), name.span),
            "type 'Inner' does not name a module-local aggregate",
            "use bool, i32, or an exact aggregate declaration name",
        ));
        assert!(context.errors.is_empty(), "outer mapping alone emits no diagnostic");
        assert!(context.child_type(shape.children[0].0).is_none());
        assert!(lowerer.instructions.is_empty());
        assert!(lowerer.places.is_empty());
        assert!(lowerer.constructor_storage_is_clear());
        assert_no_materialization(lowerer);
        assert_eq!(credits(lowerer), [0; 4]);
        assert_eq!(lowerer.cleanup_actions, 0);
    });
    assert_eq!(errors, expected, "exact unresolved child type name span, not constructor span");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3002");
}
