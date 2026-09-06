#[derive(Debug, Eq, PartialEq)]
struct MatrixRow {
    requirement: String,
    source: Vec<Evidence>,
    ir: Vec<Evidence>,
}

#[derive(Debug, Eq, PartialEq)]
struct Evidence {
    file: String,
    test: String,
}

fn item(file: &str, test: &str) -> Evidence {
    Evidence { file: file.to_owned(), test: test.to_owned() }
}

fn evidence(cell: &str) -> Vec<Evidence> {
    cell.split("; ")
        .map(|entry| {
            let entry = entry
                .strip_prefix('`')
                .and_then(|entry| entry.strip_suffix('`'))
                .unwrap_or_else(|| panic!("non-canonical evidence cell: {cell}"));
            let (file, test) =
                entry.split_once("::").unwrap_or_else(|| panic!("missing file binding: {entry}"));
            item(file, test)
        })
        .collect()
}

fn rows(matrix: &str) -> Vec<MatrixRow> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let header = lines
        .iter()
        .position(|line| *line == "| Requirement | Source evidence | Independent IR evidence |")
        .expect("canonical matrix header");
    assert_eq!(lines[header + 1], "| --- | --- | --- |");
    lines[header + 2..]
        .iter()
        .take_while(|line| line.starts_with("| "))
        .map(|line| {
            let cells = line
                .strip_prefix("| ")
                .and_then(|line| line.strip_suffix(" |"))
                .expect("canonical table delimiters")
                .split(" | ")
                .collect::<Vec<_>>();
            assert_eq!(cells.len(), 3, "canonical three-column matrix row");
            MatrixRow {
                requirement: cells[0].to_owned(),
                source: evidence(cells[1]),
                ir: evidence(cells[2]),
            }
        })
        .collect()
}

fn retained_boundaries(matrix: &str) -> Vec<&str> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let heading = lines
        .iter()
        .position(|line| *line == "## Retained boundaries")
        .expect("retained-boundary heading");
    lines[heading + 2..]
        .iter()
        .take_while(|line| line.starts_with("- "))
        .map(|line| line.trim_start_matches("- "))
        .collect()
}

fn closure_status(matrix: &str) -> Vec<&str> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let heading =
        lines.iter().position(|line| *line == "## Closure status").expect("closure-status heading");
    lines[heading + 2..].iter().take_while(|line| line.starts_with("- ")).copied().collect()
}

fn assert_enabled_test(source: &str, evidence: &Evidence, authority: &str) {
    let declaration = format!("fn {}()", evidence.test);
    let lines = source.lines().collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(&declaration))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "{authority} evidence must resolve exactly once: {evidence:?}");
    let (line, _) = matches[0];
    let attributes = lines[..line].iter().rev().take_while(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with("#[")
    });
    let attributes = attributes.filter(|line| !line.trim().is_empty()).collect::<Vec<_>>();
    assert!(
        attributes.iter().any(|line| line.trim() == "#[test]"),
        "enabled #[test] required: {evidence:?}"
    );
    assert!(
        attributes.iter().all(|line| !line.trim().starts_with("#[ignore")),
        "ignored evidence is forbidden: {evidence:?}"
    );
}

fn source_file(path: &str) -> &'static str {
    match path {
        "structured_match_complete_source.rs" => {
            include_str!("structured_match_complete_source.rs")
        }
        "structured_match_source.rs" => include_str!("structured_match_source.rs"),
        "structured_string_source.rs" => include_str!("structured_string_source.rs"),
        "structured_indexed_source.rs" => include_str!("structured_indexed_source.rs"),
        "structured_formal_source.rs" => include_str!("structured_formal_source.rs"),
        "structured_graph_resources.rs" => include_str!("structured_graph_resources.rs"),
        "structured_match_resources.rs" => include_str!("structured_match_resources.rs"),
        _ => panic!("unbound source evidence file: {path}"),
    }
}

macro_rules! ir_file {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/",
            $path
        ))
    };
}

