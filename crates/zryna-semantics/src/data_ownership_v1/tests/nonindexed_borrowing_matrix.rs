#[derive(Debug, Eq, PartialEq)]
struct Row {
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
    Evidence { file: file.into(), test: test.into() }
}

fn evidence(cell: &str) -> Vec<Evidence> {
    cell.split("; ")
        .map(|entry| {
            let entry = entry
                .strip_prefix('`')
                .and_then(|entry| entry.strip_suffix('`'))
                .unwrap_or_else(|| panic!("non-canonical evidence: {entry}"));
            let (file, test) = entry.split_once("::").expect("file-qualified evidence");
            item(file, test)
        })
        .collect()
}

fn rows(matrix: &str) -> Vec<Row> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let header = lines
        .iter()
        .position(|line| *line == "| Requirement | Source evidence | Independent IR evidence |")
        .expect("matrix header");
    assert_eq!(lines[header + 1], "| --- | --- | --- |");
    lines[header + 2..]
        .iter()
        .take_while(|line| line.starts_with("| "))
        .map(|line| {
            let cells = line
                .strip_prefix("| ")
                .and_then(|line| line.strip_suffix(" |"))
                .expect("table delimiters")
                .split(" | ")
                .collect::<Vec<_>>();
            assert_eq!(cells.len(), 3);
            Row { requirement: cells[0].into(), source: evidence(cells[1]), ir: evidence(cells[2]) }
        })
        .collect()
}

fn assert_enabled(source: &str, evidence: &Evidence) {
    let declaration = format!("fn {}()", evidence.test);
    let lines = source.lines().collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(&declaration))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "evidence must resolve exactly once: {evidence:?}");
    let attributes = lines[..matches[0].0]
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty() || line.trim().starts_with("#["))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert!(attributes.iter().any(|line| line.trim() == "#[test]"), "enabled test: {evidence:?}");
    assert!(
        attributes.iter().all(|line| !line.trim().starts_with("#[ignore")),
        "ignored evidence is forbidden: {evidence:?}"
    );
}

fn source_file(path: &str) -> &'static str {
    match path {
        "nonindexed_static_owned_borrow.rs" => include_str!("nonindexed_static_owned_borrow.rs"),
        "active_enum_payload_borrow_source.rs" => {
            include_str!("active_enum_payload_borrow_source.rs")
        }
        "enum_match_payload_borrow_source.rs" => {
            include_str!("enum_match_payload_borrow_source.rs")
        }
        "nonindexed_borrow_calls.rs" => include_str!("nonindexed_borrow_calls.rs"),
        "nonindexed_static_owned_borrow/calls.rs" => {
            include_str!("nonindexed_static_owned_borrow/calls.rs")
        }
        "active_enum_payload_borrow_fixture/calls.rs" => {
            include_str!("active_enum_payload_borrow_fixture/calls.rs")
        }
        "enum_match_payload_borrow_fixture/calls.rs" => {
            include_str!("enum_match_payload_borrow_fixture/calls.rs")
        }
        "nonindexed_borrow_resources.rs" => include_str!("nonindexed_borrow_resources.rs"),
        "nonindexed_borrow_resource_frontiers.rs" => {
            include_str!("nonindexed_borrow_resource_frontiers.rs")
        }
        "lexical_borrow_calls.rs" => include_str!("lexical_borrow_calls.rs"),
        "structured_graph_resources.rs" => include_str!("structured_graph_resources.rs"),
        _ => panic!("unbound source evidence: {path}"),
    }
}

macro_rules! ir {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/",
            $path
        ))
    };
}

fn ir_file(path: &str) -> &'static str {
    match path {
        "tests.rs" => ir!("tests.rs"),
        "nonindexed_owned_borrow_proof.rs" => ir!("tests/nonindexed_owned_borrow_proof.rs"),
        "active_enum_payload_borrow.rs" => ir!("tests/active_enum_payload_borrow.rs"),
        "enum_match_payload_borrow_ir.rs" => ir!("tests/enum_match_payload_borrow_ir.rs"),
        "borrow_nonindexed_call_scope.rs" => ir!("tests/borrow_nonindexed_call_scope.rs"),
        "nonindexed_projection_call_scope.rs" => {
            ir!("tests/nonindexed_projection_call_scope.rs")
        }
        "enum_match_payload_call_ir.rs" => ir!("tests/enum_match_payload_call_ir.rs"),
        "borrow_resource_boundaries.rs" => ir!("tests/borrow_resource_boundaries.rs"),
        _ => panic!("unbound IR evidence: {path}"),
    }
}

