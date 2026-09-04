use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::{
    self as ownership, tests::constructor_envelope_fixtures as fixtures,
};
use fixtures::Fixture;
use zryna_ir::data_ownership_v1::VerifiedInstructionKind;
use zryna_syntax::v4::{RawExpressionKind, RawProjectSyntaxSnapshot, verify_snapshot};
#[path = "constructor_preparation_control_fixtures.rs"]
mod control_fixtures;
#[path = "constructor_preparation_copy_prefix.rs"]
mod copy_prefix;
use control_fixtures::{copy_parameter, empty_nested, repeated_clone, zero_array};

fn replay(
    source: &str,
    snapshot: RawProjectSyntaxSnapshot,
    constructor: VerifiedInstructionKind,
    count: usize,
    parameters: usize,
    copies: usize,
    empty: bool,
) {
    let sources = fixtures::sources(source);
    let syntax =
        verify_snapshot(snapshot, &sources).expect("source-authenticated positive control");
    let mut previous = None;
    for _ in 0..2 {
        let program = ownership::lower(fixtures::input(&syntax, &sources))
            .expect("ordinary mandatory independent full IR verification");
        let functions = program
            .modules()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
            .collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].parameters().count(), parameters);
        let instructions = functions[0]
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .collect::<Vec<_>>();
        let kinds = instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>();
        assert_eq!(kinds.iter().filter(|kind| **kind == constructor).count(), count);
        assert_eq!(
            kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::CopyFromPlace).count(),
            copies
        );
        if empty {
            assert_eq!(
                kinds
                    .iter()
                    .filter(|kind| **kind == VerifiedInstructionKind::StringFromUtf8)
                    .count(),
                0
            );
            for instruction in
                instructions.iter().filter(|instruction| instruction.kind() == constructor)
            {
                assert_eq!(instruction.value_operands().count(), 0);
            }
        }
        if let Some(previous) = &previous {
            assert_eq!(&kinds, previous);
        }
        previous = Some(kinds);
    }
}

