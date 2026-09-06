use super::super::{MAX_CALL_EDGES, preflight_codes};
use super::has;
use crate::data_ownership_v1::{Errors, checked_add, raw, verify};
use zryna_source::SourceMap;

mod fixture;
use fixture::program as cross_module_mixed_owned_call;

fn call_parts(program: &mut raw::Program) -> (&mut raw::FunctionId, &mut Vec<raw::CallArgument>) {
    let raw::InstructionKind::DirectCall { callee, arguments, .. } =
        &mut program.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    (callee, arguments)
}

#[test]
fn cross_module_aggregate_container_and_handle_call_is_verified_independently() {
    let (sources, linear, linux, program) = cross_module_mixed_owned_call();
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(program, &sources, entry, linear, linux)
        .expect("aggregate, container, and handle-bearing call");
    let modules = verified.modules().collect::<Vec<_>>();
    assert_eq!(modules.len(), 2);
    let call = modules[0]
        .functions()
        .next()
        .expect("caller")
        .blocks()
        .next()
        .expect("caller block")
        .instructions()
        .next()
        .expect("call");
    assert_eq!(call.callee().expect("sealed callee").module(), 1);
    assert_eq!(call.callee().expect("sealed callee").declaration(), 0);
    assert_eq!(call.call_arguments().count(), 6);
    assert!(call.derived_drop_actions().next().is_none());
}

fn assert_hostile_replay_and_recovery(
    mutation: raw::Program,
    expected_code: &str,
    genuine: &raw::Program,
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) {
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(mutation.clone(), sources, entry, linear.clone(), linux.clone())
        .expect_err("hostile cross-module owned call");
    assert!(has(&diagnostics, expected_code), "missing {expected_code}: {diagnostics:?}");
    assert_eq!(
        verify(mutation, sources, entry, linear.clone(), linux.clone())
            .expect_err("deterministic hostile replay"),
        diagnostics
    );
    verify(genuine.clone(), sources, entry, linear.clone(), linux.clone())
        .expect("same-instance recovery after hostile call");
}

#[test]
#[allow(clippy::too_many_lines)]
fn cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup() {
    let (sources, linear, linux, genuine) = cross_module_mixed_owned_call();

    let mut foreign_identity = genuine.clone();
    call_parts(&mut foreign_identity).0.module = raw::ModuleId(2);
    assert_hostile_replay_and_recovery(
        foreign_identity,
        "ZRYNA-I3009",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut wrong_arity = genuine.clone();
    call_parts(&mut wrong_arity).1.pop();
    for cleanup in &mut wrong_arity.modules[0].functions[0].cleanup_plans {
        cleanup.actions = vec![raw::DropAction::DropPlace(raw::PlaceId(5))];
    }
    assert_hostile_replay_and_recovery(
        wrong_arity,
        "ZRYNA-I3009",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut wrong_order = genuine.clone();
    call_parts(&mut wrong_order).1.swap(0, 1);
    assert_hostile_replay_and_recovery(
        wrong_order,
        "ZRYNA-I3009",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut wrong_result = genuine.clone();
    wrong_result.modules[0].functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("call result")
        .ty = raw::TypeId(4);
    wrong_result.modules[0].functions[0].places[6].ty = raw::TypeId(4);
    wrong_result.modules[0].functions[0].result = raw::TypeId(4);
    assert_hostile_replay_and_recovery(
        wrong_result,
        "ZRYNA-I3009",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut duplicate_owner = genuine.clone();
    call_parts(&mut duplicate_owner).1[5] = raw::CallArgument::Value(raw::ValueId(4));
    assert_hostile_replay_and_recovery(
        duplicate_owner,
        "ZRYNA-I3012",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut wrong_cleanup = genuine.clone();
    wrong_cleanup.modules[0].functions[0].cleanup_plans[0]
        .actions
        .push(raw::DropAction::DropPlace(raw::PlaceId(0)));
    assert_hostile_replay_and_recovery(
        wrong_cleanup,
        "ZRYNA-I3012",
        &genuine,
        &sources,
        &linear,
        &linux,
    );

    let mut partial_mask = genuine.clone();
    let caller = &mut partial_mask.modules[0].functions[0];
    let span = caller.span;
    caller.places[6] = raw::Place {
        id: raw::PlaceId(6),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
    };
    caller.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(6)),
    });
    caller.places.push(raw::Place {
        id: raw::PlaceId(8),
        ty: raw::TypeId(3),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(7)),
    });
    caller.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(6), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(6) },
        },
    );
    caller.blocks[0].instructions[1].result.as_mut().expect("call result").id = raw::ValueId(7);
    if let raw::Terminator::Return { value, .. } = &mut caller.blocks[0].terminators[0].kind {
        *value = raw::ValueId(7);
    }
    assert_hostile_replay_and_recovery(
        partial_mask,
        "ZRYNA-I3010",
        &genuine,
        &sources,
        &linear,
        &linux,
    );
}