fn section_items<'a>(matrix: &'a str, heading: &str) -> Vec<&'a str> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| *line == heading).expect("section heading");
    lines[start + 2..].iter().take_while(|line| line.starts_with("- ")).copied().collect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn nonindexed_borrowing_matrix_binds_enabled_evidence_and_exact_boundaries() {
    let matrix = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/M3_NONINDEXED_OWNED_BORROWING_MATRIX.md"
    ));
    let expected = vec![
        Row {
            requirement: "Owned roots and static Struct/FixedArray places retain exact ownership, masks, overlap, clone, replacement, and recovery".into(),
            source: vec![
                item("nonindexed_static_owned_borrow.rs", "nonindexed_owned_struct_and_array_places_clone_replace_and_restore"),
                item("nonindexed_static_owned_borrow.rs", "nonindexed_owned_static_places_reject_partial_wrong_mode_and_wrong_type_then_recover"),
                item("nonindexed_static_owned_borrow.rs", "nonindexed_owned_parent_and_subobject_borrows_overlap_exactly"),
            ],
            ir: vec![
                item("nonindexed_owned_borrow_proof.rs", "nonindexed_owned_root_and_static_subobject_borrows_have_independent_ir_authority"),
                item("nonindexed_owned_borrow_proof.rs", "nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers"),
                item("nonindexed_owned_borrow_proof.rs", "nonindexed_owned_static_place_and_lifetime_forgery_replay_deterministically"),
                item("tests.rs", "direct_call_accepts_two_exclusive_disjoint_projected_authorities"),
                item("tests.rs", "projected_borrow_does_not_change_owner_masks_or_cleanup"),
            ],
        },
        Row {
            requirement: "Refined active enum payloads preserve exact variant, referent, parent cleanup, replacement, and lexical restoration".into(),
            source: vec![
                item("enum_match_payload_borrow_source.rs", "exhaustive_match_arms_borrow_only_the_refined_active_owned_payload"),
                item("enum_match_payload_borrow_source.rs", "inactive_match_payload_borrow_rejects_deterministically_then_recovers"),
                item("active_enum_payload_borrow_source.rs", "refined_active_enum_payload_shared_and_exclusive_borrows_restore_parent"),
                item("active_enum_payload_borrow_source.rs", "refined_enum_payload_borrow_rejects_inactive_and_foreign_variants_then_recovers"),
            ],
            ir: vec![
                item("enum_match_payload_borrow_ir.rs", "enum_match_arm_borrow_is_bound_to_exact_variant_region_and_cleanup"),
                item("enum_match_payload_borrow_ir.rs", "enum_match_payload_borrow_rejects_variant_nominal_region_and_cleanup_forgeries"),
                item("active_enum_payload_borrow.rs", "active_enum_payload_borrow_seals_refinement_mode_replacement_and_parent_cleanup"),
                item("active_enum_payload_borrow.rs", "enum_payload_borrow_rejects_inactive_foreign_type_and_mode_forgeries_then_recovers"),
                item("active_enum_payload_borrow.rs", "enum_payload_borrow_rejects_moved_overlap_disjoint_and_ended_authority_then_recovers"),
            ],
        },
        Row {
            requirement: "Nested lexical and direct-call use neither clones authority nor permits escape and restores the exact owner".into(),
            source: vec![
                item("nonindexed_borrow_calls.rs", "nested_nonindexed_lexical_call_preserves_authority_and_restores_owner"),
                item("nonindexed_borrow_calls.rs", "nested_nonindexed_call_does_not_clone_authority_or_fabricate_owned_results"),
                item("nonindexed_static_owned_borrow/calls.rs", "static_owned_borrows_pass_shared_and_exclusive_authority_to_direct_calls"),
                item("active_enum_payload_borrow_fixture/calls.rs", "refined_payload_borrows_pass_shared_and_exclusive_authority_to_direct_calls"),
                item("enum_match_payload_borrow_fixture/calls.rs", "exhaustive_match_payload_calls_preserve_shared_and_exclusive_arm_authority"),
                item("enum_match_payload_borrow_fixture/calls.rs", "exhaustive_match_payload_calls_reject_inactive_and_wrong_access_then_recover"),
            ],
            ir: vec![
                item("borrow_nonindexed_call_scope.rs", "nonindexed_lexical_call_is_verified_as_nonescaping_authority"),
                item("borrow_nonindexed_call_scope.rs", "nonindexed_call_rejects_inactive_wrong_region_and_escape_then_recovers"),
                item("nonindexed_projection_call_scope.rs", "static_struct_and_array_projection_calls_preserve_lexical_authority"),
                item("nonindexed_projection_call_scope.rs", "refined_enum_payload_calls_preserve_lexical_authority"),
                item("nonindexed_projection_call_scope.rs", "projection_calls_reject_wrong_region_and_repeated_exclusive_then_recover"),
                item("nonindexed_projection_call_scope.rs", "enum_payload_calls_reject_wrong_region_and_repeated_exclusive_then_recover"),
                item("enum_match_payload_call_ir.rs", "exhaustive_enum_match_arm_calls_preserve_exact_borrow_region_and_cleanup"),
                item("enum_match_payload_call_ir.rs", "exhaustive_enum_match_calls_reject_arm_access_region_and_cleanup_forgeries"),
            ],
        },
        Row {
            requirement: "Exact and first-extra resource dimensions, overflow, atomic rejection, and deterministic replay are checked".into(),
            source: vec![
                item("nonindexed_borrow_resource_frontiers.rs", "static_and_active_enum_borrow_lowering_hit_exact_transition_capacity_and_recover"),
                item("nonindexed_borrow_resource_frontiers.rs", "exhaustive_match_payload_borrow_clone_hits_exact_resource_costs_and_first_extra_recovers"),
                item("nonindexed_borrow_resource_frontiers.rs", "exhaustive_match_payload_borrow_clone_overflow_is_atomic_and_recovers"),
                item("nonindexed_borrow_resources.rs", "nonindexed_owned_borrow_resource_dimensions_accept_exact_and_reject_first_extra"),
                item("nonindexed_borrow_resources.rs", "nonindexed_owned_borrow_resource_overflow_is_checked_and_recovery_is_stable"),
                item("lexical_borrow_calls.rs", "borrow_call_resource_preflight_accepts_exact_limits_and_rejects_first_extra_in_order"),
                item("lexical_borrow_calls.rs", "borrow_call_resource_overflow_precedes_limit_selection_and_preserves_authority_cost"),
                item("structured_graph_resources.rs", "complete_enum_matches_bound_graph_resources_atomically_and_recover"),
            ],
            ir: vec![
                item("nonindexed_owned_borrow_proof.rs", "nonindexed_owned_borrow_places_reach_exact_limit_and_reject_first_extra"),
                item("borrow_resource_boundaries.rs", "dense_lexical_active_borrow_exact_and_first_extra_are_fully_verified"),
                item("nonindexed_owned_borrow_proof.rs", "nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers"),
                item("active_enum_payload_borrow.rs", "enum_payload_borrow_rejects_moved_overlap_disjoint_and_ended_authority_then_recovers"),
                item("borrow_nonindexed_call_scope.rs", "nonindexed_call_rejects_inactive_wrong_region_and_escape_then_recovers"),
            ],
        },
    ];
    let parsed = rows(matrix);
    assert_eq!(parsed, expected);
    let drifted = matrix.replace(
        "nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers",
        "nonindexed_owned_static_place_and_lifetime_forgery_replay_deterministically",
    );
    assert_ne!(
        rows(&drifted),
        expected,
        "an enabled but substituted test must not satisfy the matrix"
    );
    for row in &parsed {
        assert!(!row.source.is_empty() && !row.ir.is_empty());
        for evidence in &row.source {
            assert_enabled(source_file(&evidence.file), evidence);
        }
        for evidence in &row.ir {
            assert_enabled(ir_file(&evidence.file), evidence);
        }
    }
    assert_eq!(
        section_items(matrix, "## Retained boundaries"),
        [
            "- Dynamic-index FixedArray and Vec borrowing remains owned by #254–#256 and #274, not this matrix.",
            "- Stored, returned, or captured borrows and borrow-carrying branch or loop edges remain rejected.",
            "- Implicit lifetime shortening, arbitrary reborrowing, moves through live borrows, interior mutability, and raw pointers remain rejected.",
            "- Borrowed imports, nominal type-import grammar, indirect calls, callbacks, recursion, wildcard arms, and terminating match arms remain outside this boundary.",
            "- Runtime no-alias checks, backends, driver and CLI routes, artifacts, public ABI/profile activation, and target execution remain unavailable.",
        ]
    );
    assert_eq!(
        section_items(matrix, "## Closure status"),
        [
            "- #337 supplies owned-root and static-projection authority.",
            "- #338 supplies refined active-enum payload authority.",
            "- #339 supplies lexical and direct-call composition.",
            "- #340 supplies hostile IR, resource, documentation, and checked-matrix evidence.",
            "- #275 is the completed compiler-only child consumed by the #269 integration proof.",
        ]
    );
}
