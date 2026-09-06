#[derive(Debug, Eq, PartialEq)]
struct MatrixRow {
    requirement: String,
    source: Vec<String>,
    ir: Vec<String>,
}

fn evidence(cell: &str) -> Vec<String> {
    cell.split("; ")
        .map(|entry| {
            entry
                .strip_prefix('`')
                .and_then(|entry| entry.strip_suffix('`'))
                .unwrap_or_else(|| panic!("non-canonical evidence cell: {cell}"))
                .to_owned()
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

fn assert_test_inventory(corpus: &str, names: &[String], authority: &str) {
    for name in names {
        let declaration = format!("fn {name}()");
        assert_eq!(
            corpus.match_indices(&declaration).count(),
            1,
            "{authority} evidence must resolve exactly once: {name}"
        );
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
            source: ["structured_match_exhausts_payloadless_and_mixed_payload_variants", "structured_match_arm_order_is_canonical_and_scrutinee_is_consumed_once", "structured_match_invalid_arm_sets_reject_exactly_replay_and_recover"].map(str::to_owned).into(),
            ir: ["nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup", "nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Owned aggregate and container results through nested continuations".into(),
            source: ["structured_match_owned_payloads_join_one_result_and_continue", "structured_nested_matches_transfer_only_each_active_payload"].map(str::to_owned).into(),
            ir: ["generic_enum_payload_move_transfers_complete_owned_result_and_masks_only_active_subtree", "generic_enum_payload_move_old_enum_drop_precedes_complete_result_edge_transfer"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Constructor, call, and Vec continuation composition and failure cleanup".into(),
            source: ["structured_match_constructor_retains_earlier_operand_across_continuation", "structured_match_call_retains_arguments_until_complete_then_transfers_before_trap", "structured_match_vec_growth_cleanup_retains_both_completed_operands"].map(str::to_owned).into(),
            ir: ["nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Copy payload continuation restores one once-evaluated scrutinee".into(),
            source: ["structured_match_copy_payload_continuation_retains_surrounding_owned_parameter"].map(str::to_owned).into(),
            ir: ["copy_enum_join_restores_private_temporary_with_original_once_evaluated_ssa", "copy_enum_join_omitting_restoration_rejects_unequal_arm_refinements", "copy_enum_join_write_requires_its_active_exclusive_authority_and_exact_type"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Refinement dominance and inactive or foreign payload rejection".into(),
            source: ["structured_match_bad_arm_values_reject_and_replay_exact_diagnostics"].map(str::to_owned).into(),
            ir: ["nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload", "generic_enum_payload_move_rejects_wrong_variant_and_absent_refinement"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Moved, partial, repeated, cross-arm, join, and cleanup forgery rejection".into(),
            source: ["structured_nested_matches_transfer_only_each_active_payload"].map(str::to_owned).into(),
            ir: ["nested_match_rejects_moved_partial_repeated_and_cross_arm_ownership", "nested_match_rejects_incompatible_join_and_active_or_rest_cleanup_forgery", "generic_enum_payload_move_rejects_forged_paths_types_and_cross_arm_result_use", "generic_enum_payload_move_cleanup_is_exact_ordered_and_excludes_transferred_owner"].map(str::to_owned).into(),
        },
        MatrixRow {
            requirement: "Exact and first-extra graph, value, place, transition, and cleanup resources with overflow recovery".into(),
            source: ["complete_enum_matches_bound_graph_resources_atomically_and_recover", "complete_enum_match_value_place_transition_and_cleanup_resources_are_exact"].map(str::to_owned).into(),
            ir: ["generic_enum_payload_move_cleanup_preflight_exact_extra_and_checked_overflow_recover", "copy_enum_join_restoration_transition_preflight_exact_extra_and_overflow"].map(str::to_owned).into(),
        },
    ];
    let actual = rows(matrix);
    assert_eq!(actual, expected);

    let source = [
        include_str!("structured_match_complete_source.rs"),
        include_str!("structured_match_source.rs"),
        include_str!("structured_graph_resources.rs"),
        include_str!("structured_match_resources.rs"),
    ]
    .join("\n");
    let ir = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/structured_enum_match_hostile.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/generic_enum_payload_move.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/generic_enum_payload_move/hostile.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/generic_enum_payload_move/transfer.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/generic_enum_payload_move/resources.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/copy_enum_join.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/tests/copy_enum_join/hostile.rs"
        )),
    ]
    .join("\n");
    for row in &actual {
        assert_test_inventory(&source, &row.source, "source");
        assert_test_inventory(&ir, &row.ir, "IR");
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
            "- Parent remaining: #269",
        ]
    );
}