#[test]
fn cross_module_call_resource_preflight_and_checked_overflow_recover() {
    let (sources, linear, linux, seed) = cross_module_mixed_owned_call();
    let mut call = seed.modules[0].functions[0].blocks[0].instructions[0].clone();
    call.result = None;
    let mut exact = seed.clone();
    exact.modules[0].functions[0].blocks[0].instructions = vec![call.clone(); MAX_CALL_EDGES];
    assert!(preflight_codes(&exact, &linear).is_empty());
    let mut extra = exact;
    extra.modules[0].functions[0].blocks[0].instructions.push(call);
    assert_eq!(preflight_codes(&extra, &linear), ["ZRYNA-I3201"]);
    assert_eq!(preflight_codes(&extra, &linear), ["ZRYNA-I3201"]);

    let mut overflow = Errors::default();
    assert_eq!(checked_add(usize::MAX, 1, "call edge count", &mut overflow), usize::MAX);
    let diagnostics = overflow.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3201");

    let entry = sources.verify_file_id(0).expect("entry");
    verify(seed, &sources, entry, linear, linux).expect("recovery after resource rejection");
}

#[test]
fn matrix_binding_is_complete_and_keeps_public_activation_excluded() {
    let matrix = include_str!("../../../../../../docs/M3_OWNED_CALL_CLOSURE_MATRIX.md");
    let source_evidence = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/imported_owned_signatures.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/generic_call_source.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/named_import_argument_order.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/named_import_boundaries.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/named_import_calls.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/named_import_resources.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-semantics/src/data_ownership_v1/tests/structured_payload_cfg.rs"
        )),
    ]
    .join("\n");
    for binding in [
        "named_import_alias_retains_canonical_cross_module_identity_and_owned_cleanup",
        "imported_producer_and_consumer_retain_one_foreign_nominal_identity",
        "generic_calls_transfer_multiple_owned_and_copy_arguments_in_source_order",
        "named_import_mixed_argument_producers_are_ordered_and_later_failure_retains_earlier_owners",
        "imported_signature_accepts_the_complete_sealed_by_value_graph",
        "payload_matrix_composes_handles_vec_calls_upgrade_and_nested_cfg",
        "generic_calls_reject_wrong_argument_type_and_repeated_owner_deterministically",
        "payload_cfg_fallible_operations_keep_source_ordered_cleanup",
        "named_import_graph_rejects_a_cycle_at_the_closing_edge_exactly",
        "named_import_target_still_obeys_the_direct_call_cycle_verifier",
        "named_import_preparation_resources_are_exact_atomic_overflow_checked_and_recoverable",
    ] {
        assert!(matrix.contains(binding), "missing source closure binding: {binding}");
        assert!(source_evidence.contains(binding), "stale source closure binding: {binding}");
    }
    let ir_evidence =
        [include_str!("owned_call_closure.rs"), include_str!("../named_import_function_ids.rs")]
            .join("\n");
    for binding in [
        "named_import_cross_module_function_ids_and_cleanup_are_verified_independently",
        "cross_module_aggregate_container_and_handle_call_is_verified_independently",
        "cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup",
        "named_import_two_module_function_ids_do_not_bypass_call_cycle_verification",
        "named_import_cross_module_static_depth_is_exact_and_first_extra_rejected",
        "cross_module_call_resource_preflight_and_checked_overflow_recover",
    ] {
        assert!(matrix.contains(binding), "missing IR closure binding: {binding}");
        assert!(ir_evidence.contains(binding), "stale IR closure binding: {binding}");
    }
    for exclusion in ["driver", "CLI", "backend", "runtime", "public ABI", "#273", "#275"] {
        assert!(matrix.contains(exclusion), "missing closure exclusion: {exclusion}");
    }
    assert!(matrix.contains("#329, #330, and #331 are integrated together"));
}