#[test]
fn constructor_preparation_controls_empty_array_and_payloadless_enum_verify_and_consume() {
    for ((source, snapshot), constructor) in [
        (zero_array(), VerifiedInstructionKind::FixedArrayConstruct),
        (fixtures::snapshot(Fixture::EmptyEnum), VerifiedInstructionKind::EnumConstruct),
    ] {
        replay(&source, snapshot.clone(), constructor, 1, 0, 0, true);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            let id = root_value(lowerer, 0);
            let before = lowerer.preparation_checkpoint();
            let prepared = PreparedValue::prepare(lowerer, id, ty).expect("zero-child plan");
            assert_eq!(prepared.lowerer.preparation_checkpoint(), before);
            assert_eq!(prepared.plan.visits, 1);
            assert_eq!(prepared.plan.steps.len(), 3);
            assert!(matches!(prepared.plan.steps[0].operation, Operation::Enter { arity: 0, .. }));
            assert!(matches!(prepared.plan.steps[1].operation, Operation::Release));
            let Operation::Commit { values, .. } = &prepared.plan.steps[2].operation else {
                panic!("empty commit")
            };
            assert!(values.is_empty());
            let result = prepared.plan.result;
            assert_eq!(prepared.consume(), result);
            assert!(lowerer.constructor_storage_is_clear());
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn constructor_preparation_controls_copy_parameter_reads_verify_with_dense_results() {
    for use_i32 in [false, true] {
        let (source, snapshot) = copy_parameter(use_i32);
        replay(&source, snapshot, VerifiedInstructionKind::StructConstruct, 1, 1, 1, false);
    }
}

#[test]
fn constructor_preparation_controls_empty_struct_rejected_at_raw_protocol_boundary() {
    let (source, snapshot) = empty_nested();
    let sources = fixtures::sources(&source);
    // Budget validation precedes source authentication; this is not a lowered-program proof.
    for _ in 0..2 {
        let errors = verify_snapshot(snapshot.clone(), &sources)
            .expect_err("zero-member declaration is outside the admitted raw protocol");
        assert_eq!(
            errors,
            vec![zryna_diagnostics::Diagnostic::error(
                "ZRYNA-F1401",
                None,
                "data member count overflow",
                "reduce the bounded protocol-v4 input",
            )]
        );
    }
}

#[test]
fn constructor_preparation_controls_classified_visits_equal_result_operations() {
    for ((source, snapshot), statement, visits, steps, constructor, count) in [
        (fixtures::snapshot(Fixture::Pair), 0, 3, 6, VerifiedInstructionKind::StructConstruct, 1),
        (
            fixtures::snapshot(Fixture::Nested),
            0,
            4,
            10,
            VerifiedInstructionKind::StructConstruct,
            2,
        ),
        (
            fixtures::snapshot(Fixture::Array),
            0,
            3,
            7,
            VerifiedInstructionKind::FixedArrayConstruct,
            1,
        ),
        (fixtures::snapshot(Fixture::Enum), 1, 2, 5, VerifiedInstructionKind::EnumConstruct, 1),
    ] {
        replay(&source, snapshot.clone(), constructor, count, 0, 0, false);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            for previous in 0..statement {
                assert!(run_statement(lowerer, previous, ty));
            }
            let id = root_value(lowerer, statement);
            let before = lowerer.preparation_checkpoint();
            let prepared = PreparedValue::prepare(lowerer, id, ty).expect("bounded plan");
            assert_eq!(prepared.lowerer.preparation_checkpoint(), before);
            assert_eq!(prepared.plan.visits, visits);
            assert_eq!(prepared.plan.steps.len(), steps);
            assert_eq!(
                prepared
                    .plan
                    .steps
                    .iter()
                    .filter(|step| matches!(
                        step.operation,
                        Operation::Leaf(_) | Operation::Commit { .. }
                    ))
                    .count(),
                visits
            );
            assert_eq!(
                prepared.plan.steps.iter().filter(|step| step.value.is_some()).count(),
                visits
            );
            let result = prepared.plan.result;
            assert_eq!(prepared.consume(), result);
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn constructor_preparation_controls_repeated_string_clone_creates_one_prefix_within_plan() {
    let (source, snapshot) = repeated_clone();
    replay(&source, snapshot.clone(), VerifiedInstructionKind::StructConstruct, 2, 0, 0, false);
    for precache in [false, true] {
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            assert!(lowerer.projections.is_empty(), "fixture initially has no cached prefix");
            let field = lowerer
                .function
                .body
                .expressions
                .iter()
                .position(|expression| {
                    matches!(expression.kind, RawExpressionKind::FieldAccess { .. })
                })
                .and_then(|id| u32::try_from(id).ok())
                .expect("first source-spelled p.first");
            let cached = if precache {
                let projection = lowerer.owned_place(field).expect("real prior canonical prefix");
                Some((projection.place, lowerer.places[projection.place.0 as usize].span))
            } else {
                None
            };
            let original_places = lowerer.places.clone();
            let original_projections = lowerer.projections.clone();
            let owners = lowerer.owners.clone();
            assert_eq!(owners.pending().len(), 1, "one genuine preceding Pair root");
            let before = lowerer.preparation_checkpoint();
            let id = root_value(lowerer, 1);
            let prepared =
                PreparedValue::prepare(lowerer, id, ty).expect("two String-clone children");
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
                    if let Operation::Prefix { id, .. } = &step.operation {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                prefixes.len(),
                usize::from(!precache),
                "one fresh canonical prefix or reuse the prior real cached prefix"
            );
            let canonical = if let Some((place, _)) = cached {
                place
            } else {
                assert_eq!(prefixes[0].0 as usize, original_places.len());
                prefixes[0]
            };
            let clones = prepared
                .plan
                .steps
                .iter()
                .filter_map(|step| {
                    if let Operation::Leaf(Leaf::StringClone { source, .. }) = &step.operation {
                        Some((source.place, step.value.expect("distinct clone result")))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(clones.len(), 2);
            assert_eq!(clones[0].0, canonical);
            assert_eq!(clones[1].0, canonical);
            assert_ne!(clones[0].1, clones[1].1);
            assert_eq!(prepared.plan.projections.len(), 1);
            let result = prepared.plan.result;
            assert_eq!(prepared.consume(), result);
            for owner in owners.pending() {
                assert!(lowerer.owners.contains(*owner));
            }
            assert!(lowerer.moved_projections.is_empty());
            assert!(lowerer.partial_roots.is_empty());
            assert_eq!(lowerer.owned_place(field).expect("same canonical prefix").place, canonical);
            if let Some((place, original_span)) = cached {
                assert_eq!(
                    lowerer.places[place.0 as usize].span, original_span,
                    "prior first source span survives both later planned uses"
                );
            }
        });
        assert!(errors.is_empty());
    }
}