fn independent_ir_file(path: &str) -> &'static str {
    match path {
        "structured_enum_match_hostile.rs" => ir_file!("structured_enum_match_hostile.rs"),
        "generic_enum_payload_move.rs" => ir_file!("generic_enum_payload_move.rs"),
        "generic_enum_payload_move/transfer.rs" => {
            ir_file!("generic_enum_payload_move/transfer.rs")
        }
        "generic_enum_payload_move/hostile.rs" => ir_file!("generic_enum_payload_move/hostile.rs"),
        "generic_enum_payload_move/resources.rs" => {
            ir_file!("generic_enum_payload_move/resources.rs")
        }
        "mixed_constructor_authority.rs" => ir_file!("mixed_constructor_authority.rs"),
        "named_import_function_ids/owned_call_closure.rs" => {
            ir_file!("named_import_function_ids/owned_call_closure.rs")
        }
        "generic_recursive_composition.rs" => ir_file!("generic_recursive_composition.rs"),
        "indexed_vec_projection.rs" => ir_file!("indexed_vec_projection.rs"),
        "handle_aware_clone.rs" => ir_file!("handle_aware_clone.rs"),
        "copy_enum_join.rs" => ir_file!("copy_enum_join.rs"),
        "copy_enum_join/hostile.rs" => ir_file!("copy_enum_join/hostile.rs"),
        _ => panic!("unbound independent IR evidence file: {path}"),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn complete_enum_matching_matrix_binds_exact_test_inventory_and_boundaries() {
    let matrix = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/M3_COMPLETE_ENUM_MATCHING_MATRIX.md"
    ));
    let expected = [
        MatrixRow {
            requirement: "Exhaustive variants, canonical ordinals, one scrutinee evaluation, and active binding".into(),
            source: vec![
                item("structured_match_complete_source.rs", "structured_match_exhausts_payloadless_and_mixed_payload_variants"),
                item("structured_match_complete_source.rs", "structured_match_arm_order_is_canonical_and_scrutinee_is_consumed_once"),
                item("structured_match_complete_source.rs", "structured_match_invalid_arm_sets_reject_exactly_replay_and_recover"),
            ],
            ir: vec![
                item("structured_enum_match_hostile.rs", "nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup"),
                item("structured_enum_match_hostile.rs", "nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload"),
            ],
        },
        MatrixRow {
            requirement: "Owned aggregate and container results, nested transfer, and fallible preparation cleanup".into(),
            source: vec![
                item("structured_match_source.rs", "structured_match_owned_payloads_join_one_result_and_continue"),
                item("structured_match_complete_source.rs", "structured_nested_matches_transfer_only_each_active_payload"),
                item("structured_match_complete_source.rs", "nested_complete_match_fallible_payload_preparation_retains_prior_owners_and_recovers"),
            ],
            ir: vec![
                item("generic_enum_payload_move.rs", "generic_enum_payload_move_transfers_complete_owned_result_and_masks_only_active_subtree"),
                item("generic_enum_payload_move/transfer.rs", "generic_enum_payload_move_old_enum_drop_precedes_complete_result_edge_transfer"),
            ],
        },
        MatrixRow {
            requirement: "Constructor, call, and Vec continuation composition and failure cleanup".into(),
            source: vec![
                item("structured_match_source.rs", "structured_match_constructor_retains_earlier_operand_across_continuation"),
                item("structured_match_source.rs", "structured_match_call_retains_arguments_until_complete_then_transfers_before_trap"),
                item("structured_match_source.rs", "structured_match_vec_growth_cleanup_retains_both_completed_operands"),
            ],
            ir: vec![
                item("mixed_constructor_authority.rs", "mixed_raw_constructor_seed_verifies_exact_types_transfers_and_replay"),
                item("named_import_function_ids/owned_call_closure.rs", "cross_module_aggregate_container_and_handle_call_is_verified_independently"),
                item("generic_recursive_composition.rs", "recursive_vec_observation_replacement_and_push_reject_forged_authority"),
            ],
        },
        MatrixRow {
            requirement: "String, indexed, formal-authority, and direct-return contexts".into(),
            source: vec![
                item("structured_string_source.rs", "structured_string_reads_retain_exact_places_and_temporary_owners_across_match"),
                item("structured_indexed_source.rs", "structured_indexed_match_evaluates_once_before_the_only_bounds_site"),
                item("structured_formal_source.rs", "structured_formal_authority_is_forwarded_across_match_without_end_or_reborrow"),
                item("structured_match_complete_source.rs", "structured_nested_matches_transfer_only_each_active_payload"),
            ],
            ir: vec![
                item("structured_enum_match_hostile.rs", "nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup"),
                item("indexed_vec_projection.rs", "indexed_vec_projection_retains_bounds_region_and_exact_owned_replacement"),
                item("handle_aware_clone.rs", "formal_borrow_source_retains_caller_authority_without_inventing_a_local_root"),
            ],
        },
        MatrixRow {
            requirement: "Copy payload continuation restores one once-evaluated scrutinee".into(),
            source: vec![item("structured_match_source.rs", "structured_match_copy_payload_continuation_retains_surrounding_owned_parameter")],
            ir: vec![
                item("copy_enum_join.rs", "copy_enum_join_restores_private_temporary_with_original_once_evaluated_ssa"),
                item("copy_enum_join/hostile.rs", "copy_enum_join_omitting_restoration_rejects_unequal_arm_refinements"),
                item("copy_enum_join/hostile.rs", "copy_enum_join_write_requires_its_active_exclusive_authority_and_exact_type"),
            ],
        },
        MatrixRow {
            requirement: "Refinement dominance and inactive or foreign payload rejection".into(),
            source: vec![item("structured_match_source.rs", "structured_match_bad_arm_values_reject_and_replay_exact_diagnostics")],
            ir: vec![
                item("structured_enum_match_hostile.rs", "nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload"),
                item("generic_enum_payload_move/hostile.rs", "generic_enum_payload_move_rejects_wrong_variant_and_absent_refinement"),
            ],
        },
        MatrixRow {
            requirement: "Moved, partial, repeated, cross-arm, join, and cleanup forgery rejection".into(),
            source: vec![
                item("structured_match_complete_source.rs", "structured_nested_matches_transfer_only_each_active_payload"),
                item("structured_match_complete_source.rs", "nested_complete_match_fallible_payload_preparation_retains_prior_owners_and_recovers"),
            ],
            ir: vec![
                item("structured_enum_match_hostile.rs", "nested_match_rejects_moved_partial_repeated_and_cross_arm_ownership"),
                item("structured_enum_match_hostile.rs", "nested_match_rejects_incompatible_join_and_active_or_rest_cleanup_forgery"),
                item("generic_enum_payload_move/hostile.rs", "generic_enum_payload_move_rejects_forged_paths_types_and_cross_arm_result_use"),
                item("generic_enum_payload_move/hostile.rs", "generic_enum_payload_move_cleanup_is_exact_ordered_and_excludes_transferred_owner"),
            ],
        },
        MatrixRow {
            requirement: "Exact and first-extra graph, value, place, transition, and cleanup resources with overflow recovery".into(),
            source: vec![
                item("structured_graph_resources.rs", "complete_enum_matches_bound_graph_resources_atomically_and_recover"),
                item("structured_match_resources.rs", "complete_enum_match_value_place_transition_and_cleanup_resources_are_exact"),
            ],
            ir: vec![
                item("generic_enum_payload_move/resources.rs", "generic_enum_payload_move_cleanup_preflight_exact_extra_and_checked_overflow_recover"),
                item("copy_enum_join/hostile.rs", "copy_enum_join_restoration_transition_preflight_exact_extra_and_overflow"),
            ],
        },
    ];
    let actual = rows(matrix);
    assert_eq!(actual, expected);

    for row in &actual {
        for evidence in &row.source {
            assert_enabled_test(source_file(&evidence.file), evidence, "source");
        }
        for evidence in &row.ir {
            assert_enabled_test(independent_ir_file(&evidence.file), evidence, "IR");
        }
    }

    assert_eq!(
        retained_boundaries(matrix),
        [
            "terminating match arms",
            "source-selected discriminants and wildcard arms",
            "implicit clone and inactive payload access",
            "stored or returned borrows and borrow-carrying CFG edges",
            "Issue #275 non-indexed active-payload borrowing",
            "runtime, backend, driver, CLI, artifacts, public ABI, and public profile activation",
        ]
    );
    assert_eq!(
        closure_status(matrix),
        [
            "- Issue: #273",
            "- Integrated children: #333, #334, #335",
            "- Status: compiler-only closure candidate",
            "- Parent integration: completed by #269 after all source/IR children",
        ]
    );
}
