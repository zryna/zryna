use super::{
    Errors, FunctionCatalog, FunctionSignature, MAX_SEMANTIC_DIAGNOSTICS, OwnedCfgBudgetLimit,
    OwnedCfgState, OwnedStringBranchState, OwnedStringEstimateContext,
    OwnedStringPreparationBudget, OwnerState, PartialTransferBudgetViolation, PrivateStringLowerer,
    ProgramCfgBudgetLimit, RootBorrowBudgetLimit, RootBorrowResources, SemanticInput,
    ValueBudgetLimit, accumulate_generated_cfg_function, accumulate_generated_value_function,
    aggregate_clone_budget_violation, aggregate_operand_budget_violation,
    aggregate_transition_budget_violation, authenticated_type_capabilities,
    checked_string_concat_bytes, cleanup_action_budget_violation, cleanup_actions_after_additions,
    cleanup_actions_after_preparation, cleanup_actions_after_transfer,
    conditional_root_borrow_budget_violation, conditional_root_borrow_resources,
    dense_owned_value_id, derived_value_count, enum_payload_move_resource_estimate,
    enum_payload_move_resource_violation, estimate_owned_string_expression,
    generated_cfg_budget_violation, is_direct_owned_root_borrow_candidate,
    is_terminal_owned_phi_candidate, loop_root_borrow_resources, lower,
    owned_call_cleanup_budget_violation, owned_cfg_budget_violation, owned_place_budget_violation,
    owned_root_borrow_resource_violation, owned_value_budget_violation,
    partial_assignment_budget_preflight, partial_assignment_place_delta,
    partial_return_budget_preflight, partial_return_place_delta, partial_transfer_budget_preflight,
    partial_transfer_place_delta, preflight_aggregate_operand_total, preflight_owned_loop_body,
    preflight_owned_loop_exit, preflight_owned_place_capacity,
    preflight_owned_place_capacity_with_reserved, preflight_owned_string_preparation,
    projected_aggregate_assignment_budget_violation,
    projected_aggregate_clone_assignment_budget_violation,
    projected_aggregate_clone_budget_violation, projected_root_borrow_resource_counts,
    projected_string_clone_budget_violation, projected_subobject_assignment_budget_violation,
    projected_subobject_move_budget_violation, projected_subobject_return_budget_violation,
    raw_function_value_count, raw_terminator_edge_count, resource_budget_violation,
    root_borrow_resource_violation, semantic_preflight, span,
    straight_root_borrow_budget_violation, string_byte_budget_violation, terminal_owned_if,
    value_budget_violation, vec_push_target_invalid,
};
use zryna_ir::data_ownership_v1::{
    PlaceIdentity as FaultPlaceIdentity, ValueIdentity as FaultValueIdentity,
    VerifiedActiveVariant, VerifiedBorrowAccess, VerifiedCleanupRole, VerifiedDropActionKind,
    VerifiedFunction, VerifiedInstruction as FaultVerifiedInstruction, VerifiedInstructionKind,
    VerifiedPlaceKind, VerifiedTerminatorKind, VerifiedTrapIdentity, raw,
};
use zryna_ownership_runtime_abi::{
    LogicalOperation, MAX_VEC_ELEMENTS, RuntimeStatus, VerifiedOwnershipRuntimeAbi,
    VerifiedStatusDisposition, VerifiedStatusTrapIdentity, operation_accepts_status,
    validate_failure_atomic_transition,
};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap, Span as FaultSpan};
use zryna_syntax::v4::{
    PROTOCOL_VERSION, RawBlockSyntax, RawDataDeclaration, RawDataDeclarationKind, RawDataField,
    RawExpressionSyntax, RawFunctionBodySyntax, RawFunctionSyntax, RawIdentifierSyntax,
    RawParameterSyntax, RawProjectSyntaxSnapshot, RawSourceUnit, RawStatementKind,
    RawStatementSyntax, RawTypeSyntax, RawTypeSyntaxKind, decode_snapshot, verify_snapshot,
};

mod aggregate_contracts;
mod aggregate_fixture_support;
mod aggregate_name_resolution;
mod cfg_control_flow_budgets;
mod cfg_owner_state;
mod cfg_validation;
mod conditional_root_borrows;
mod copy_calls;
mod derived_value_budgets;
mod enum_payloads;
mod enum_validation;
mod expression_preflight;
mod fixed_arrays;
mod generation_budgets;
mod loop_root_borrows;
mod nominal_declaration_budget;
mod owned_root_borrow_reads;
mod pair_oracle;
mod projected_borrows;
mod straight_root_borrows;
mod string_assignment;
mod string_budgets;
mod string_call_core;
mod string_call_fixture_support;
mod string_call_validation;
mod string_core;
mod struct_validation;
mod termination_validation;
mod vec_assignment;
mod vec_call_fixture_support;
mod vec_calls;
mod vec_clone_budgets;
mod vec_clone_core;

use aggregate_fixture_support::{
    ARRAY_OOB_SOURCE, ARRAY_RESPONSE, ARRAY_VALID_SOURCE, ENUM_RESPONSE, ENUM_SOURCE,
};
use string_call_fixture_support::{
    private_nested_string_call_fixture, private_string_call_fixture,
};
use vec_call_fixture_support::{private_vec_call_fixture, private_vec_nested_string_call_fixture};

const PAIR_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/syntax-v4-shorthand.zry");
const PAIR_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/syntax-v4-shorthand.json");
const PAIR_SCORE_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/pair-score-v4.zry");
const PAIR_SCORE_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/pair-score-v4.json");
const PAIR_ORACLE: &str = include_str!("../../../../tests/m3-fixtures/pair-oracle-v1.json");
const SHARED_ROOT_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow.zry");
const SHARED_ROOT_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow.json");
const PROJECTED_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-borrow.zry");
const PROJECTED_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/projected-borrow.json");
const PROJECTED_BORROW_SHARED_OVERLAP_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-borrow-shared-overlap.zry");
const PROJECTED_BORROW_SHARED_OVERLAP_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/projected-borrow-shared-overlap.json");
const PROJECTED_BORROW_EXCLUSIONS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-borrow-exclusions.zry");
const PROJECTED_BORROW_EXCLUSIONS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/projected-borrow-exclusions.json");
const OWNED_ROOT_BORROW_READS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/owned-root-borrow-reads.zry");
const OWNED_ROOT_BORROW_READS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/owned-root-borrow-reads.json");
const OWNED_ROOT_BORROW_EXCLUSIONS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/owned-root-borrow-exclusions.zry");
const OWNED_ROOT_BORROW_EXCLUSIONS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/owned-root-borrow-exclusions.json");
const SHARED_ROOT_BORROW_REPLACE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-replace.zry");
const SHARED_ROOT_BORROW_REPLACE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-replace.json");
const SHARED_ROOT_BORROW_ESCAPE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-escape.zry");
const SHARED_ROOT_BORROW_ESCAPE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-escape.json");
const SHARED_ROOT_BORROW_BOOL_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-bool.zry");
const SHARED_ROOT_BORROW_BOOL_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-bool.json");
const SHARED_ROOT_OWNER_READ_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-owner-read.zry");
const SHARED_ROOT_OWNER_READ_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-owner-read.json");
const SHARED_ROOT_BORROW_MUTABLE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-mutable.zry");
const SHARED_ROOT_BORROW_MUTABLE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-mutable.json");
const SHARED_ROOT_BORROW_WRONG_REFERENT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-wrong-referent.zry");
const SHARED_ROOT_BORROW_WRONG_REFERENT_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-wrong-referent.json");
const SHARED_ROOT_BORROW_UNUSED_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-borrow-unused.zry");
const SHARED_ROOT_BORROW_UNUSED_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-borrow-unused.json");
const EXCLUSIVE_ROOT_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/exclusive-root-borrow.zry");
const EXCLUSIVE_ROOT_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/exclusive-root-borrow.json");
const EXCLUSIVE_ROOT_BORROW_BOOL_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/exclusive-root-borrow-bool.zry");
const EXCLUSIVE_ROOT_BORROW_BOOL_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/exclusive-root-borrow-bool.json");
const SHARED_ROOT_REBORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/shared-root-reborrow.zry");
const SHARED_ROOT_REBORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/shared-root-reborrow.json");
const BORROW_CONFLICT_SHARED_EXCLUSIVE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-conflict-shared-exclusive.zry");
const BORROW_CONFLICT_SHARED_EXCLUSIVE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-conflict-shared-exclusive.json");
const BORROW_CONFLICT_EXCLUSIVE_SHARED_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-conflict-exclusive-shared.zry");
const BORROW_CONFLICT_EXCLUSIVE_SHARED_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-conflict-exclusive-shared.json");
const BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-conflict-exclusive-exclusive.zry");
const BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-conflict-exclusive-exclusive.json");
const BORROW_REBORROW_MUT_FROM_SHARED_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-reborrow-mut-from-shared.zry");
const BORROW_REBORROW_MUT_FROM_SHARED_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-reborrow-mut-from-shared.json");
const BORROW_REBORROW_SHARED_FROM_EXCLUSIVE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-reborrow-shared-from-exclusive.zry");
const BORROW_REBORROW_SHARED_FROM_EXCLUSIVE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-reborrow-shared-from-exclusive.json");
const BORROW_SHARED_WRITE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-shared-write.zry");
const BORROW_SHARED_WRITE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-shared-write.json");
const BORROW_EXCLUSIVE_WRONG_WRITE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-exclusive-wrong-write.zry");
const BORROW_EXCLUSIVE_WRONG_WRITE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-exclusive-wrong-write.json");
const BORROW_EXCLUSIVE_OWNER_READ_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-exclusive-owner-read.zry");
const BORROW_EXCLUSIVE_OWNER_READ_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-exclusive-owner-read.json");
const BORROW_EXCLUSIVE_ROOT_WRITE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-exclusive-root-write.zry");
const BORROW_EXCLUSIVE_ROOT_WRITE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-exclusive-root-write.json");
const BORROW_EXCLUSIVE_IMMUTABLE_ROOT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-exclusive-immutable-root.zry");
const BORROW_EXCLUSIVE_IMMUTABLE_ROOT_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-exclusive-immutable-root.json");
const BORROW_EXCLUSIVE_NONREFERENCE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/borrow-exclusive-nonreference.zry");
const BORROW_EXCLUSIVE_NONREFERENCE_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/borrow-exclusive-nonreference.json");
const CONDITIONAL_ROOT_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow.zry");
const CONDITIONAL_ROOT_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow.json");
const CONDITIONAL_ROOT_BORROW_ONE_ARM_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-one-arm.zry");
const CONDITIONAL_ROOT_BORROW_ONE_ARM_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-one-arm.json");
const CONDITIONAL_ROOT_BORROW_EXCLUSIVE_ARMS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-exclusive-arms.zry");
const CONDITIONAL_ROOT_BORROW_EXCLUSIVE_ARMS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-exclusive-arms.json");
const CONDITIONAL_ROOT_BORROW_CONFLICT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-conflict.zry");
const CONDITIONAL_ROOT_BORROW_CONFLICT_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-conflict.json");
const CONDITIONAL_ROOT_BORROW_OWNER_READ_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-owner-read.zry");
const CONDITIONAL_ROOT_BORROW_OWNER_READ_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-owner-read.json");
const CONDITIONAL_ROOT_BORROW_ELSE_ONLY_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-else-only.zry");
const CONDITIONAL_ROOT_BORROW_ELSE_ONLY_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-else-only.json");
const CONDITIONAL_ROOT_BORROW_EXCLUSIONS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/conditional-root-borrow-exclusions.zry");
const CONDITIONAL_ROOT_BORROW_EXCLUSIONS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/conditional-root-borrow-exclusions.json");
const LOOP_ROOT_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/loop-root-borrow.zry");
const LOOP_ROOT_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/loop-root-borrow.json");
const LOOP_SHARED_ROOT_BORROW_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/loop-shared-root-borrow.zry");
const LOOP_SHARED_ROOT_BORROW_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/loop-shared-root-borrow.json");
const LOOP_ROOT_BORROW_EXCLUSIONS_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/loop-root-borrow-exclusions.zry");
const LOOP_ROOT_BORROW_EXCLUSIONS_JSON: &[u8] =
    include_bytes!("../../../../tests/m3-fixtures/loop-root-borrow-exclusions.json");
const STRING_SOURCE: &str = "function bad(): String { return \"x\"; }";
const STRING_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":22},"kind":{"kind":"string","keyword_span":{"file":0,"start":16,"end":22}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":38},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":23,"end":38},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":38},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0],"close_brace_span":{"file":0,"start":37,"end":38}}],"statements":[{"span":{"file":0,"start":25,"end":36},"kind":{"kind":"return","keyword_span":{"file":0,"start":25,"end":31},"value":0,"semicolon_span":{"file":0,"start":35,"end":36}}}],"expressions":[{"span":{"file":0,"start":32,"end":35},"kind":{"kind":"string-literal","spelling":"\"x\""}}]}}]}],"diagnostics":[]}}"#;
const MULTIBYTE_STRING_SOURCE: &str = "function snow(): String { return \"snowman: ☃\"; }";
const MULTIBYTE_STRING_RESPONSE: &str = r#"{"id":3,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":17,"end":23},"kind":{"kind":"string","keyword_span":{"file":0,"start":17,"end":23}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":50},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"snow","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":24,"end":50},"root_block":0,"blocks":[{"span":{"file":0,"start":24,"end":50},"open_brace_span":{"file":0,"start":24,"end":25},"statements":[0],"close_brace_span":{"file":0,"start":49,"end":50}}],"statements":[{"span":{"file":0,"start":26,"end":48},"kind":{"kind":"return","keyword_span":{"file":0,"start":26,"end":32},"value":0,"semicolon_span":{"file":0,"start":47,"end":48}}}],"expressions":[{"span":{"file":0,"start":33,"end":47},"kind":{"kind":"string-literal","spelling":"\"snowman: ☃\""}}]}}]}],"diagnostics":[]}}"#;
const LOCAL_STRING_SOURCE: &str =
    "function take(): String { const value: String = \"hello\"; return value; }";
const LOCAL_STRING_RESPONSE: &str = r#"{"id":10,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":17,"end":23},"kind":{"kind":"string","keyword_span":{"file":0,"start":17,"end":23}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":72},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"take","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":24,"end":72},"root_block":0,"blocks":[{"span":{"file":0,"start":24,"end":72},"open_brace_span":{"file":0,"start":24,"end":25},"statements":[0,1],"close_brace_span":{"file":0,"start":71,"end":72}}],"statements":[{"span":{"file":0,"start":26,"end":56},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":26,"end":31},"mutable":false,"name":{"text":"value","span":{"file":0,"start":32,"end":37}},"type_syntax":1,"equals_span":{"file":0,"start":46,"end":47},"initializer":0,"semicolon_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":57,"end":70},"kind":{"kind":"return","keyword_span":{"file":0,"start":57,"end":63},"value":1,"semicolon_span":{"file":0,"start":69,"end":70}}}],"expressions":[{"span":{"file":0,"start":48,"end":55},"kind":{"kind":"string-literal","spelling":"\"hello\""}},{"span":{"file":0,"start":64,"end":69},"kind":{"kind":"reference","name":{"text":"value","span":{"file":0,"start":64,"end":69}}}}]}}]}],"diagnostics":[]}}"#;
const THREE_LOCAL_STRING_SOURCE: &str = "function take(): String { const first: String = \"a\"; const second: String = \"b\"; const result: String = \"c\"; return result; }";
const THREE_LOCAL_STRING_RESPONSE: &str = r#"{"id":11,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":17,"end":23},"kind":{"kind":"string","keyword_span":{"file":0,"start":17,"end":23}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}},{"span":{"file":0,"start":67,"end":73},"kind":{"kind":"string","keyword_span":{"file":0,"start":67,"end":73}}},{"span":{"file":0,"start":95,"end":101},"kind":{"kind":"string","keyword_span":{"file":0,"start":95,"end":101}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":125},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"take","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":24,"end":125},"root_block":0,"blocks":[{"span":{"file":0,"start":24,"end":125},"open_brace_span":{"file":0,"start":24,"end":25},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":124,"end":125}}],"statements":[{"span":{"file":0,"start":26,"end":52},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":26,"end":31},"mutable":false,"name":{"text":"first","span":{"file":0,"start":32,"end":37}},"type_syntax":1,"equals_span":{"file":0,"start":46,"end":47},"initializer":0,"semicolon_span":{"file":0,"start":51,"end":52}}},{"span":{"file":0,"start":53,"end":80},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":53,"end":58},"mutable":false,"name":{"text":"second","span":{"file":0,"start":59,"end":65}},"type_syntax":2,"equals_span":{"file":0,"start":74,"end":75},"initializer":1,"semicolon_span":{"file":0,"start":79,"end":80}}},{"span":{"file":0,"start":81,"end":108},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":81,"end":86},"mutable":false,"name":{"text":"result","span":{"file":0,"start":87,"end":93}},"type_syntax":3,"equals_span":{"file":0,"start":102,"end":103},"initializer":2,"semicolon_span":{"file":0,"start":107,"end":108}}},{"span":{"file":0,"start":109,"end":123},"kind":{"kind":"return","keyword_span":{"file":0,"start":109,"end":115},"value":3,"semicolon_span":{"file":0,"start":122,"end":123}}}],"expressions":[{"span":{"file":0,"start":48,"end":51},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":76,"end":79},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":104,"end":107},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":116,"end":122},"kind":{"kind":"reference","name":{"text":"result","span":{"file":0,"start":116,"end":122}}}}]}}]}],"diagnostics":[]}}"#;
const USE_AFTER_MOVE_SOURCE: &str = "function bad(): String { const first: String = \"a\"; const second: String = first; return first; }";
const USE_AFTER_MOVE_RESPONSE: &str = r#"{"id":12,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":22},"kind":{"kind":"string","keyword_span":{"file":0,"start":16,"end":22}}},{"span":{"file":0,"start":38,"end":44},"kind":{"kind":"string","keyword_span":{"file":0,"start":38,"end":44}}},{"span":{"file":0,"start":66,"end":72},"kind":{"kind":"string","keyword_span":{"file":0,"start":66,"end":72}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":97},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":23,"end":97},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":97},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0,1,2],"close_brace_span":{"file":0,"start":96,"end":97}}],"statements":[{"span":{"file":0,"start":25,"end":51},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":25,"end":30},"mutable":false,"name":{"text":"first","span":{"file":0,"start":31,"end":36}},"type_syntax":1,"equals_span":{"file":0,"start":45,"end":46},"initializer":0,"semicolon_span":{"file":0,"start":50,"end":51}}},{"span":{"file":0,"start":52,"end":81},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":52,"end":57},"mutable":false,"name":{"text":"second","span":{"file":0,"start":58,"end":64}},"type_syntax":2,"equals_span":{"file":0,"start":73,"end":74},"initializer":1,"semicolon_span":{"file":0,"start":80,"end":81}}},{"span":{"file":0,"start":82,"end":95},"kind":{"kind":"return","keyword_span":{"file":0,"start":82,"end":88},"value":2,"semicolon_span":{"file":0,"start":94,"end":95}}}],"expressions":[{"span":{"file":0,"start":47,"end":50},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":75,"end":80},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":75,"end":80}}}},{"span":{"file":0,"start":89,"end":94},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":89,"end":94}}}}]}}]}],"diagnostics":[]}}"#;
const STRING_ASSIGN_MOVE_SOURCE: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = y; return x; }";
const STRING_ASSIGN_MOVE_RESPONSE: &str = r#"{"id":203,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":19,"end":25},"kind":{"kind":"string","keyword_span":{"file":0,"start":19,"end":25}}},{"span":{"file":0,"start":35,"end":41},"kind":{"kind":"string","keyword_span":{"file":0,"start":35,"end":41}}},{"span":{"file":0,"start":60,"end":66},"kind":{"kind":"string","keyword_span":{"file":0,"start":60,"end":66}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":94},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"assign","span":{"file":0,"start":9,"end":15}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":26,"end":94},"root_block":0,"blocks":[{"span":{"file":0,"start":26,"end":94},"open_brace_span":{"file":0,"start":26,"end":27},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":93,"end":94}}],"statements":[{"span":{"file":0,"start":28,"end":50},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":28,"end":31},"mutable":true,"name":{"text":"x","span":{"file":0,"start":32,"end":33}},"type_syntax":1,"equals_span":{"file":0,"start":42,"end":43},"initializer":0,"semicolon_span":{"file":0,"start":49,"end":50}}},{"span":{"file":0,"start":51,"end":75},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":51,"end":56},"mutable":false,"name":{"text":"y","span":{"file":0,"start":57,"end":58}},"type_syntax":2,"equals_span":{"file":0,"start":67,"end":68},"initializer":1,"semicolon_span":{"file":0,"start":74,"end":75}}},{"span":{"file":0,"start":76,"end":82},"kind":{"kind":"assignment","target":2,"equals_span":{"file":0,"start":78,"end":79},"value":3,"semicolon_span":{"file":0,"start":81,"end":82}}},{"span":{"file":0,"start":83,"end":92},"kind":{"kind":"return","keyword_span":{"file":0,"start":83,"end":89},"value":4,"semicolon_span":{"file":0,"start":91,"end":92}}}],"expressions":[{"span":{"file":0,"start":44,"end":49},"kind":{"kind":"string-literal","spelling":"\"old\""}},{"span":{"file":0,"start":69,"end":74},"kind":{"kind":"string-literal","spelling":"\"new\""}},{"span":{"file":0,"start":76,"end":77},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":76,"end":77}}}},{"span":{"file":0,"start":80,"end":81},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":80,"end":81}}}},{"span":{"file":0,"start":90,"end":91},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":90,"end":91}}}}]}}]}],"diagnostics":[]}}"#;
const VEC_ASSIGN_STRING_SOURCE: &str = "function a(): Vec<String> { let x: Vec<String> = Vec<String>([\"a\"]); x = Vec<String>([\"b\"]); return x; }";
const VEC_ASSIGN_STRING_RESPONSE: &str = r#"{"id":210,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":18,"end":24},"kind":{"kind":"string","keyword_span":{"file":0,"start":18,"end":24}}},{"span":{"file":0,"start":14,"end":25},"kind":{"kind":"vec","keyword_span":{"file":0,"start":14,"end":17},"less_than_span":{"file":0,"start":17,"end":18},"argument":0,"greater_than_span":{"file":0,"start":24,"end":25}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}},{"span":{"file":0,"start":35,"end":46},"kind":{"kind":"vec","keyword_span":{"file":0,"start":35,"end":38},"less_than_span":{"file":0,"start":38,"end":39},"argument":2,"greater_than_span":{"file":0,"start":45,"end":46}}},{"span":{"file":0,"start":53,"end":59},"kind":{"kind":"string","keyword_span":{"file":0,"start":53,"end":59}}},{"span":{"file":0,"start":49,"end":60},"kind":{"kind":"vec","keyword_span":{"file":0,"start":49,"end":52},"less_than_span":{"file":0,"start":52,"end":53},"argument":4,"greater_than_span":{"file":0,"start":59,"end":60}}},{"span":{"file":0,"start":77,"end":83},"kind":{"kind":"string","keyword_span":{"file":0,"start":77,"end":83}}},{"span":{"file":0,"start":73,"end":84},"kind":{"kind":"vec","keyword_span":{"file":0,"start":73,"end":76},"less_than_span":{"file":0,"start":76,"end":77},"argument":6,"greater_than_span":{"file":0,"start":83,"end":84}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":104},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"a","span":{"file":0,"start":9,"end":10}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":26,"end":104},"root_block":0,"blocks":[{"span":{"file":0,"start":26,"end":104},"open_brace_span":{"file":0,"start":26,"end":27},"statements":[0,1,2],"close_brace_span":{"file":0,"start":103,"end":104}}],"statements":[{"span":{"file":0,"start":28,"end":68},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":28,"end":31},"mutable":true,"name":{"text":"x","span":{"file":0,"start":32,"end":33}},"type_syntax":3,"equals_span":{"file":0,"start":47,"end":48},"initializer":1,"semicolon_span":{"file":0,"start":67,"end":68}}},{"span":{"file":0,"start":69,"end":92},"kind":{"kind":"assignment","target":2,"equals_span":{"file":0,"start":71,"end":72},"value":4,"semicolon_span":{"file":0,"start":91,"end":92}}},{"span":{"file":0,"start":93,"end":102},"kind":{"kind":"return","keyword_span":{"file":0,"start":93,"end":99},"value":5,"semicolon_span":{"file":0,"start":101,"end":102}}}],"expressions":[{"span":{"file":0,"start":62,"end":65},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":49,"end":67},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":60,"end":61},"open_bracket_span":{"file":0,"start":61,"end":62},"elements":[0],"close_bracket_span":{"file":0,"start":65,"end":66},"close_paren_span":{"file":0,"start":66,"end":67}}},{"span":{"file":0,"start":69,"end":70},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":69,"end":70}}}},{"span":{"file":0,"start":86,"end":89},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":73,"end":91},"kind":{"kind":"vec-construction","type_syntax":7,"open_paren_span":{"file":0,"start":84,"end":85},"open_bracket_span":{"file":0,"start":85,"end":86},"elements":[3],"close_bracket_span":{"file":0,"start":89,"end":90},"close_paren_span":{"file":0,"start":90,"end":91}}},{"span":{"file":0,"start":100,"end":101},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":100,"end":101}}}}]}}]}],"diagnostics":[]}}"#;
const VEC_ASSIGN_I32_SOURCE: &str = "function a(): Vec<i32> { let x: Vec<i32> = Vec<i32>([]); const y: Vec<i32> = Vec<i32>([]); x = y; return x; }";
const VEC_ASSIGN_I32_RESPONSE: &str = r#"{"id":211,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":18,"end":21},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":18,"end":21}}}},{"span":{"file":0,"start":14,"end":22},"kind":{"kind":"vec","keyword_span":{"file":0,"start":14,"end":17},"less_than_span":{"file":0,"start":17,"end":18},"argument":0,"greater_than_span":{"file":0,"start":21,"end":22}}},{"span":{"file":0,"start":36,"end":39},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":36,"end":39}}}},{"span":{"file":0,"start":32,"end":40},"kind":{"kind":"vec","keyword_span":{"file":0,"start":32,"end":35},"less_than_span":{"file":0,"start":35,"end":36},"argument":2,"greater_than_span":{"file":0,"start":39,"end":40}}},{"span":{"file":0,"start":47,"end":50},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":47,"end":50}}}},{"span":{"file":0,"start":43,"end":51},"kind":{"kind":"vec","keyword_span":{"file":0,"start":43,"end":46},"less_than_span":{"file":0,"start":46,"end":47},"argument":4,"greater_than_span":{"file":0,"start":50,"end":51}}},{"span":{"file":0,"start":70,"end":73},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":70,"end":73}}}},{"span":{"file":0,"start":66,"end":74},"kind":{"kind":"vec","keyword_span":{"file":0,"start":66,"end":69},"less_than_span":{"file":0,"start":69,"end":70},"argument":6,"greater_than_span":{"file":0,"start":73,"end":74}}},{"span":{"file":0,"start":81,"end":84},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":81,"end":84}}}},{"span":{"file":0,"start":77,"end":85},"kind":{"kind":"vec","keyword_span":{"file":0,"start":77,"end":80},"less_than_span":{"file":0,"start":80,"end":81},"argument":8,"greater_than_span":{"file":0,"start":84,"end":85}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":109},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"a","span":{"file":0,"start":9,"end":10}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":23,"end":109},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":109},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":108,"end":109}}],"statements":[{"span":{"file":0,"start":25,"end":56},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":25,"end":28},"mutable":true,"name":{"text":"x","span":{"file":0,"start":29,"end":30}},"type_syntax":3,"equals_span":{"file":0,"start":41,"end":42},"initializer":0,"semicolon_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":57,"end":90},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":57,"end":62},"mutable":false,"name":{"text":"y","span":{"file":0,"start":63,"end":64}},"type_syntax":7,"equals_span":{"file":0,"start":75,"end":76},"initializer":1,"semicolon_span":{"file":0,"start":89,"end":90}}},{"span":{"file":0,"start":91,"end":97},"kind":{"kind":"assignment","target":2,"equals_span":{"file":0,"start":93,"end":94},"value":3,"semicolon_span":{"file":0,"start":96,"end":97}}},{"span":{"file":0,"start":98,"end":107},"kind":{"kind":"return","keyword_span":{"file":0,"start":98,"end":104},"value":4,"semicolon_span":{"file":0,"start":106,"end":107}}}],"expressions":[{"span":{"file":0,"start":43,"end":55},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":51,"end":52},"open_bracket_span":{"file":0,"start":52,"end":53},"elements":[],"close_bracket_span":{"file":0,"start":53,"end":54},"close_paren_span":{"file":0,"start":54,"end":55}}},{"span":{"file":0,"start":77,"end":89},"kind":{"kind":"vec-construction","type_syntax":9,"open_paren_span":{"file":0,"start":85,"end":86},"open_bracket_span":{"file":0,"start":86,"end":87},"elements":[],"close_bracket_span":{"file":0,"start":87,"end":88},"close_paren_span":{"file":0,"start":88,"end":89}}},{"span":{"file":0,"start":91,"end":92},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":91,"end":92}}}},{"span":{"file":0,"start":95,"end":96},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":95,"end":96}}}},{"span":{"file":0,"start":105,"end":106},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":105,"end":106}}}}]}}]}],"diagnostics":[]}}"#;
const STRING_CLONE_SOURCE: &str =
    "function cloneString(): String { const source: String = \"snow\"; return clone(source); }";
const STRING_CLONE_RESPONSE: &str = r#"{"id":20,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":24,"end":30},"kind":{"kind":"string","keyword_span":{"file":0,"start":24,"end":30}}},{"span":{"file":0,"start":47,"end":53},"kind":{"kind":"string","keyword_span":{"file":0,"start":47,"end":53}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":87},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"cloneString","span":{"file":0,"start":9,"end":20}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":31,"end":87},"root_block":0,"blocks":[{"span":{"file":0,"start":31,"end":87},"open_brace_span":{"file":0,"start":31,"end":32},"statements":[0,1],"close_brace_span":{"file":0,"start":86,"end":87}}],"statements":[{"span":{"file":0,"start":33,"end":63},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":33,"end":38},"mutable":false,"name":{"text":"source","span":{"file":0,"start":39,"end":45}},"type_syntax":1,"equals_span":{"file":0,"start":54,"end":55},"initializer":0,"semicolon_span":{"file":0,"start":62,"end":63}}},{"span":{"file":0,"start":64,"end":85},"kind":{"kind":"return","keyword_span":{"file":0,"start":64,"end":70},"value":2,"semicolon_span":{"file":0,"start":84,"end":85}}}],"expressions":[{"span":{"file":0,"start":56,"end":62},"kind":{"kind":"string-literal","spelling":"\"snow\""}},{"span":{"file":0,"start":77,"end":83},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":77,"end":83}}}},{"span":{"file":0,"start":71,"end":84},"kind":{"kind":"clone","keyword_span":{"file":0,"start":71,"end":76},"open_paren_span":{"file":0,"start":76,"end":77},"value":1,"close_paren_span":{"file":0,"start":83,"end":84}}}]}}]}],"diagnostics":[]}}"#;
const STRING_CONCAT_SOURCE: &str = "function join(): String { const left: String = \"ab\"; const right: String = \"cd\"; return concat(left, right); }";
const STRING_CONCAT_RESPONSE: &str = r#"{"id":21,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":17,"end":23},"kind":{"kind":"string","keyword_span":{"file":0,"start":17,"end":23}}},{"span":{"file":0,"start":38,"end":44},"kind":{"kind":"string","keyword_span":{"file":0,"start":38,"end":44}}},{"span":{"file":0,"start":66,"end":72},"kind":{"kind":"string","keyword_span":{"file":0,"start":66,"end":72}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":110},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"join","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":24,"end":110},"root_block":0,"blocks":[{"span":{"file":0,"start":24,"end":110},"open_brace_span":{"file":0,"start":24,"end":25},"statements":[0,1,2],"close_brace_span":{"file":0,"start":109,"end":110}}],"statements":[{"span":{"file":0,"start":26,"end":52},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":26,"end":31},"mutable":false,"name":{"text":"left","span":{"file":0,"start":32,"end":36}},"type_syntax":1,"equals_span":{"file":0,"start":45,"end":46},"initializer":0,"semicolon_span":{"file":0,"start":51,"end":52}}},{"span":{"file":0,"start":53,"end":80},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":53,"end":58},"mutable":false,"name":{"text":"right","span":{"file":0,"start":59,"end":64}},"type_syntax":2,"equals_span":{"file":0,"start":73,"end":74},"initializer":1,"semicolon_span":{"file":0,"start":79,"end":80}}},{"span":{"file":0,"start":81,"end":108},"kind":{"kind":"return","keyword_span":{"file":0,"start":81,"end":87},"value":4,"semicolon_span":{"file":0,"start":107,"end":108}}}],"expressions":[{"span":{"file":0,"start":47,"end":51},"kind":{"kind":"string-literal","spelling":"\"ab\""}},{"span":{"file":0,"start":75,"end":79},"kind":{"kind":"string-literal","spelling":"\"cd\""}},{"span":{"file":0,"start":95,"end":99},"kind":{"kind":"reference","name":{"text":"left","span":{"file":0,"start":95,"end":99}}}},{"span":{"file":0,"start":101,"end":106},"kind":{"kind":"reference","name":{"text":"right","span":{"file":0,"start":101,"end":106}}}},{"span":{"file":0,"start":88,"end":107},"kind":{"kind":"call","callee":{"text":"concat","span":{"file":0,"start":88,"end":94}},"open_paren_span":{"file":0,"start":94,"end":95},"arguments":[2,3],"close_paren_span":{"file":0,"start":106,"end":107}}}]}}]}],"diagnostics":[]}}"#;
const MOVED_STRING_CLONE_SOURCE: &str = "function bad(): String { const source: String = \"x\"; const moved: String = source; return clone(source); }";
const MOVED_STRING_CLONE_RESPONSE: &str = r#"{"id":22,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":22},"kind":{"kind":"string","keyword_span":{"file":0,"start":16,"end":22}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}},{"span":{"file":0,"start":66,"end":72},"kind":{"kind":"string","keyword_span":{"file":0,"start":66,"end":72}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":106},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":23,"end":106},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":106},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0,1,2],"close_brace_span":{"file":0,"start":105,"end":106}}],"statements":[{"span":{"file":0,"start":25,"end":52},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":25,"end":30},"mutable":false,"name":{"text":"source","span":{"file":0,"start":31,"end":37}},"type_syntax":1,"equals_span":{"file":0,"start":46,"end":47},"initializer":0,"semicolon_span":{"file":0,"start":51,"end":52}}},{"span":{"file":0,"start":53,"end":82},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":53,"end":58},"mutable":false,"name":{"text":"moved","span":{"file":0,"start":59,"end":64}},"type_syntax":2,"equals_span":{"file":0,"start":73,"end":74},"initializer":1,"semicolon_span":{"file":0,"start":81,"end":82}}},{"span":{"file":0,"start":83,"end":104},"kind":{"kind":"return","keyword_span":{"file":0,"start":83,"end":89},"value":3,"semicolon_span":{"file":0,"start":103,"end":104}}}],"expressions":[{"span":{"file":0,"start":48,"end":51},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":75,"end":81},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":75,"end":81}}}},{"span":{"file":0,"start":96,"end":102},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":96,"end":102}}}},{"span":{"file":0,"start":90,"end":103},"kind":{"kind":"clone","keyword_span":{"file":0,"start":90,"end":95},"open_paren_span":{"file":0,"start":95,"end":96},"value":2,"close_paren_span":{"file":0,"start":102,"end":103}}}]}}]}],"diagnostics":[]}}"#;
const BAD_STRING_CONCAT_SOURCE: &str =
    "function bad(): String { const source: String = \"x\"; return concat(source); }";
const BAD_STRING_CONCAT_RESPONSE: &str = r#"{"id":23,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":22},"kind":{"kind":"string","keyword_span":{"file":0,"start":16,"end":22}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":77},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":23,"end":77},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":77},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0,1],"close_brace_span":{"file":0,"start":76,"end":77}}],"statements":[{"span":{"file":0,"start":25,"end":52},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":25,"end":30},"mutable":false,"name":{"text":"source","span":{"file":0,"start":31,"end":37}},"type_syntax":1,"equals_span":{"file":0,"start":46,"end":47},"initializer":0,"semicolon_span":{"file":0,"start":51,"end":52}}},{"span":{"file":0,"start":53,"end":75},"kind":{"kind":"return","keyword_span":{"file":0,"start":53,"end":59},"value":2,"semicolon_span":{"file":0,"start":74,"end":75}}}],"expressions":[{"span":{"file":0,"start":48,"end":51},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":67,"end":73},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":67,"end":73}}}},{"span":{"file":0,"start":60,"end":74},"kind":{"kind":"call","callee":{"text":"concat","span":{"file":0,"start":60,"end":66}},"open_paren_span":{"file":0,"start":66,"end":67},"arguments":[1],"close_paren_span":{"file":0,"start":73,"end":74}}}]}}]}],"diagnostics":[]}}"#;
const VEC_STRING_SOURCE: &str = "function make(): Vec<String> { const first: String = \"a\"; const values: Vec<String> = Vec<String>([first, \"b\"]); return values; }";
const VEC_STRING_RESPONSE: &str = r#"{"id":30,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":21,"end":27},"kind":{"kind":"string","keyword_span":{"file":0,"start":21,"end":27}}},{"span":{"file":0,"start":17,"end":28},"kind":{"kind":"vec","keyword_span":{"file":0,"start":17,"end":20},"less_than_span":{"file":0,"start":20,"end":21},"argument":0,"greater_than_span":{"file":0,"start":27,"end":28}}},{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":76,"end":82},"kind":{"kind":"string","keyword_span":{"file":0,"start":76,"end":82}}},{"span":{"file":0,"start":72,"end":83},"kind":{"kind":"vec","keyword_span":{"file":0,"start":72,"end":75},"less_than_span":{"file":0,"start":75,"end":76},"argument":3,"greater_than_span":{"file":0,"start":82,"end":83}}},{"span":{"file":0,"start":90,"end":96},"kind":{"kind":"string","keyword_span":{"file":0,"start":90,"end":96}}},{"span":{"file":0,"start":86,"end":97},"kind":{"kind":"vec","keyword_span":{"file":0,"start":86,"end":89},"less_than_span":{"file":0,"start":89,"end":90},"argument":5,"greater_than_span":{"file":0,"start":96,"end":97}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":129},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":29,"end":129},"root_block":0,"blocks":[{"span":{"file":0,"start":29,"end":129},"open_brace_span":{"file":0,"start":29,"end":30},"statements":[0,1,2],"close_brace_span":{"file":0,"start":128,"end":129}}],"statements":[{"span":{"file":0,"start":31,"end":57},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":31,"end":36},"mutable":false,"name":{"text":"first","span":{"file":0,"start":37,"end":42}},"type_syntax":2,"equals_span":{"file":0,"start":51,"end":52},"initializer":0,"semicolon_span":{"file":0,"start":56,"end":57}}},{"span":{"file":0,"start":58,"end":112},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":58,"end":63},"mutable":false,"name":{"text":"values","span":{"file":0,"start":64,"end":70}},"type_syntax":4,"equals_span":{"file":0,"start":84,"end":85},"initializer":3,"semicolon_span":{"file":0,"start":111,"end":112}}},{"span":{"file":0,"start":113,"end":127},"kind":{"kind":"return","keyword_span":{"file":0,"start":113,"end":119},"value":4,"semicolon_span":{"file":0,"start":126,"end":127}}}],"expressions":[{"span":{"file":0,"start":53,"end":56},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":106,"end":109},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":86,"end":111},"kind":{"kind":"vec-construction","type_syntax":6,"open_paren_span":{"file":0,"start":97,"end":98},"open_bracket_span":{"file":0,"start":98,"end":99},"elements":[1,2],"close_bracket_span":{"file":0,"start":109,"end":110},"close_paren_span":{"file":0,"start":110,"end":111}}},{"span":{"file":0,"start":120,"end":126},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":120,"end":126}}}}]}}]}],"diagnostics":[]}}"#;
const EMPTY_VEC_SOURCE: &str = "function empty(): Vec<i32> { return Vec<i32>([]); }";
const EMPTY_VEC_RESPONSE: &str = r#"{"id":31,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":22,"end":25},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":22,"end":25}}}},{"span":{"file":0,"start":18,"end":26},"kind":{"kind":"vec","keyword_span":{"file":0,"start":18,"end":21},"less_than_span":{"file":0,"start":21,"end":22},"argument":0,"greater_than_span":{"file":0,"start":25,"end":26}}},{"span":{"file":0,"start":40,"end":43},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":40,"end":43}}}},{"span":{"file":0,"start":36,"end":44},"kind":{"kind":"vec","keyword_span":{"file":0,"start":36,"end":39},"less_than_span":{"file":0,"start":39,"end":40},"argument":2,"greater_than_span":{"file":0,"start":43,"end":44}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":51},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"empty","span":{"file":0,"start":9,"end":14}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":27,"end":51},"root_block":0,"blocks":[{"span":{"file":0,"start":27,"end":51},"open_brace_span":{"file":0,"start":27,"end":28},"statements":[0],"close_brace_span":{"file":0,"start":50,"end":51}}],"statements":[{"span":{"file":0,"start":29,"end":49},"kind":{"kind":"return","keyword_span":{"file":0,"start":29,"end":35},"value":0,"semicolon_span":{"file":0,"start":48,"end":49}}}],"expressions":[{"span":{"file":0,"start":36,"end":48},"kind":{"kind":"vec-construction","type_syntax":3,"open_paren_span":{"file":0,"start":44,"end":45},"open_bracket_span":{"file":0,"start":45,"end":46},"elements":[],"close_bracket_span":{"file":0,"start":46,"end":47},"close_paren_span":{"file":0,"start":47,"end":48}}}]}}]}],"diagnostics":[]}}"#;
const MOVED_VEC_ELEMENT_SOURCE: &str = "function bad(): Vec<String> { const first: String = \"a\"; return Vec<String>([first, first]); }";
const MOVED_VEC_ELEMENT_RESPONSE: &str = r#"{"id":32,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":20,"end":26},"kind":{"kind":"string","keyword_span":{"file":0,"start":20,"end":26}}},{"span":{"file":0,"start":16,"end":27},"kind":{"kind":"vec","keyword_span":{"file":0,"start":16,"end":19},"less_than_span":{"file":0,"start":19,"end":20},"argument":0,"greater_than_span":{"file":0,"start":26,"end":27}}},{"span":{"file":0,"start":43,"end":49},"kind":{"kind":"string","keyword_span":{"file":0,"start":43,"end":49}}},{"span":{"file":0,"start":68,"end":74},"kind":{"kind":"string","keyword_span":{"file":0,"start":68,"end":74}}},{"span":{"file":0,"start":64,"end":75},"kind":{"kind":"vec","keyword_span":{"file":0,"start":64,"end":67},"less_than_span":{"file":0,"start":67,"end":68},"argument":3,"greater_than_span":{"file":0,"start":74,"end":75}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":94},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":28,"end":94},"root_block":0,"blocks":[{"span":{"file":0,"start":28,"end":94},"open_brace_span":{"file":0,"start":28,"end":29},"statements":[0,1],"close_brace_span":{"file":0,"start":93,"end":94}}],"statements":[{"span":{"file":0,"start":30,"end":56},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":30,"end":35},"mutable":false,"name":{"text":"first","span":{"file":0,"start":36,"end":41}},"type_syntax":2,"equals_span":{"file":0,"start":50,"end":51},"initializer":0,"semicolon_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":57,"end":92},"kind":{"kind":"return","keyword_span":{"file":0,"start":57,"end":63},"value":3,"semicolon_span":{"file":0,"start":91,"end":92}}}],"expressions":[{"span":{"file":0,"start":52,"end":55},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":77,"end":82},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":77,"end":82}}}},{"span":{"file":0,"start":84,"end":89},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":84,"end":89}}}},{"span":{"file":0,"start":64,"end":91},"kind":{"kind":"vec-construction","type_syntax":4,"open_paren_span":{"file":0,"start":75,"end":76},"open_bracket_span":{"file":0,"start":76,"end":77},"elements":[1,2],"close_bracket_span":{"file":0,"start":89,"end":90},"close_paren_span":{"file":0,"start":90,"end":91}}}]}}]}],"diagnostics":[]}}"#;
const VEC_PUSH_SOURCE: &str = "function append(): Vec<String> { let values: Vec<String> = Vec<String>([\"a\"]); push(values, \"b\"); return values; }";
const VEC_PUSH_RESPONSE: &str = r#"{"id":40,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":23,"end":29},"kind":{"kind":"string","keyword_span":{"file":0,"start":23,"end":29}}},{"span":{"file":0,"start":19,"end":30},"kind":{"kind":"vec","keyword_span":{"file":0,"start":19,"end":22},"less_than_span":{"file":0,"start":22,"end":23},"argument":0,"greater_than_span":{"file":0,"start":29,"end":30}}},{"span":{"file":0,"start":49,"end":55},"kind":{"kind":"string","keyword_span":{"file":0,"start":49,"end":55}}},{"span":{"file":0,"start":45,"end":56},"kind":{"kind":"vec","keyword_span":{"file":0,"start":45,"end":48},"less_than_span":{"file":0,"start":48,"end":49},"argument":2,"greater_than_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":63,"end":69},"kind":{"kind":"string","keyword_span":{"file":0,"start":63,"end":69}}},{"span":{"file":0,"start":59,"end":70},"kind":{"kind":"vec","keyword_span":{"file":0,"start":59,"end":62},"less_than_span":{"file":0,"start":62,"end":63},"argument":4,"greater_than_span":{"file":0,"start":69,"end":70}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":114},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"append","span":{"file":0,"start":9,"end":15}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":31,"end":114},"root_block":0,"blocks":[{"span":{"file":0,"start":31,"end":114},"open_brace_span":{"file":0,"start":31,"end":32},"statements":[0,1,2],"close_brace_span":{"file":0,"start":113,"end":114}}],"statements":[{"span":{"file":0,"start":33,"end":78},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":33,"end":36},"mutable":true,"name":{"text":"values","span":{"file":0,"start":37,"end":43}},"type_syntax":3,"equals_span":{"file":0,"start":57,"end":58},"initializer":1,"semicolon_span":{"file":0,"start":77,"end":78}}},{"span":{"file":0,"start":79,"end":97},"kind":{"kind":"expression-statement","expression":4,"semicolon_span":{"file":0,"start":96,"end":97}}},{"span":{"file":0,"start":98,"end":112},"kind":{"kind":"return","keyword_span":{"file":0,"start":98,"end":104},"value":5,"semicolon_span":{"file":0,"start":111,"end":112}}}],"expressions":[{"span":{"file":0,"start":72,"end":75},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":59,"end":77},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":70,"end":71},"open_bracket_span":{"file":0,"start":71,"end":72},"elements":[0],"close_bracket_span":{"file":0,"start":75,"end":76},"close_paren_span":{"file":0,"start":76,"end":77}}},{"span":{"file":0,"start":84,"end":90},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":84,"end":90}}}},{"span":{"file":0,"start":92,"end":95},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":79,"end":96},"kind":{"kind":"vec-push","keyword_span":{"file":0,"start":79,"end":83},"open_paren_span":{"file":0,"start":83,"end":84},"vector":2,"comma_span":{"file":0,"start":90,"end":91},"value":3,"close_paren_span":{"file":0,"start":95,"end":96}}},{"span":{"file":0,"start":105,"end":111},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":105,"end":111}}}}]}}]}],"diagnostics":[]}}"#;
const VEC_INDEX_SOURCE: &str =
    "function get(): i32 { const values: Vec<i32> = Vec<i32>([10, 20]); return values[-1]; }";
const VEC_INDEX_RESPONSE: &str = r#"{"id":41,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":19},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":16,"end":19}}}},{"span":{"file":0,"start":40,"end":43},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":40,"end":43}}}},{"span":{"file":0,"start":36,"end":44},"kind":{"kind":"vec","keyword_span":{"file":0,"start":36,"end":39},"less_than_span":{"file":0,"start":39,"end":40},"argument":1,"greater_than_span":{"file":0,"start":43,"end":44}}},{"span":{"file":0,"start":51,"end":54},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":51,"end":54}}},{"span":{"file":0,"start":47,"end":55},"kind":{"kind":"vec","keyword_span":{"file":0,"start":47,"end":50},"less_than_span":{"file":0,"start":50,"end":51},"argument":3,"greater_than_span":{"file":0,"start":54,"end":55}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":87},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"get","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":20,"end":87},"root_block":0,"blocks":[{"span":{"file":0,"start":20,"end":87},"open_brace_span":{"file":0,"start":20,"end":21},"statements":[0,1],"close_brace_span":{"file":0,"start":86,"end":87}}],"statements":[{"span":{"file":0,"start":22,"end":66},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":22,"end":27},"mutable":false,"name":{"text":"values","span":{"file":0,"start":28,"end":34}},"type_syntax":2,"equals_span":{"file":0,"start":45,"end":46},"initializer":2,"semicolon_span":{"file":0,"start":65,"end":66}}},{"span":{"file":0,"start":67,"end":85},"kind":{"kind":"return","keyword_span":{"file":0,"start":67,"end":73},"value":5,"semicolon_span":{"file":0,"start":84,"end":85}}}],"expressions":[{"span":{"file":0,"start":57,"end":59},"kind":{"kind":"i32-literal","spelling":"10"}},{"span":{"file":0,"start":61,"end":63},"kind":{"kind":"i32-literal","spelling":"20"}},{"span":{"file":0,"start":47,"end":65},"kind":{"kind":"vec-construction","type_syntax":4,"open_paren_span":{"file":0,"start":55,"end":56},"open_bracket_span":{"file":0,"start":56,"end":57},"elements":[0,1],"close_bracket_span":{"file":0,"start":63,"end":64},"close_paren_span":{"file":0,"start":64,"end":65}}},{"span":{"file":0,"start":74,"end":80},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":74,"end":80}}}},{"span":{"file":0,"start":81,"end":83},"kind":{"kind":"i32-literal","spelling":"-1"}},{"span":{"file":0,"start":74,"end":84},"kind":{"kind":"index","base":3,"open_bracket_span":{"file":0,"start":80,"end":81},"index":4,"close_bracket_span":{"file":0,"start":83,"end":84}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_PAIR_SOURCE: &str = "interface OwnedPair extends ZrynaStruct { first: String; flag: bool; }\nfunction make(): OwnedPair { const p: OwnedPair = OwnedPair({ flag: true, first: \"a\" }); return p; }";
const OWNED_PAIR_RESPONSE: &str = r#"{"id":81,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":49,"end":55},"kind":{"kind":"string","keyword_span":{"file":0,"start":49,"end":55}}},{"span":{"file":0,"start":63,"end":67},"kind":{"kind":"named","name":{"text":"bool","span":{"file":0,"start":63,"end":67}}}},{"span":{"file":0,"start":88,"end":97},"kind":{"kind":"named","name":{"text":"OwnedPair","span":{"file":0,"start":88,"end":97}}}},{"span":{"file":0,"start":109,"end":118},"kind":{"kind":"named","name":{"text":"OwnedPair","span":{"file":0,"start":109,"end":118}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":70},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"OwnedPair","span":{"file":0,"start":10,"end":19}},"extends_span":{"file":0,"start":20,"end":27},"marker_span":{"file":0,"start":28,"end":39},"open_brace_span":{"file":0,"start":40,"end":41},"close_brace_span":{"file":0,"start":69,"end":70},"fields":[{"span":{"file":0,"start":42,"end":56},"name":{"text":"first","span":{"file":0,"start":42,"end":47}},"colon_span":{"file":0,"start":47,"end":48},"semicolon_span":{"file":0,"start":55,"end":56},"type_syntax":0},{"span":{"file":0,"start":57,"end":68},"name":{"text":"flag","span":{"file":0,"start":57,"end":61}},"colon_span":{"file":0,"start":61,"end":62},"semicolon_span":{"file":0,"start":67,"end":68},"type_syntax":1}]}}],"functions":[{"span":{"file":0,"start":71,"end":171},"export_span":null,"function_span":{"file":0,"start":71,"end":79},"name":{"text":"make","span":{"file":0,"start":80,"end":84}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":98,"end":171},"root_block":0,"blocks":[{"span":{"file":0,"start":98,"end":171},"open_brace_span":{"file":0,"start":98,"end":99},"statements":[0,1],"close_brace_span":{"file":0,"start":170,"end":171}}],"statements":[{"span":{"file":0,"start":100,"end":159},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":100,"end":105},"mutable":false,"name":{"text":"p","span":{"file":0,"start":106,"end":107}},"type_syntax":3,"equals_span":{"file":0,"start":119,"end":120},"initializer":2,"semicolon_span":{"file":0,"start":158,"end":159}}},{"span":{"file":0,"start":160,"end":169},"kind":{"kind":"return","keyword_span":{"file":0,"start":160,"end":166},"value":3,"semicolon_span":{"file":0,"start":168,"end":169}}}],"expressions":[{"span":{"file":0,"start":139,"end":143},"kind":{"kind":"bool-literal","value":true}},{"span":{"file":0,"start":152,"end":155},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":121,"end":158},"kind":{"kind":"struct-construction","type_name":{"text":"OwnedPair","span":{"file":0,"start":121,"end":130}},"open_paren_span":{"file":0,"start":130,"end":131},"open_brace_span":{"file":0,"start":131,"end":132},"fields":[{"span":{"file":0,"start":133,"end":143},"kind":{"kind":"explicit","name":{"text":"flag","span":{"file":0,"start":133,"end":137}},"colon_span":{"file":0,"start":137,"end":138},"value":0}},{"span":{"file":0,"start":145,"end":155},"kind":{"kind":"explicit","name":{"text":"first","span":{"file":0,"start":145,"end":150}},"colon_span":{"file":0,"start":150,"end":151},"value":1}}],"close_brace_span":{"file":0,"start":156,"end":157},"close_paren_span":{"file":0,"start":157,"end":158}}},{"span":{"file":0,"start":167,"end":168},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":167,"end":168}}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ARRAY_SOURCE: &str = "function make(): FixedArray<String, 2> { const a: FixedArray<String, 2> = FixedArray<String, 2>([\"x\", \"y\"]); return a; }";
const OWNED_ARRAY_RESPONSE: &str = r#"{"id":82,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":34},"kind":{"kind":"string","keyword_span":{"file":0,"start":28,"end":34}}},{"span":{"file":0,"start":17,"end":38},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":34,"end":35},"length_span":{"file":0,"start":36,"end":37},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":37,"end":38}}},{"span":{"file":0,"start":61,"end":67},"kind":{"kind":"string","keyword_span":{"file":0,"start":61,"end":67}}},{"span":{"file":0,"start":50,"end":71},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":50,"end":60},"less_than_span":{"file":0,"start":60,"end":61},"element":2,"comma_span":{"file":0,"start":67,"end":68},"length_span":{"file":0,"start":69,"end":70},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":70,"end":71}}},{"span":{"file":0,"start":85,"end":91},"kind":{"kind":"string","keyword_span":{"file":0,"start":85,"end":91}}},{"span":{"file":0,"start":74,"end":95},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":74,"end":84},"less_than_span":{"file":0,"start":84,"end":85},"element":4,"comma_span":{"file":0,"start":91,"end":92},"length_span":{"file":0,"start":93,"end":94},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":94,"end":95}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":120},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":39,"end":120},"root_block":0,"blocks":[{"span":{"file":0,"start":39,"end":120},"open_brace_span":{"file":0,"start":39,"end":40},"statements":[0,1],"close_brace_span":{"file":0,"start":119,"end":120}}],"statements":[{"span":{"file":0,"start":41,"end":108},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":41,"end":46},"mutable":false,"name":{"text":"a","span":{"file":0,"start":47,"end":48}},"type_syntax":3,"equals_span":{"file":0,"start":72,"end":73},"initializer":2,"semicolon_span":{"file":0,"start":107,"end":108}}},{"span":{"file":0,"start":109,"end":118},"kind":{"kind":"return","keyword_span":{"file":0,"start":109,"end":115},"value":3,"semicolon_span":{"file":0,"start":117,"end":118}}}],"expressions":[{"span":{"file":0,"start":97,"end":100},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":102,"end":105},"kind":{"kind":"string-literal","spelling":"\"y\""}},{"span":{"file":0,"start":74,"end":107},"kind":{"kind":"fixed-array-construction","type_syntax":5,"open_paren_span":{"file":0,"start":95,"end":96},"open_bracket_span":{"file":0,"start":96,"end":97},"elements":[0,1],"close_bracket_span":{"file":0,"start":105,"end":106},"close_paren_span":{"file":0,"start":106,"end":107}}},{"span":{"file":0,"start":116,"end":117},"kind":{"kind":"reference","name":{"text":"a","span":{"file":0,"start":116,"end":117}}}}]}}]}],"diagnostics":[]}}"#;
const NESTED_OWNED_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\ninterface Outer extends ZrynaStruct { inner: Inner; tail: String; }\nfunction make(): Outer { return Outer({ tail: \"b\", inner: Inner({ text: \"a\" }) }); }";
const NESTED_OWNED_RESPONSE: &str = r#"{"id":83,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":112,"end":118},"kind":{"kind":"string","keyword_span":{"file":0,"start":112,"end":118}}},{"span":{"file":0,"start":139,"end":144},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":139,"end":144}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}},{"span":{"file":0,"start":54,"end":121},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":54,"end":63},"name":{"text":"Outer","span":{"file":0,"start":64,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":89},"open_brace_span":{"file":0,"start":90,"end":91},"close_brace_span":{"file":0,"start":120,"end":121},"fields":[{"span":{"file":0,"start":92,"end":105},"name":{"text":"inner","span":{"file":0,"start":92,"end":97}},"colon_span":{"file":0,"start":97,"end":98},"semicolon_span":{"file":0,"start":104,"end":105},"type_syntax":1},{"span":{"file":0,"start":106,"end":119},"name":{"text":"tail","span":{"file":0,"start":106,"end":110}},"colon_span":{"file":0,"start":110,"end":111},"semicolon_span":{"file":0,"start":118,"end":119},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":122,"end":206},"export_span":null,"function_span":{"file":0,"start":122,"end":130},"name":{"text":"make","span":{"file":0,"start":131,"end":135}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":145,"end":206},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":206},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0],"close_brace_span":{"file":0,"start":205,"end":206}}],"statements":[{"span":{"file":0,"start":147,"end":204},"kind":{"kind":"return","keyword_span":{"file":0,"start":147,"end":153},"value":3,"semicolon_span":{"file":0,"start":203,"end":204}}}],"expressions":[{"span":{"file":0,"start":168,"end":171},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":194,"end":197},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":180,"end":200},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":180,"end":185}},"open_paren_span":{"file":0,"start":185,"end":186},"open_brace_span":{"file":0,"start":186,"end":187},"fields":[{"span":{"file":0,"start":188,"end":197},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":188,"end":192}},"colon_span":{"file":0,"start":192,"end":193},"value":1}}],"close_brace_span":{"file":0,"start":198,"end":199},"close_paren_span":{"file":0,"start":199,"end":200}}},{"span":{"file":0,"start":154,"end":203},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":154,"end":159}},"open_paren_span":{"file":0,"start":159,"end":160},"open_brace_span":{"file":0,"start":160,"end":161},"fields":[{"span":{"file":0,"start":162,"end":171},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":162,"end":166}},"colon_span":{"file":0,"start":166,"end":167},"value":0}},{"span":{"file":0,"start":173,"end":200},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":173,"end":178}},"colon_span":{"file":0,"start":178,"end":179},"value":2}}],"close_brace_span":{"file":0,"start":201,"end":202},"close_paren_span":{"file":0,"start":202,"end":203}}}]}}]}],"diagnostics":[]}}"#;
const PROJECTED_INNER_MOVE_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\ninterface Outer extends ZrynaStruct { inner: Inner; tail: String; }\nfunction make(): Outer { const o: Outer = Outer({ tail: \"b\", inner: Inner({ text: \"a\" }) }); const moved: Inner = o.inner; return Outer({ tail: \"c\", inner: moved }); }";
const PROJECTED_INNER_MOVE_RESPONSE: &str = r#"{"id":811,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":112,"end":118},"kind":{"kind":"string","keyword_span":{"file":0,"start":112,"end":118}}},{"span":{"file":0,"start":139,"end":144},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":139,"end":144}}}},{"span":{"file":0,"start":156,"end":161},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":156,"end":161}}}},{"span":{"file":0,"start":228,"end":233},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":228,"end":233}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}},{"span":{"file":0,"start":54,"end":121},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":54,"end":63},"name":{"text":"Outer","span":{"file":0,"start":64,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":89},"open_brace_span":{"file":0,"start":90,"end":91},"close_brace_span":{"file":0,"start":120,"end":121},"fields":[{"span":{"file":0,"start":92,"end":105},"name":{"text":"inner","span":{"file":0,"start":92,"end":97}},"colon_span":{"file":0,"start":97,"end":98},"semicolon_span":{"file":0,"start":104,"end":105},"type_syntax":1},{"span":{"file":0,"start":106,"end":119},"name":{"text":"tail","span":{"file":0,"start":106,"end":110}},"colon_span":{"file":0,"start":110,"end":111},"semicolon_span":{"file":0,"start":118,"end":119},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":122,"end":289},"export_span":null,"function_span":{"file":0,"start":122,"end":130},"name":{"text":"make","span":{"file":0,"start":131,"end":135}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":145,"end":289},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":289},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0,1,2],"close_brace_span":{"file":0,"start":288,"end":289}}],"statements":[{"span":{"file":0,"start":147,"end":214},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":147,"end":152},"mutable":false,"name":{"text":"o","span":{"file":0,"start":153,"end":154}},"type_syntax":4,"equals_span":{"file":0,"start":162,"end":163},"initializer":3,"semicolon_span":{"file":0,"start":213,"end":214}}},{"span":{"file":0,"start":215,"end":244},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":215,"end":220},"mutable":false,"name":{"text":"moved","span":{"file":0,"start":221,"end":226}},"type_syntax":5,"equals_span":{"file":0,"start":234,"end":235},"initializer":5,"semicolon_span":{"file":0,"start":243,"end":244}}},{"span":{"file":0,"start":245,"end":287},"kind":{"kind":"return","keyword_span":{"file":0,"start":245,"end":251},"value":8,"semicolon_span":{"file":0,"start":286,"end":287}}}],"expressions":[{"span":{"file":0,"start":178,"end":181},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":204,"end":207},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":190,"end":210},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":190,"end":195}},"open_paren_span":{"file":0,"start":195,"end":196},"open_brace_span":{"file":0,"start":196,"end":197},"fields":[{"span":{"file":0,"start":198,"end":207},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":198,"end":202}},"colon_span":{"file":0,"start":202,"end":203},"value":1}}],"close_brace_span":{"file":0,"start":208,"end":209},"close_paren_span":{"file":0,"start":209,"end":210}}},{"span":{"file":0,"start":164,"end":213},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":164,"end":169}},"open_paren_span":{"file":0,"start":169,"end":170},"open_brace_span":{"file":0,"start":170,"end":171},"fields":[{"span":{"file":0,"start":172,"end":181},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":172,"end":176}},"colon_span":{"file":0,"start":176,"end":177},"value":0}},{"span":{"file":0,"start":183,"end":210},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":183,"end":188}},"colon_span":{"file":0,"start":188,"end":189},"value":2}}],"close_brace_span":{"file":0,"start":211,"end":212},"close_paren_span":{"file":0,"start":212,"end":213}}},{"span":{"file":0,"start":236,"end":237},"kind":{"kind":"reference","name":{"text":"o","span":{"file":0,"start":236,"end":237}}}},{"span":{"file":0,"start":236,"end":243},"kind":{"kind":"field-access","base":4,"dot_span":{"file":0,"start":237,"end":238},"field":{"text":"inner","span":{"file":0,"start":238,"end":243}}}},{"span":{"file":0,"start":266,"end":269},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":278,"end":283},"kind":{"kind":"reference","name":{"text":"moved","span":{"file":0,"start":278,"end":283}}}},{"span":{"file":0,"start":252,"end":286},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":252,"end":257}},"open_paren_span":{"file":0,"start":257,"end":258},"open_brace_span":{"file":0,"start":258,"end":259},"fields":[{"span":{"file":0,"start":260,"end":269},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":260,"end":264}},"colon_span":{"file":0,"start":264,"end":265},"value":6}},{"span":{"file":0,"start":271,"end":283},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":271,"end":276}},"colon_span":{"file":0,"start":276,"end":277},"value":7}}],"close_brace_span":{"file":0,"start":284,"end":285},"close_paren_span":{"file":0,"start":285,"end":286}}}]}}]}],"diagnostics":[]}}"#;
const PROJECTED_ARRAY_ELEMENT_MOVE_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\nfunction make(): FixedArray<Inner, 2> { const a: FixedArray<Inner, 2> = FixedArray<Inner, 2>([Inner({ text: \"a\" }), Inner({ text: \"b\" })]); const moved: Inner = a[0]; return FixedArray<Inner, 2>([moved, Inner({ text: \"c\" })]); }";
const PROJECTED_ARRAY_ELEMENT_MOVE_RESPONSE: &str = r#"{"id":812,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":82,"end":87},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":82,"end":87}}}},{"span":{"file":0,"start":71,"end":91},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":71,"end":81},"less_than_span":{"file":0,"start":81,"end":82},"element":1,"comma_span":{"file":0,"start":87,"end":88},"length_span":{"file":0,"start":89,"end":90},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":90,"end":91}}},{"span":{"file":0,"start":114,"end":119},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":114,"end":119}}}},{"span":{"file":0,"start":103,"end":123},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":103,"end":113},"less_than_span":{"file":0,"start":113,"end":114},"element":3,"comma_span":{"file":0,"start":119,"end":120},"length_span":{"file":0,"start":121,"end":122},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":122,"end":123}}},{"span":{"file":0,"start":137,"end":142},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":137,"end":142}}}},{"span":{"file":0,"start":126,"end":146},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":126,"end":136},"less_than_span":{"file":0,"start":136,"end":137},"element":5,"comma_span":{"file":0,"start":142,"end":143},"length_span":{"file":0,"start":144,"end":145},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":145,"end":146}}},{"span":{"file":0,"start":207,"end":212},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":207,"end":212}}}},{"span":{"file":0,"start":239,"end":244},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":239,"end":244}}}},{"span":{"file":0,"start":228,"end":248},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":228,"end":238},"less_than_span":{"file":0,"start":238,"end":239},"element":8,"comma_span":{"file":0,"start":244,"end":245},"length_span":{"file":0,"start":246,"end":247},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":247,"end":248}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}}],"functions":[{"span":{"file":0,"start":54,"end":282},"export_span":null,"function_span":{"file":0,"start":54,"end":62},"name":{"text":"make","span":{"file":0,"start":63,"end":67}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":92,"end":282},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":282},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1,2],"close_brace_span":{"file":0,"start":281,"end":282}}],"statements":[{"span":{"file":0,"start":94,"end":193},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":94,"end":99},"mutable":false,"name":{"text":"a","span":{"file":0,"start":100,"end":101}},"type_syntax":4,"equals_span":{"file":0,"start":124,"end":125},"initializer":4,"semicolon_span":{"file":0,"start":192,"end":193}}},{"span":{"file":0,"start":194,"end":220},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":194,"end":199},"mutable":false,"name":{"text":"moved","span":{"file":0,"start":200,"end":205}},"type_syntax":7,"equals_span":{"file":0,"start":213,"end":214},"initializer":7,"semicolon_span":{"file":0,"start":219,"end":220}}},{"span":{"file":0,"start":221,"end":280},"kind":{"kind":"return","keyword_span":{"file":0,"start":221,"end":227},"value":11,"semicolon_span":{"file":0,"start":279,"end":280}}}],"expressions":[{"span":{"file":0,"start":162,"end":165},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":148,"end":168},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":148,"end":153}},"open_paren_span":{"file":0,"start":153,"end":154},"open_brace_span":{"file":0,"start":154,"end":155},"fields":[{"span":{"file":0,"start":156,"end":165},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":156,"end":160}},"colon_span":{"file":0,"start":160,"end":161},"value":0}}],"close_brace_span":{"file":0,"start":166,"end":167},"close_paren_span":{"file":0,"start":167,"end":168}}},{"span":{"file":0,"start":184,"end":187},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":170,"end":190},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":170,"end":175}},"open_paren_span":{"file":0,"start":175,"end":176},"open_brace_span":{"file":0,"start":176,"end":177},"fields":[{"span":{"file":0,"start":178,"end":187},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":178,"end":182}},"colon_span":{"file":0,"start":182,"end":183},"value":2}}],"close_brace_span":{"file":0,"start":188,"end":189},"close_paren_span":{"file":0,"start":189,"end":190}}},{"span":{"file":0,"start":126,"end":192},"kind":{"kind":"fixed-array-construction","type_syntax":6,"open_paren_span":{"file":0,"start":146,"end":147},"open_bracket_span":{"file":0,"start":147,"end":148},"elements":[1,3],"close_bracket_span":{"file":0,"start":190,"end":191},"close_paren_span":{"file":0,"start":191,"end":192}}},{"span":{"file":0,"start":215,"end":216},"kind":{"kind":"reference","name":{"text":"a","span":{"file":0,"start":215,"end":216}}}},{"span":{"file":0,"start":217,"end":218},"kind":{"kind":"i32-literal","spelling":"0"}},{"span":{"file":0,"start":215,"end":219},"kind":{"kind":"index","base":5,"open_bracket_span":{"file":0,"start":216,"end":217},"index":6,"close_bracket_span":{"file":0,"start":218,"end":219}}},{"span":{"file":0,"start":250,"end":255},"kind":{"kind":"reference","name":{"text":"moved","span":{"file":0,"start":250,"end":255}}}},{"span":{"file":0,"start":271,"end":274},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":257,"end":277},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":257,"end":262}},"open_paren_span":{"file":0,"start":262,"end":263},"open_brace_span":{"file":0,"start":263,"end":264},"fields":[{"span":{"file":0,"start":265,"end":274},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":265,"end":269}},"colon_span":{"file":0,"start":269,"end":270},"value":9}}],"close_brace_span":{"file":0,"start":275,"end":276},"close_paren_span":{"file":0,"start":276,"end":277}}},{"span":{"file":0,"start":228,"end":279},"kind":{"kind":"fixed-array-construction","type_syntax":9,"open_paren_span":{"file":0,"start":248,"end":249},"open_bracket_span":{"file":0,"start":249,"end":250},"elements":[8,10],"close_bracket_span":{"file":0,"start":277,"end":278},"close_paren_span":{"file":0,"start":278,"end":279}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_TRIO_SOURCE: &str = "interface Trio extends ZrynaStruct { a: String; b: String; c: String; }\nfunction make(): Trio { return Trio({ c: \"c\", b: \"b\", a: \"a\" }); }";
const OWNED_TRIO_RESPONSE: &str = r#"{"id":84,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":40,"end":46},"kind":{"kind":"string","keyword_span":{"file":0,"start":40,"end":46}}},{"span":{"file":0,"start":51,"end":57},"kind":{"kind":"string","keyword_span":{"file":0,"start":51,"end":57}}},{"span":{"file":0,"start":62,"end":68},"kind":{"kind":"string","keyword_span":{"file":0,"start":62,"end":68}}},{"span":{"file":0,"start":89,"end":93},"kind":{"kind":"named","name":{"text":"Trio","span":{"file":0,"start":89,"end":93}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":71},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Trio","span":{"file":0,"start":10,"end":14}},"extends_span":{"file":0,"start":15,"end":22},"marker_span":{"file":0,"start":23,"end":34},"open_brace_span":{"file":0,"start":35,"end":36},"close_brace_span":{"file":0,"start":70,"end":71},"fields":[{"span":{"file":0,"start":37,"end":47},"name":{"text":"a","span":{"file":0,"start":37,"end":38}},"colon_span":{"file":0,"start":38,"end":39},"semicolon_span":{"file":0,"start":46,"end":47},"type_syntax":0},{"span":{"file":0,"start":48,"end":58},"name":{"text":"b","span":{"file":0,"start":48,"end":49}},"colon_span":{"file":0,"start":49,"end":50},"semicolon_span":{"file":0,"start":57,"end":58},"type_syntax":1},{"span":{"file":0,"start":59,"end":69},"name":{"text":"c","span":{"file":0,"start":59,"end":60}},"colon_span":{"file":0,"start":60,"end":61},"semicolon_span":{"file":0,"start":68,"end":69},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":72,"end":138},"export_span":null,"function_span":{"file":0,"start":72,"end":80},"name":{"text":"make","span":{"file":0,"start":81,"end":85}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":94,"end":138},"root_block":0,"blocks":[{"span":{"file":0,"start":94,"end":138},"open_brace_span":{"file":0,"start":94,"end":95},"statements":[0],"close_brace_span":{"file":0,"start":137,"end":138}}],"statements":[{"span":{"file":0,"start":96,"end":136},"kind":{"kind":"return","keyword_span":{"file":0,"start":96,"end":102},"value":3,"semicolon_span":{"file":0,"start":135,"end":136}}}],"expressions":[{"span":{"file":0,"start":113,"end":116},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":121,"end":124},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":129,"end":132},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":103,"end":135},"kind":{"kind":"struct-construction","type_name":{"text":"Trio","span":{"file":0,"start":103,"end":107}},"open_paren_span":{"file":0,"start":107,"end":108},"open_brace_span":{"file":0,"start":108,"end":109},"fields":[{"span":{"file":0,"start":110,"end":116},"kind":{"kind":"explicit","name":{"text":"c","span":{"file":0,"start":110,"end":111}},"colon_span":{"file":0,"start":111,"end":112},"value":0}},{"span":{"file":0,"start":118,"end":124},"kind":{"kind":"explicit","name":{"text":"b","span":{"file":0,"start":118,"end":119}},"colon_span":{"file":0,"start":119,"end":120},"value":1}},{"span":{"file":0,"start":126,"end":132},"kind":{"kind":"explicit","name":{"text":"a","span":{"file":0,"start":126,"end":127}},"colon_span":{"file":0,"start":127,"end":128},"value":2}}],"close_brace_span":{"file":0,"start":133,"end":134},"close_paren_span":{"file":0,"start":134,"end":135}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_CROSS_SOURCE: &str = "interface Box extends ZrynaStruct { items: FixedArray<String, 2>; }\nfunction make(): Box { return Box({ items: FixedArray<String, 2>([\"a\", \"b\"]) }); }";
const OWNED_CROSS_RESPONSE: &str = r#"{"id":85,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":54,"end":60},"kind":{"kind":"string","keyword_span":{"file":0,"start":54,"end":60}}},{"span":{"file":0,"start":43,"end":64},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":43,"end":53},"less_than_span":{"file":0,"start":53,"end":54},"element":0,"comma_span":{"file":0,"start":60,"end":61},"length_span":{"file":0,"start":62,"end":63},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":63,"end":64}}},{"span":{"file":0,"start":85,"end":88},"kind":{"kind":"named","name":{"text":"Box","span":{"file":0,"start":85,"end":88}}}},{"span":{"file":0,"start":122,"end":128},"kind":{"kind":"string","keyword_span":{"file":0,"start":122,"end":128}}},{"span":{"file":0,"start":111,"end":132},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":111,"end":121},"less_than_span":{"file":0,"start":121,"end":122},"element":3,"comma_span":{"file":0,"start":128,"end":129},"length_span":{"file":0,"start":130,"end":131},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":131,"end":132}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":67},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Box","span":{"file":0,"start":10,"end":13}},"extends_span":{"file":0,"start":14,"end":21},"marker_span":{"file":0,"start":22,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":66,"end":67},"fields":[{"span":{"file":0,"start":36,"end":65},"name":{"text":"items","span":{"file":0,"start":36,"end":41}},"colon_span":{"file":0,"start":41,"end":42},"semicolon_span":{"file":0,"start":64,"end":65},"type_syntax":1}]}}],"functions":[{"span":{"file":0,"start":68,"end":150},"export_span":null,"function_span":{"file":0,"start":68,"end":76},"name":{"text":"make","span":{"file":0,"start":77,"end":81}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":89,"end":150},"root_block":0,"blocks":[{"span":{"file":0,"start":89,"end":150},"open_brace_span":{"file":0,"start":89,"end":90},"statements":[0],"close_brace_span":{"file":0,"start":149,"end":150}}],"statements":[{"span":{"file":0,"start":91,"end":148},"kind":{"kind":"return","keyword_span":{"file":0,"start":91,"end":97},"value":3,"semicolon_span":{"file":0,"start":147,"end":148}}}],"expressions":[{"span":{"file":0,"start":134,"end":137},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":139,"end":142},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":111,"end":144},"kind":{"kind":"fixed-array-construction","type_syntax":4,"open_paren_span":{"file":0,"start":132,"end":133},"open_bracket_span":{"file":0,"start":133,"end":134},"elements":[0,1],"close_bracket_span":{"file":0,"start":142,"end":143},"close_paren_span":{"file":0,"start":143,"end":144}}},{"span":{"file":0,"start":98,"end":147},"kind":{"kind":"struct-construction","type_name":{"text":"Box","span":{"file":0,"start":98,"end":101}},"open_paren_span":{"file":0,"start":101,"end":102},"open_brace_span":{"file":0,"start":102,"end":103},"fields":[{"span":{"file":0,"start":104,"end":144},"kind":{"kind":"explicit","name":{"text":"items","span":{"file":0,"start":104,"end":109}},"colon_span":{"file":0,"start":109,"end":110},"value":2}}],"close_brace_span":{"file":0,"start":145,"end":146},"close_paren_span":{"file":0,"start":146,"end":147}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_NONE_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { return Maybe.none(); }";
const OWNED_ENUM_NONE_RESPONSE: &str = r#"{"id":10,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":116},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":116},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":116},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0],"close_brace_span":{"file":0,"start":115,"end":116}}],"statements":[{"span":{"file":0,"start":94,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":94,"end":100},"value":0,"semicolon_span":{"file":0,"start":113,"end":114}}}],"expressions":[{"span":{"file":0,"start":101,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":101,"end":106}},"dot_span":{"file":0,"start":106,"end":107},"variant":{"text":"none","span":{"file":0,"start":107,"end":111}},"open_paren_span":{"file":0,"start":111,"end":112},"payload":null,"close_paren_span":{"file":0,"start":112,"end":113}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_COPY_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction make(): Maybe { return Maybe.some(7); }";
const OWNED_ENUM_COPY_RESPONSE: &str = r#"{"id":11,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":83,"end":88},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":83,"end":88}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":114},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"make","span":{"file":0,"start":75,"end":79}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":89,"end":114},"root_block":0,"blocks":[{"span":{"file":0,"start":89,"end":114},"open_brace_span":{"file":0,"start":89,"end":90},"statements":[0],"close_brace_span":{"file":0,"start":113,"end":114}}],"statements":[{"span":{"file":0,"start":91,"end":112},"kind":{"kind":"return","keyword_span":{"file":0,"start":91,"end":97},"value":1,"semicolon_span":{"file":0,"start":111,"end":112}}}],"expressions":[{"span":{"file":0,"start":109,"end":110},"kind":{"kind":"i32-literal","spelling":"7"}},{"span":{"file":0,"start":98,"end":111},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":98,"end":103}},"dot_span":{"file":0,"start":103,"end":104},"variant":{"text":"some","span":{"file":0,"start":104,"end":108}},"open_paren_span":{"file":0,"start":108,"end":109},"payload":0,"close_paren_span":{"file":0,"start":110,"end":111}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_STRING_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { const survivor: String = \"s\"; const x: Maybe = Maybe.some(\"x\"); const y: Maybe = x; return y; }";
const OWNED_ENUM_STRING_RESPONSE: &str = r#"{"id":12,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}},{"span":{"file":0,"start":110,"end":116},"kind":{"kind":"string","keyword_span":{"file":0,"start":110,"end":116}}},{"span":{"file":0,"start":133,"end":138},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":133,"end":138}}}},{"span":{"file":0,"start":167,"end":172},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":167,"end":172}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":189},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":189},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":189},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":188,"end":189}}],"statements":[{"span":{"file":0,"start":94,"end":123},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":94,"end":99},"mutable":false,"name":{"text":"survivor","span":{"file":0,"start":100,"end":108}},"type_syntax":2,"equals_span":{"file":0,"start":117,"end":118},"initializer":0,"semicolon_span":{"file":0,"start":122,"end":123}}},{"span":{"file":0,"start":124,"end":157},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":124,"end":129},"mutable":false,"name":{"text":"x","span":{"file":0,"start":130,"end":131}},"type_syntax":3,"equals_span":{"file":0,"start":139,"end":140},"initializer":2,"semicolon_span":{"file":0,"start":156,"end":157}}},{"span":{"file":0,"start":158,"end":177},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":158,"end":163},"mutable":false,"name":{"text":"y","span":{"file":0,"start":164,"end":165}},"type_syntax":4,"equals_span":{"file":0,"start":173,"end":174},"initializer":3,"semicolon_span":{"file":0,"start":176,"end":177}}},{"span":{"file":0,"start":178,"end":187},"kind":{"kind":"return","keyword_span":{"file":0,"start":178,"end":184},"value":4,"semicolon_span":{"file":0,"start":186,"end":187}}}],"expressions":[{"span":{"file":0,"start":119,"end":122},"kind":{"kind":"string-literal","spelling":"\"s\""}},{"span":{"file":0,"start":152,"end":155},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":141,"end":156},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":141,"end":146}},"dot_span":{"file":0,"start":146,"end":147},"variant":{"text":"some","span":{"file":0,"start":147,"end":151}},"open_paren_span":{"file":0,"start":151,"end":152},"payload":1,"close_paren_span":{"file":0,"start":155,"end":156}}},{"span":{"file":0,"start":175,"end":176},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":175,"end":176}}}},{"span":{"file":0,"start":185,"end":186},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":185,"end":186}}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_NESTED_SOURCE: &str = "interface Box extends ZrynaStruct { text: String; }\ninterface Wrapped extends ZrynaEnum { none: ZrynaNone; some: Box; }\nfunction make(): Wrapped { return Wrapped.some(Box({ text: \"x\" })); }";
const OWNED_ENUM_NESTED_RESPONSE: &str = r#"{"id":13,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":42,"end":48},"kind":{"kind":"string","keyword_span":{"file":0,"start":42,"end":48}}},{"span":{"file":0,"start":113,"end":116},"kind":{"kind":"named","name":{"text":"Box","span":{"file":0,"start":113,"end":116}}}},{"span":{"file":0,"start":137,"end":144},"kind":{"kind":"named","name":{"text":"Wrapped","span":{"file":0,"start":137,"end":144}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":51},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Box","span":{"file":0,"start":10,"end":13}},"extends_span":{"file":0,"start":14,"end":21},"marker_span":{"file":0,"start":22,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":50,"end":51},"fields":[{"span":{"file":0,"start":36,"end":49},"name":{"text":"text","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":48,"end":49},"type_syntax":0}]}},{"span":{"file":0,"start":52,"end":119},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":52,"end":61},"name":{"text":"Wrapped","span":{"file":0,"start":62,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":87},"open_brace_span":{"file":0,"start":88,"end":89},"close_brace_span":{"file":0,"start":118,"end":119},"variants":[{"span":{"file":0,"start":90,"end":106},"name":{"text":"none","span":{"file":0,"start":90,"end":94}},"colon_span":{"file":0,"start":94,"end":95},"semicolon_span":{"file":0,"start":105,"end":106},"payload_type":null,"none_span":{"file":0,"start":96,"end":105}},{"span":{"file":0,"start":107,"end":117},"name":{"text":"some","span":{"file":0,"start":107,"end":111}},"colon_span":{"file":0,"start":111,"end":112},"semicolon_span":{"file":0,"start":116,"end":117},"payload_type":1,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":120,"end":189},"export_span":null,"function_span":{"file":0,"start":120,"end":128},"name":{"text":"make","span":{"file":0,"start":129,"end":133}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":145,"end":189},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":189},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0],"close_brace_span":{"file":0,"start":188,"end":189}}],"statements":[{"span":{"file":0,"start":147,"end":187},"kind":{"kind":"return","keyword_span":{"file":0,"start":147,"end":153},"value":2,"semicolon_span":{"file":0,"start":186,"end":187}}}],"expressions":[{"span":{"file":0,"start":179,"end":182},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":167,"end":185},"kind":{"kind":"struct-construction","type_name":{"text":"Box","span":{"file":0,"start":167,"end":170}},"open_paren_span":{"file":0,"start":170,"end":171},"open_brace_span":{"file":0,"start":171,"end":172},"fields":[{"span":{"file":0,"start":173,"end":182},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":173,"end":177}},"colon_span":{"file":0,"start":177,"end":178},"value":0}}],"close_brace_span":{"file":0,"start":183,"end":184},"close_paren_span":{"file":0,"start":184,"end":185}}},{"span":{"file":0,"start":154,"end":186},"kind":{"kind":"enum-construction","type_name":{"text":"Wrapped","span":{"file":0,"start":154,"end":161}},"dot_span":{"file":0,"start":161,"end":162},"variant":{"text":"some","span":{"file":0,"start":162,"end":166}},"open_paren_span":{"file":0,"start":166,"end":167},"payload":1,"close_paren_span":{"file":0,"start":185,"end":186}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_MOVED_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { const x: Maybe = Maybe.some(\"x\"); const y: Maybe = x; return x; }";
const OWNED_ENUM_MOVED_RESPONSE: &str = r#"{"id":14,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}},{"span":{"file":0,"start":103,"end":108},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":103,"end":108}}}},{"span":{"file":0,"start":137,"end":142},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":137,"end":142}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":159},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":159},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":159},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1,2],"close_brace_span":{"file":0,"start":158,"end":159}}],"statements":[{"span":{"file":0,"start":94,"end":127},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":94,"end":99},"mutable":false,"name":{"text":"x","span":{"file":0,"start":100,"end":101}},"type_syntax":2,"equals_span":{"file":0,"start":109,"end":110},"initializer":1,"semicolon_span":{"file":0,"start":126,"end":127}}},{"span":{"file":0,"start":128,"end":147},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":128,"end":133},"mutable":false,"name":{"text":"y","span":{"file":0,"start":134,"end":135}},"type_syntax":3,"equals_span":{"file":0,"start":143,"end":144},"initializer":2,"semicolon_span":{"file":0,"start":146,"end":147}}},{"span":{"file":0,"start":148,"end":157},"kind":{"kind":"return","keyword_span":{"file":0,"start":148,"end":154},"value":3,"semicolon_span":{"file":0,"start":156,"end":157}}}],"expressions":[{"span":{"file":0,"start":122,"end":125},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":111,"end":126},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":111,"end":116}},"dot_span":{"file":0,"start":116,"end":117},"variant":{"text":"some","span":{"file":0,"start":117,"end":121}},"open_paren_span":{"file":0,"start":121,"end":122},"payload":0,"close_paren_span":{"file":0,"start":125,"end":126}}},{"span":{"file":0,"start":145,"end":146},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":145,"end":146}}}},{"span":{"file":0,"start":155,"end":156},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":155,"end":156}}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_VEC_SOURCE: &str = "interface Bad extends ZrynaEnum { some: Vec<String>; }\nfunction make(): Bad { return Bad.some(Vec<String>([\"x\"])); }";
const OWNED_ENUM_VEC_RESPONSE: &str = r#"{"id":15,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":40,"end":51},"kind":{"kind":"vec","keyword_span":{"file":0,"start":40,"end":43},"less_than_span":{"file":0,"start":43,"end":44},"argument":0,"greater_than_span":{"file":0,"start":50,"end":51}}},{"span":{"file":0,"start":72,"end":75},"kind":{"kind":"named","name":{"text":"Bad","span":{"file":0,"start":72,"end":75}}}},{"span":{"file":0,"start":98,"end":104},"kind":{"kind":"string","keyword_span":{"file":0,"start":98,"end":104}}},{"span":{"file":0,"start":94,"end":105},"kind":{"kind":"vec","keyword_span":{"file":0,"start":94,"end":97},"less_than_span":{"file":0,"start":97,"end":98},"argument":3,"greater_than_span":{"file":0,"start":104,"end":105}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":54},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Bad","span":{"file":0,"start":10,"end":13}},"extends_span":{"file":0,"start":14,"end":21},"marker_span":{"file":0,"start":22,"end":31},"open_brace_span":{"file":0,"start":32,"end":33},"close_brace_span":{"file":0,"start":53,"end":54},"variants":[{"span":{"file":0,"start":34,"end":52},"name":{"text":"some","span":{"file":0,"start":34,"end":38}},"colon_span":{"file":0,"start":38,"end":39},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":1,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":55,"end":116},"export_span":null,"function_span":{"file":0,"start":55,"end":63},"name":{"text":"make","span":{"file":0,"start":64,"end":68}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":76,"end":116},"root_block":0,"blocks":[{"span":{"file":0,"start":76,"end":116},"open_brace_span":{"file":0,"start":76,"end":77},"statements":[0],"close_brace_span":{"file":0,"start":115,"end":116}}],"statements":[{"span":{"file":0,"start":78,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":78,"end":84},"value":2,"semicolon_span":{"file":0,"start":113,"end":114}}}],"expressions":[{"span":{"file":0,"start":107,"end":110},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":94,"end":112},"kind":{"kind":"vec-construction","type_syntax":4,"open_paren_span":{"file":0,"start":105,"end":106},"open_bracket_span":{"file":0,"start":106,"end":107},"elements":[0],"close_bracket_span":{"file":0,"start":110,"end":111},"close_paren_span":{"file":0,"start":111,"end":112}}},{"span":{"file":0,"start":85,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Bad","span":{"file":0,"start":85,"end":88}},"dot_span":{"file":0,"start":88,"end":89},"variant":{"text":"some","span":{"file":0,"start":89,"end":93}},"open_paren_span":{"file":0,"start":93,"end":94},"payload":1,"close_paren_span":{"file":0,"start":112,"end":113}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_DUP_RETURN_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { return Maybe.none(); return Maybe.none(); }";
const OWNED_ENUM_DUP_RETURN_RESPONSE: &str = r#"{"id":20,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":137},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":137},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":137},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1],"close_brace_span":{"file":0,"start":136,"end":137}}],"statements":[{"span":{"file":0,"start":94,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":94,"end":100},"value":0,"semicolon_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":115,"end":135},"kind":{"kind":"return","keyword_span":{"file":0,"start":115,"end":121},"value":1,"semicolon_span":{"file":0,"start":134,"end":135}}}],"expressions":[{"span":{"file":0,"start":101,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":101,"end":106}},"dot_span":{"file":0,"start":106,"end":107},"variant":{"text":"none","span":{"file":0,"start":107,"end":111}},"open_paren_span":{"file":0,"start":111,"end":112},"payload":null,"close_paren_span":{"file":0,"start":112,"end":113}}},{"span":{"file":0,"start":122,"end":134},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":122,"end":127}},"dot_span":{"file":0,"start":127,"end":128},"variant":{"text":"none","span":{"file":0,"start":128,"end":132}},"open_paren_span":{"file":0,"start":132,"end":133},"payload":null,"close_paren_span":{"file":0,"start":133,"end":134}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_LOCAL_AFTER_RETURN_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { return Maybe.none(); const x: Maybe = Maybe.none(); }";
const OWNED_ENUM_LOCAL_AFTER_RETURN_RESPONSE: &str = r#"{"id":21,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}},{"span":{"file":0,"start":124,"end":129},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":124,"end":129}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":147},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":147},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":147},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1],"close_brace_span":{"file":0,"start":146,"end":147}}],"statements":[{"span":{"file":0,"start":94,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":94,"end":100},"value":0,"semicolon_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":115,"end":145},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":115,"end":120},"mutable":false,"name":{"text":"x","span":{"file":0,"start":121,"end":122}},"type_syntax":2,"equals_span":{"file":0,"start":130,"end":131},"initializer":1,"semicolon_span":{"file":0,"start":144,"end":145}}}],"expressions":[{"span":{"file":0,"start":101,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":101,"end":106}},"dot_span":{"file":0,"start":106,"end":107},"variant":{"text":"none","span":{"file":0,"start":107,"end":111}},"open_paren_span":{"file":0,"start":111,"end":112},"payload":null,"close_paren_span":{"file":0,"start":112,"end":113}}},{"span":{"file":0,"start":132,"end":144},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":132,"end":137}},"dot_span":{"file":0,"start":137,"end":138},"variant":{"text":"none","span":{"file":0,"start":138,"end":142}},"open_paren_span":{"file":0,"start":142,"end":143},"payload":null,"close_paren_span":{"file":0,"start":143,"end":144}}}]}}]}],"diagnostics":[]}}"#;

const PROJECTED_INNER_DIRECT_RETURN_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\ninterface Outer extends ZrynaStruct { inner: Inner; tail: String; }\nfunction make(): Inner { const o: Outer = Outer({ tail: \"b\", inner: Inner({ text: \"a\" }) }); return o.inner; }";
const PROJECTED_INNER_DIRECT_RETURN_RESPONSE: &str = r#"{"id":813,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":112,"end":118},"kind":{"kind":"string","keyword_span":{"file":0,"start":112,"end":118}}},{"span":{"file":0,"start":139,"end":144},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":139,"end":144}}}},{"span":{"file":0,"start":156,"end":161},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":156,"end":161}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}},{"span":{"file":0,"start":54,"end":121},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":54,"end":63},"name":{"text":"Outer","span":{"file":0,"start":64,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":89},"open_brace_span":{"file":0,"start":90,"end":91},"close_brace_span":{"file":0,"start":120,"end":121},"fields":[{"span":{"file":0,"start":92,"end":105},"name":{"text":"inner","span":{"file":0,"start":92,"end":97}},"colon_span":{"file":0,"start":97,"end":98},"semicolon_span":{"file":0,"start":104,"end":105},"type_syntax":1},{"span":{"file":0,"start":106,"end":119},"name":{"text":"tail","span":{"file":0,"start":106,"end":110}},"colon_span":{"file":0,"start":110,"end":111},"semicolon_span":{"file":0,"start":118,"end":119},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":122,"end":232},"export_span":null,"function_span":{"file":0,"start":122,"end":130},"name":{"text":"make","span":{"file":0,"start":131,"end":135}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":145,"end":232},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":232},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0,1],"close_brace_span":{"file":0,"start":231,"end":232}}],"statements":[{"span":{"file":0,"start":147,"end":214},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":147,"end":152},"mutable":false,"name":{"text":"o","span":{"file":0,"start":153,"end":154}},"type_syntax":4,"equals_span":{"file":0,"start":162,"end":163},"initializer":3,"semicolon_span":{"file":0,"start":213,"end":214}}},{"span":{"file":0,"start":215,"end":230},"kind":{"kind":"return","keyword_span":{"file":0,"start":215,"end":221},"value":5,"semicolon_span":{"file":0,"start":229,"end":230}}}],"expressions":[{"span":{"file":0,"start":178,"end":181},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":204,"end":207},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":190,"end":210},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":190,"end":195}},"open_paren_span":{"file":0,"start":195,"end":196},"open_brace_span":{"file":0,"start":196,"end":197},"fields":[{"span":{"file":0,"start":198,"end":207},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":198,"end":202}},"colon_span":{"file":0,"start":202,"end":203},"value":1}}],"close_brace_span":{"file":0,"start":208,"end":209},"close_paren_span":{"file":0,"start":209,"end":210}}},{"span":{"file":0,"start":164,"end":213},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":164,"end":169}},"open_paren_span":{"file":0,"start":169,"end":170},"open_brace_span":{"file":0,"start":170,"end":171},"fields":[{"span":{"file":0,"start":172,"end":181},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":172,"end":176}},"colon_span":{"file":0,"start":176,"end":177},"value":0}},{"span":{"file":0,"start":183,"end":210},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":183,"end":188}},"colon_span":{"file":0,"start":188,"end":189},"value":2}}],"close_brace_span":{"file":0,"start":211,"end":212},"close_paren_span":{"file":0,"start":212,"end":213}}},{"span":{"file":0,"start":222,"end":223},"kind":{"kind":"reference","name":{"text":"o","span":{"file":0,"start":222,"end":223}}}},{"span":{"file":0,"start":222,"end":229},"kind":{"kind":"field-access","base":4,"dot_span":{"file":0,"start":223,"end":224},"field":{"text":"inner","span":{"file":0,"start":224,"end":229}}}}]}}]}],"diagnostics":[]}}"#;
const FIXED_ARRAY_SUBOBJECT_RETURN_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-return.zry");
const FIXED_ARRAY_SUBOBJECT_RETURN_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-return.json");
const PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\ninterface Outer extends ZrynaStruct { inner: Inner; tail: String; }\nfunction make(): Outer { let o: Outer = Outer({ tail: \"b\", inner: Inner({ text: \"a\" }) }); const replacement: Inner = Inner({ text: \"c\" }); o.inner = replacement; return o; }";
const PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE: &str = r#"{"id":814,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":112,"end":118},"kind":{"kind":"string","keyword_span":{"file":0,"start":112,"end":118}}},{"span":{"file":0,"start":139,"end":144},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":139,"end":144}}}},{"span":{"file":0,"start":154,"end":159},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":154,"end":159}}}},{"span":{"file":0,"start":232,"end":237},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":232,"end":237}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}},{"span":{"file":0,"start":54,"end":121},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":54,"end":63},"name":{"text":"Outer","span":{"file":0,"start":64,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":89},"open_brace_span":{"file":0,"start":90,"end":91},"close_brace_span":{"file":0,"start":120,"end":121},"fields":[{"span":{"file":0,"start":92,"end":105},"name":{"text":"inner","span":{"file":0,"start":92,"end":97}},"colon_span":{"file":0,"start":97,"end":98},"semicolon_span":{"file":0,"start":104,"end":105},"type_syntax":1},{"span":{"file":0,"start":106,"end":119},"name":{"text":"tail","span":{"file":0,"start":106,"end":110}},"colon_span":{"file":0,"start":110,"end":111},"semicolon_span":{"file":0,"start":118,"end":119},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":122,"end":296},"export_span":null,"function_span":{"file":0,"start":122,"end":130},"name":{"text":"make","span":{"file":0,"start":131,"end":135}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":145,"end":296},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":296},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":295,"end":296}}],"statements":[{"span":{"file":0,"start":147,"end":212},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":147,"end":150},"mutable":true,"name":{"text":"o","span":{"file":0,"start":151,"end":152}},"type_syntax":4,"equals_span":{"file":0,"start":160,"end":161},"initializer":3,"semicolon_span":{"file":0,"start":211,"end":212}}},{"span":{"file":0,"start":213,"end":261},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":213,"end":218},"mutable":false,"name":{"text":"replacement","span":{"file":0,"start":219,"end":230}},"type_syntax":5,"equals_span":{"file":0,"start":238,"end":239},"initializer":5,"semicolon_span":{"file":0,"start":260,"end":261}}},{"span":{"file":0,"start":262,"end":284},"kind":{"kind":"assignment","target":7,"equals_span":{"file":0,"start":270,"end":271},"value":8,"semicolon_span":{"file":0,"start":283,"end":284}}},{"span":{"file":0,"start":285,"end":294},"kind":{"kind":"return","keyword_span":{"file":0,"start":285,"end":291},"value":9,"semicolon_span":{"file":0,"start":293,"end":294}}}],"expressions":[{"span":{"file":0,"start":176,"end":179},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":202,"end":205},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":188,"end":208},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":188,"end":193}},"open_paren_span":{"file":0,"start":193,"end":194},"open_brace_span":{"file":0,"start":194,"end":195},"fields":[{"span":{"file":0,"start":196,"end":205},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":196,"end":200}},"colon_span":{"file":0,"start":200,"end":201},"value":1}}],"close_brace_span":{"file":0,"start":206,"end":207},"close_paren_span":{"file":0,"start":207,"end":208}}},{"span":{"file":0,"start":162,"end":211},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":162,"end":167}},"open_paren_span":{"file":0,"start":167,"end":168},"open_brace_span":{"file":0,"start":168,"end":169},"fields":[{"span":{"file":0,"start":170,"end":179},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":170,"end":174}},"colon_span":{"file":0,"start":174,"end":175},"value":0}},{"span":{"file":0,"start":181,"end":208},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":181,"end":186}},"colon_span":{"file":0,"start":186,"end":187},"value":2}}],"close_brace_span":{"file":0,"start":209,"end":210},"close_paren_span":{"file":0,"start":210,"end":211}}},{"span":{"file":0,"start":254,"end":257},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":240,"end":260},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":240,"end":245}},"open_paren_span":{"file":0,"start":245,"end":246},"open_brace_span":{"file":0,"start":246,"end":247},"fields":[{"span":{"file":0,"start":248,"end":257},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":248,"end":252}},"colon_span":{"file":0,"start":252,"end":253},"value":4}}],"close_brace_span":{"file":0,"start":258,"end":259},"close_paren_span":{"file":0,"start":259,"end":260}}},{"span":{"file":0,"start":262,"end":263},"kind":{"kind":"reference","name":{"text":"o","span":{"file":0,"start":262,"end":263}}}},{"span":{"file":0,"start":262,"end":269},"kind":{"kind":"field-access","base":6,"dot_span":{"file":0,"start":263,"end":264},"field":{"text":"inner","span":{"file":0,"start":264,"end":269}}}},{"span":{"file":0,"start":272,"end":283},"kind":{"kind":"reference","name":{"text":"replacement","span":{"file":0,"start":272,"end":283}}}},{"span":{"file":0,"start":292,"end":293},"kind":{"kind":"reference","name":{"text":"o","span":{"file":0,"start":292,"end":293}}}}]}}]}],"diagnostics":[]}}"#;
const PROJECTED_SUBOBJECT_ASSIGNMENT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-assignment.zry");
const PROJECTED_SUBOBJECT_ASSIGNMENT_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-assignment.json");
const FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-assignment.zry");
const FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-assignment.json");
const PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-assignment.zry");
const PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-assignment.json");
const FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-clone-assignment.zry");
const FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/fixed-array-subobject-clone-assignment.json");
const PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-after-move.zry");
const PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-after-move.json");
const PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_SOURCE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-with-parameter.zry");
const PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_RESPONSE: &str =
    include_str!("../../../../tests/m3-fixtures/projected-subobject-clone-with-parameter.json");

fn response_snapshot(response: &str) -> RawProjectSyntaxSnapshot {
    let value: serde_json::Value = serde_json::from_str(response).expect("adapter response JSON");
    let result = value.get("result").expect("adapter result");
    decode_snapshot(&serde_json::to_vec(result).expect("snapshot JSON")).expect("v4 snapshot")
}

fn projected_inner_child_after_parent_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = PROJECTED_INNER_MOVE_SOURCE.to_owned();
    let start = source.find("\"c\"").expect("return tail literal");
    source.replace_range(start..start + 3, "o.inner.text");
    let start = u32::try_from(start).expect("tail projection offset");
    let mut raw = shift_snapshot(response_snapshot(PROJECTED_INNER_MOVE_RESPONSE), start, 9);
    let body = &mut raw.files[0].functions[0].body;
    let moved_reference = body.expressions[7].clone();
    let mut outer = body.expressions[8].clone();
    let s = |from, to| zryna_source::UntrustedSpan { file: 0, start: from, end: to };
    body.expressions[6] = RawExpressionSyntax {
        span: s(start, start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "o".to_owned(), span: s(start, start + 1) },
        },
    };
    body.expressions[7] = RawExpressionSyntax {
        span: s(start, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 6,
            dot_span: s(start + 1, start + 2),
            field: RawIdentifierSyntax { text: "inner".to_owned(), span: s(start + 2, start + 7) },
        },
    };
    body.expressions[8] = RawExpressionSyntax {
        span: s(start, start + 12),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 7,
            dot_span: s(start + 7, start + 8),
            field: RawIdentifierSyntax { text: "text".to_owned(), span: s(start + 8, start + 12) },
        },
    };
    let moved = u32::try_from(body.expressions.len()).expect("shifted moved reference id");
    body.expressions.push(moved_reference);
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } = &mut outer.kind
    else {
        panic!("shifted Outer construction")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value: tail, .. } =
        &mut fields[0].kind
    else {
        panic!("explicit tail field")
    };
    *tail = 8;
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value: inner, .. } =
        &mut fields[1].kind
    else {
        panic!("explicit inner field")
    };
    *inner = moved;
    let result = u32::try_from(body.expressions.len()).expect("shifted Outer result id");
    body.expressions.push(outer);
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("shifted return")
    };
    *value = result;
    (source, raw)
}

fn clone_final_return_snapshot(source: &str, response: &str) -> (String, RawProjectSyntaxSnapshot) {
    let raw = response_snapshot(response);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: reference_value, .. } =
        body.statements.last().expect("return").kind
    else {
        panic!("return")
    };
    let reference = body.expressions[reference_value as usize].clone();
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &reference.kind else {
        panic!("final reference")
    };
    let start = reference.span.start;
    let end = reference.span.end;
    let mut updated_source = source.to_owned();
    updated_source.replace_range(
        usize::try_from(start).expect("start")..usize::try_from(end).expect("end"),
        &format!("clone({})", name.text),
    );
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let body = &mut raw.files[0].functions[0].body;
    let reference = &mut body.expressions[reference_value as usize];
    reference.span.end -= 1;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut reference.kind else {
        panic!("shifted final reference")
    };
    name.span.end -= 1;
    let new_value = u32::try_from(body.expressions.len()).expect("expression id");
    let RawStatementKind::Return { value, .. } =
        &mut body.statements.last_mut().expect("return").kind
    else {
        panic!("return")
    };
    *value = new_value;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: reference_value,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (updated_source, raw)
}

fn exclude_inserted_close(value: &mut serde_json::Value, close: u32) {
    match value {
        serde_json::Value::Object(object)
            if object.contains_key("file")
                && object.contains_key("start")
                && object.contains_key("end") =>
        {
            let from = object["start"].as_u64().expect("span start");
            let to = object["end"].as_u64().expect("span end");
            if from < u64::from(close) && to == u64::from(close + 1) {
                object.insert("end".to_owned(), serde_json::Value::from(close));
            }
        }
        serde_json::Value::Object(object) => {
            for child in object.values_mut() {
                exclude_inserted_close(child, close);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                exclude_inserted_close(child, close);
            }
        }
        _ => {}
    }
}

fn projected_aggregate_clone_local_snapshot(
    source: &str,
    response: &str,
) -> (String, RawProjectSyntaxSnapshot) {
    let raw = response_snapshot(response);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { initializer: operand_id, .. } =
        body.statements[1].kind
    else {
        panic!("projected aggregate local")
    };
    let operand = body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut updated_source = source.to_owned();
    let spelling = updated_source
        .get(
            usize::try_from(start).expect("operand start")
                ..usize::try_from(end).expect("operand end"),
        )
        .expect("projected operand")
        .to_owned();
    updated_source.replace_range(
        usize::try_from(start).expect("operand start")..usize::try_from(end).expect("operand end"),
        &format!("clone({spelling})"),
    );
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("clone operand snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("projected aggregate local")
    };
    *initializer = clone;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (updated_source, raw)
}

fn projected_aggregate_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let mut raw = response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE);
    let RawStatementKind::Assignment { value: operand_id, .. } =
        raw.files[0].functions[0].body.statements[2].kind
    else {
        panic!("projected aggregate assignment")
    };
    let operand = raw.files[0].functions[0].body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut source = PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE.to_owned();
    let spelling = source
        .get(
            usize::try_from(start).expect("operand start")
                ..usize::try_from(end).expect("operand end"),
        )
        .expect("assignment operand")
        .to_owned();
    source.replace_range(
        usize::try_from(start).expect("operand start")..usize::try_from(end).expect("operand end"),
        &format!("clone({spelling})"),
    );
    raw = shift_snapshot(raw, start, 6);
    raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("clone assignment snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("projected aggregate assignment")
    };
    *value = clone;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (source, raw)
}

fn two_projected_aggregate_clone_sites_snapshot(
    local_before_assignment: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str = "const copy: Inner = clone(src.inner); ";
    let mut raw = response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE);
    let insertion_statement = if local_before_assignment { 2 } else { 3 };
    let insertion = raw.files[0].functions[0].body.statements[insertion_statement].span.start;
    let mut source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.to_owned();
    source.insert_str(usize::try_from(insertion).expect("insertion offset"), INSERT);
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("inserted declaration length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let inner_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("projected clone local type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(12, 17),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Inner".to_owned(), span: s(12, 17) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_reference = u32::try_from(body.expressions.len()).expect("source reference");
    body.expressions.push(RawExpressionSyntax {
        span: s(26, 29),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "src".to_owned(), span: s(26, 29) },
        },
    });
    let source_projection = u32::try_from(body.expressions.len()).expect("source projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(26, 35),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: source_reference,
            dot_span: s(29, 30),
            field: RawIdentifierSyntax { text: "inner".to_owned(), span: s(30, 35) },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(20, 36),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(20, 25),
            open_paren_span: s(25, 26),
            value: source_projection,
            close_paren_span: s(35, 36),
        },
    });
    body.statements.insert(
        insertion_statement,
        RawStatementSyntax {
            span: s(0, 37),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "copy".to_owned(), span: s(6, 10) },
                type_syntax: inner_type,
                equals_span: s(18, 19),
                initializer: cloned,
                semicolon_span: s(36, 37),
            },
        },
    );
    body.blocks[0].statements = (0..body.statements.len())
        .map(|index| u32::try_from(index).expect("statement id"))
        .collect();
    (source, raw)
}

fn projected_aggregate_clone_direct_return_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let mut raw = response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE);
    let RawStatementKind::Return { value: operand_id, .. } =
        raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("projected aggregate return")
    };
    let operand = raw.files[0].functions[0].body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    let spelling = source
        .get(
            usize::try_from(start).expect("return start")
                ..usize::try_from(end).expect("return end"),
        )
        .expect("projected return operand")
        .to_owned();
    source.replace_range(
        usize::try_from(start).expect("return start")..usize::try_from(end).expect("return end"),
        &format!("clone({spelling})"),
    );
    raw = shift_snapshot(raw, start, 6);
    raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("direct clone snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("projected aggregate return")
    };
    *value = clone;
    (source, raw)
}

fn projected_aggregate_direct_return_with_parameter_snapshot() -> (String, RawProjectSyntaxSnapshot)
{
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    let insertion = u32::try_from(source.find("()").expect("empty parameter list") + 1)
        .expect("parameter insertion");
    source.insert_str(usize::try_from(insertion).expect("parameter insertion"), "flag: i32");
    let mut raw =
        shift_snapshot(response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE), insertion, 9);
    let file = &mut raw.files[0];
    file.type_syntax.insert(
        3,
        RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: insertion + 6, end: insertion + 9 },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 6,
                        end: insertion + 9,
                    },
                },
            },
        },
    );
    let function = &mut file.functions[0];
    function.result_type += 1;
    function.parameters.push(RawParameterSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 9 },
        name: RawIdentifierSyntax {
            text: "flag".to_owned(),
            span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 4 },
        },
        type_syntax: 3,
    });
    let RawStatementKind::LocalDeclaration { type_syntax, .. } =
        &mut function.body.statements[0].kind
    else {
        panic!("outer local")
    };
    *type_syntax += 1;
    (source, raw)
}

#[derive(Clone, Copy)]
enum OwnedPairAssignmentRhs {
    Fresh,
    CloneTarget,
    SelfMove,
}

#[allow(clippy::too_many_lines)]
fn owned_pair_assignment_snapshot(
    rhs: OwnedPairAssignmentRhs,
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let assignment = match rhs {
        OwnedPairAssignmentRhs::Fresh => "p = OwnedPair({ flag: false, first: \"b\" }); ",
        OwnedPairAssignmentRhs::CloneTarget => "p = clone(p); ",
        OwnedPairAssignmentRhs::SelfMove => "p = p; ",
    };
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    if mutable {
        source.replace_range(100..105, "let  ");
    }
    let insertion = source.find("return p;").expect("return insertion");
    source.insert_str(insertion, assignment);
    let insertion = u32::try_from(insertion).expect("fixture insertion");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(assignment.len()).expect("assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable: is_mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    if mutable {
        keyword_span.end = keyword_span.start + 3;
        *is_mutable = true;
    }
    let target = u32::try_from(body.expressions.len()).expect("target expression");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
            },
        },
    });
    let value = match rhs {
        OwnedPairAssignmentRhs::Fresh => {
            let bool_value = u32::try_from(body.expressions.len()).expect("bool value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 22,
                    end: insertion + 27,
                },
                kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
            });
            let string_value = u32::try_from(body.expressions.len()).expect("String value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 36,
                    end: insertion + 39,
                },
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"b\"".to_owned(),
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("Struct value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 42,
                },
                kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
                    type_name: RawIdentifierSyntax {
                        text: "OwnedPair".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 4,
                            end: insertion + 13,
                        },
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 13,
                        end: insertion + 14,
                    },
                    open_brace_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 14,
                        end: insertion + 15,
                    },
                    fields: vec![
                        zryna_syntax::v4::RawFieldInitializer {
                            span: zryna_source::UntrustedSpan {
                                file: 0,
                                start: insertion + 16,
                                end: insertion + 27,
                            },
                            kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                                name: RawIdentifierSyntax {
                                    text: "flag".to_owned(),
                                    span: zryna_source::UntrustedSpan {
                                        file: 0,
                                        start: insertion + 16,
                                        end: insertion + 20,
                                    },
                                },
                                colon_span: zryna_source::UntrustedSpan {
                                    file: 0,
                                    start: insertion + 20,
                                    end: insertion + 21,
                                },
                                value: bool_value,
                            },
                        },
                        zryna_syntax::v4::RawFieldInitializer {
                            span: zryna_source::UntrustedSpan {
                                file: 0,
                                start: insertion + 29,
                                end: insertion + 39,
                            },
                            kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                                name: RawIdentifierSyntax {
                                    text: "first".to_owned(),
                                    span: zryna_source::UntrustedSpan {
                                        file: 0,
                                        start: insertion + 29,
                                        end: insertion + 34,
                                    },
                                },
                                colon_span: zryna_source::UntrustedSpan {
                                    file: 0,
                                    start: insertion + 34,
                                    end: insertion + 35,
                                },
                                value: string_value,
                            },
                        },
                    ],
                    close_brace_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 40,
                        end: insertion + 41,
                    },
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 41,
                        end: insertion + 42,
                    },
                },
            });
            value
        }
        OwnedPairAssignmentRhs::CloneTarget => {
            let source_value = u32::try_from(body.expressions.len()).expect("clone source");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 10,
                    end: insertion + 11,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 10,
                            end: insertion + 11,
                        },
                    },
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("clone value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 12,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Clone {
                    keyword_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 4,
                        end: insertion + 9,
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 9,
                        end: insertion + 10,
                    },
                    value: source_value,
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 11,
                        end: insertion + 12,
                    },
                },
            });
            value
        }
        OwnedPairAssignmentRhs::SelfMove => {
            let value = u32::try_from(body.expressions.len()).expect("self-move value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 5,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 4,
                            end: insertion + 5,
                        },
                    },
                },
            });
            value
        }
    };
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion,
                end: insertion + u32::try_from(assignment.trim_end().len()).expect("statement"),
            },
            kind: RawStatementKind::Assignment {
                target,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 2,
                    end: insertion + 3,
                },
                value,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion
                        + u32::try_from(assignment.trim_end().len() - 1).expect("semicolon"),
                    end: insertion + u32::try_from(assignment.trim_end().len()).expect("semicolon"),
                },
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[derive(Clone, Copy)]
enum OwnedPairProjectedStringAssignmentRhs {
    Fresh,
    TargetMove,
}

fn owned_pair_projected_string_assignment_snapshot(
    rhs: OwnedPairProjectedStringAssignmentRhs,
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let assignment = match rhs {
        OwnedPairProjectedStringAssignmentRhs::Fresh => "p.first = \"b\"; ",
        OwnedPairProjectedStringAssignmentRhs::TargetMove => "p.first = p.first; ",
    };
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    if mutable {
        source.replace_range(100..105, "let  ");
    }
    let insertion = source.find("return p;").expect("projected assignment insertion");
    source.insert_str(insertion, assignment);
    let insertion = u32::try_from(insertion).expect("projected assignment offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(assignment.len()).expect("projected assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable: is_mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    if mutable {
        keyword_span.end = keyword_span.start + 3;
        *is_mutable = true;
    }
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let target_base = u32::try_from(body.expressions.len()).expect("target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: target_base,
            dot_span: s(insertion + 1, insertion + 2),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 2, insertion + 7),
            },
        },
    });
    let value = match rhs {
        OwnedPairProjectedStringAssignmentRhs::Fresh => {
            let value = u32::try_from(body.expressions.len()).expect("fresh String value");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 13),
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"b\"".to_owned(),
                },
            });
            value
        }
        OwnedPairProjectedStringAssignmentRhs::TargetMove => {
            let base = u32::try_from(body.expressions.len()).expect("RHS projection base");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 11),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: s(insertion + 10, insertion + 11),
                    },
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("RHS projection");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 17),
                kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                    base,
                    dot_span: s(insertion + 11, insertion + 12),
                    field: RawIdentifierSyntax {
                        text: "first".to_owned(),
                        span: s(insertion + 12, insertion + 17),
                    },
                },
            });
            value
        }
    };
    let statement_end = insertion + u32::try_from(assignment.trim_end().len()).expect("statement");
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, statement_end),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 8, insertion + 9),
                value,
                semicolon_span: s(statement_end - 1, statement_end),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

fn owned_pair_projected_string_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let start = source.find("\"b\"").expect("projected clone operand");
    let replacement = "clone(p.first)";
    source.replace_range(start..start + 3, replacement);
    let start = u32::try_from(start).expect("projected clone offset");
    raw = shift_snapshot(raw, start + 3, 11);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let RawStatementKind::Assignment { value, .. } = body.statements[1].kind else {
        panic!("projected clone assignment")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(start + 6, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 6, start + 7) },
        },
    };
    let projection = u32::try_from(body.expressions.len()).expect("clone projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 6, start + 13),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: value,
            dot_span: s(start + 7, start + 8),
            field: RawIdentifierSyntax { text: "first".to_owned(), span: s(start + 8, start + 13) },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, start + 14),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(start, start + 5),
            open_paren_span: s(start + 5, start + 6),
            value: projection,
            close_paren_span: s(start + 13, start + 14),
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("projected clone assignment")
    };
    *value = cloned;
    (source, raw)
}

fn owned_pair_copy_projection_clone_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let clone_start = source.find("clone(p.first)").expect("copy clone expression");
    let field_start = clone_start + 8;
    let field_end = field_start + 5;
    source.replace_range(field_start..field_end, "flag");
    let mut raw = shift_snapshot_signed(raw, u32::try_from(field_end).expect("copy field end"), -1);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { value: cloned, .. } = body.statements[1].kind else {
        panic!("copy clone assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::Clone { value: projection, .. } =
        body.expressions[cloned as usize].kind
    else {
        panic!("copy clone expression")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { field, .. } =
        &mut body.expressions[projection as usize].kind
    else {
        panic!("copy clone projection")
    };
    field.text = "flag".to_owned();
    (source, raw)
}

fn owned_pair_copy_projection_assignment_target_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let start = source.find("p.first =").expect("projected assignment target");
    let replacement = "p.flag";
    source.replace_range(start..start + 7, replacement);
    let start = u32::try_from(start).expect("invalid target offset");
    raw = shift_snapshot_signed(
        raw,
        start + 7,
        i32::try_from(replacement.len()).expect("small target") - 7,
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { target, .. } = body.statements[1].kind else {
        panic!("projected assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { base, .. } =
        body.expressions[target as usize].kind
    else {
        panic!("projected target")
    };
    body.expressions[target as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 6 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: start + 1, end: start + 2 },
            field: RawIdentifierSyntax {
                text: "flag".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 2, end: start + 6 },
            },
        },
    };
    (source, raw)
}

fn owned_array_projected_string_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "a[0] = \"c\"; ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let mutable = source.find("const a").expect("owned array local");
    source.replace_range(mutable..mutable + 5, "let  ");
    let insertion = source.find("return a;").expect("array assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("array assignment offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("array assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned array local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let base = u32::try_from(body.expressions.len()).expect("array target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let index = u32::try_from(body.expressions.len()).expect("array target index");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 2, insertion + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let target = u32::try_from(body.expressions.len()).expect("array target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base,
            open_bracket_span: s(insertion + 1, insertion + 2),
            index,
            close_bracket_span: s(insertion + 3, insertion + 4),
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("array replacement String");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 7, insertion + 10),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"c\"".to_owned() },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 11),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 5, insertion + 6),
                value,
                semicolon_span: s(insertion + 10, insertion + 11),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

fn owned_array_projected_string_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_array_projected_string_assignment_snapshot();
    let start = source.find("\"c\"").expect("array clone operand");
    let replacement = "clone(a[0])";
    source.replace_range(start..start + 3, replacement);
    let start = u32::try_from(start).expect("array clone offset");
    raw = shift_snapshot(raw, start + 3, 8);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let RawStatementKind::Assignment { value, .. } = body.statements[1].kind else {
        panic!("array clone assignment")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(start + 6, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(start + 6, start + 7) },
        },
    };
    let index = u32::try_from(body.expressions.len()).expect("clone array index");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 8, start + 9),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let projection = u32::try_from(body.expressions.len()).expect("clone array projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 6, start + 10),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: value,
            open_bracket_span: s(start + 7, start + 8),
            index,
            close_bracket_span: s(start + 9, start + 10),
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected array clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, start + 11),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(start, start + 5),
            open_paren_span: s(start + 5, start + 6),
            value: projection,
            close_paren_span: s(start + 10, start + 11),
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("array clone assignment")
    };
    *value = cloned;
    (source, raw)
}

#[derive(Clone, Copy)]
enum OwnedPairProjectionAssignmentRhs {
    CopyField,
    MoveField,
}

fn owned_pair_projection_assignment_snapshot(
    rhs: OwnedPairProjectionAssignmentRhs,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::Fresh, true);
    let (old, replacement) = match rhs {
        OwnedPairProjectionAssignmentRhs::CopyField => ("false", "p.flag"),
        OwnedPairProjectionAssignmentRhs::MoveField => ("\"b\"", "p.first"),
    };
    let start = source.find(old).expect("projected assignment operand");
    source.replace_range(start..start + old.len(), replacement);
    let start = u32::try_from(start).expect("projected operand offset");
    let delta = i32::try_from(replacement.len()).expect("replacement length")
        - i32::try_from(old.len()).expect("old length");
    let mut raw = shift_snapshot_signed(
        raw,
        start + u32::try_from(old.len()).expect("old operand end"),
        delta,
    );
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    match rhs {
        OwnedPairProjectionAssignmentRhs::CopyField => {
            body.expressions[5] = RawExpressionSyntax {
                span: s(start, start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start, start + 1) },
                },
            };
            body.expressions.insert(
                6,
                RawExpressionSyntax {
                    span: s(start, start + 6),
                    kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                        base: 5,
                        dot_span: s(start + 1, start + 2),
                        field: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(start + 2, start + 6),
                        },
                    },
                },
            );
            let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
                &mut body.expressions[8].kind
            else {
                panic!("projected assignment Struct")
            };
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[0].kind
            else {
                panic!("flag initializer")
            };
            *value = 6;
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[1].kind
            else {
                panic!("first initializer")
            };
            *value = 7;
        }
        OwnedPairProjectionAssignmentRhs::MoveField => {
            body.expressions[6] = RawExpressionSyntax {
                span: s(start, start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start, start + 1) },
                },
            };
            body.expressions.insert(
                7,
                RawExpressionSyntax {
                    span: s(start, start + 7),
                    kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                        base: 6,
                        dot_span: s(start + 1, start + 2),
                        field: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(start + 2, start + 7),
                        },
                    },
                },
            );
            let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
                &mut body.expressions[8].kind
            else {
                panic!("projected assignment Struct")
            };
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[1].kind
            else {
                panic!("first initializer")
            };
            *value = 7;
        }
    }
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("projected aggregate assignment")
    };
    *value = 8;
    (source, raw)
}

fn owned_enum_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let source = OWNED_ENUM_STRING_SOURCE
        .replacen("const x", "let   x", 1)
        .replacen("const y: Maybe = x;", "x = Maybe.none();  ", 1)
        .replacen("return y", "return x", 1);
    let mut raw = response_snapshot(OWNED_ENUM_STRING_RESPONSE);
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    raw.files[0].type_syntax.pop();
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[1].kind
    else {
        panic!("enum target local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let assignment = u32::try_from(source.find("x = Maybe.none()").expect("enum assignment"))
        .expect("enum assignment span");
    body.expressions[3] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment, end: assignment + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment,
                    end: assignment + 1,
                },
            },
        },
    };
    let replacement = u32::try_from(body.expressions.len()).expect("enum replacement");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment + 4, end: assignment + 16 },
        kind: zryna_syntax::v4::RawExpressionKind::EnumConstruction {
            type_name: RawIdentifierSyntax {
                text: "Maybe".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment + 4,
                    end: assignment + 9,
                },
            },
            dot_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 9,
                end: assignment + 10,
            },
            variant: RawIdentifierSyntax {
                text: "none".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment + 10,
                    end: assignment + 14,
                },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 14,
                end: assignment + 15,
            },
            payload: None,
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 15,
                end: assignment + 16,
            },
        },
    });
    body.statements[2] = RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment, end: assignment + 17 },
        kind: RawStatementKind::Assignment {
            target: 3,
            equals_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 2,
                end: assignment + 3,
            },
            value: replacement,
            semicolon_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 16,
                end: assignment + 17,
            },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut body.expressions[4].kind
    else {
        panic!("enum return")
    };
    name.text = "x".to_owned();
    (source, raw)
}

fn owned_fixed_array_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "a = clone(a); ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    source.replace_range(41..46, "let  ");
    let insertion = source.find("return a;").expect("array return insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("array insertion");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("array assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned array local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let target = u32::try_from(body.expressions.len()).expect("array target");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
            },
        },
    });
    let source_value = u32::try_from(body.expressions.len()).expect("array clone source");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion + 10, end: insertion + 11 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 10,
                    end: insertion + 11,
                },
            },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("array clone");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion + 4, end: insertion + 12 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 4,
                end: insertion + 9,
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 9,
                end: insertion + 10,
            },
            value: source_value,
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 11,
                end: insertion + 12,
            },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 13 },
            kind: RawStatementKind::Assignment {
                target,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 2,
                    end: insertion + 3,
                },
                value: cloned,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 12,
                    end: insertion + 13,
                },
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

fn owned_pair_projected_return_snapshot(field: &str) -> (String, RawProjectSyntaxSnapshot) {
    let replacement = format!("OwnedPair({{ flag: p.flag, first: p.{field} }})");
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    let start = source.rfind("p;").expect("Pair return value");
    source.replace_range(start..=start, &replacement);
    let start = u32::try_from(start).expect("Pair return offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        start + 1,
        u32::try_from(replacement.len() - 1).expect("Pair replacement length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[3] = RawExpressionSyntax {
        span: s(start + 18, start + 19),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 18, start + 19) },
        },
    };
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 18, start + 24),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 3,
            dot_span: s(start + 19, start + 20),
            field: RawIdentifierSyntax { text: "flag".to_owned(), span: s(start + 20, start + 24) },
        },
    });
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 33, start + 34),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 33, start + 34) },
        },
    });
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 33, start + 34 + u32::try_from(field.len()).expect("field length") + 1),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 5,
            dot_span: s(start + 34, start + 35),
            field: RawIdentifierSyntax {
                text: field.to_owned(),
                span: s(start + 35, start + 35 + u32::try_from(field.len()).expect("field length")),
            },
        },
    });
    let end = start + u32::try_from(replacement.len()).expect("Pair replacement end");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(start, start + 9),
            },
            open_paren_span: s(start + 9, start + 10),
            open_brace_span: s(start + 10, start + 11),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(start + 12, start + 24),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(start + 12, start + 16),
                        },
                        colon_span: s(start + 16, start + 17),
                        value: 4,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(start + 26, end - 3),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(start + 26, start + 31),
                        },
                        colon_span: s(start + 31, start + 32),
                        value: 6,
                    },
                },
            ],
            close_brace_span: s(end - 2, end - 1),
            close_paren_span: s(end - 1, end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("Pair return")
    };
    *value = 7;
    (source, raw)
}

#[derive(Clone, Copy)]
enum OwnedArrayProjectionCase {
    Disjoint,
    Repeat,
    Dynamic,
    Negative,
    OutOfBounds,
}

#[allow(clippy::too_many_lines)]
fn owned_array_projected_return_snapshot(
    case: OwnedArrayProjectionCase,
) -> (String, RawProjectSyntaxSnapshot) {
    let indexes = match case {
        OwnedArrayProjectionCase::Disjoint => ("0", "1"),
        OwnedArrayProjectionCase::Repeat => ("0", "0"),
        OwnedArrayProjectionCase::Dynamic => ("a", "1"),
        OwnedArrayProjectionCase::Negative => ("-1", "1"),
        OwnedArrayProjectionCase::OutOfBounds => ("2", "1"),
    };
    let replacement = format!("FixedArray<String, 2>([a[{}], a[{}]])", indexes.0, indexes.1);
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let start = source.rfind("a;").expect("array return value");
    source.replace_range(start..=start, &replacement);
    let start = u32::try_from(start).expect("array return offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        start + 1,
        u32::try_from(replacement.len() - 1).expect("array replacement length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(start + 11, start + 17),
        kind: RawTypeSyntaxKind::String { keyword_span: s(start + 11, start + 17) },
    });
    let array_type = u32::try_from(raw.files[0].type_syntax.len()).expect("array type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(start, start + 21),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(start, start + 10),
            less_than_span: s(start + 10, start + 11),
            element: string_type,
            comma_span: s(start + 17, start + 18),
            length_span: s(start + 19, start + 20),
            length_spelling: "2".to_owned(),
            length: 2,
            greater_than_span: s(start + 20, start + 21),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let first_base_start = start + 23;
    body.expressions[3] = RawExpressionSyntax {
        span: s(first_base_start, first_base_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(first_base_start, first_base_start + 1),
            },
        },
    };
    let first_index_start = first_base_start + 2;
    let first_index = match case {
        OwnedArrayProjectionCase::Dynamic => {
            let id = u32::try_from(body.expressions.len()).expect("dynamic index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "a".to_owned(),
                        span: s(first_index_start, first_index_start + 1),
                    },
                },
            });
            id
        }
        OwnedArrayProjectionCase::Negative => {
            let literal = u32::try_from(body.expressions.len()).expect("negative literal id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start + 1, first_index_start + 2),
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
            });
            let id = u32::try_from(body.expressions.len()).expect("negative index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 2),
                kind: zryna_syntax::v4::RawExpressionKind::Negation {
                    operator_span: s(first_index_start, first_index_start + 1),
                    operand: literal,
                },
            });
            id
        }
        _ => {
            let id = u32::try_from(body.expressions.len()).expect("constant index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                    spelling: indexes.0.to_owned(),
                },
            });
            id
        }
    };
    let first_index_len = u32::try_from(indexes.0.len()).expect("first index length");
    let first_projection = u32::try_from(body.expressions.len()).expect("first projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(first_base_start, first_index_start + first_index_len + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 3,
            open_bracket_span: s(first_base_start + 1, first_base_start + 2),
            index: first_index,
            close_bracket_span: s(
                first_index_start + first_index_len,
                first_index_start + first_index_len + 1,
            ),
        },
    });
    let second_base_start = first_index_start + first_index_len + 3;
    let second_base = u32::try_from(body.expressions.len()).expect("second base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start, second_base_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(second_base_start, second_base_start + 1),
            },
        },
    });
    let second_index = u32::try_from(body.expressions.len()).expect("second index id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start + 2, second_base_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: indexes.1.to_owned() },
    });
    let second_projection = u32::try_from(body.expressions.len()).expect("second projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start, second_base_start + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: second_base,
            open_bracket_span: s(second_base_start + 1, second_base_start + 2),
            index: second_index,
            close_bracket_span: s(second_base_start + 3, second_base_start + 4),
        },
    });
    let end = start + u32::try_from(replacement.len()).expect("array replacement end");
    let result = u32::try_from(body.expressions.len()).expect("array result id");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, end),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: array_type,
            open_paren_span: s(start + 21, start + 22),
            open_bracket_span: s(start + 22, start + 23),
            elements: vec![first_projection, second_projection],
            close_bracket_span: s(end - 2, end - 1),
            close_paren_span: s(end - 1, end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array return")
    };
    *value = result;
    (source, raw)
}

fn owned_array_projected_clone_return_snapshot(
    case: OwnedArrayProjectionCase,
    ordinal: usize,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_array_projected_return_snapshot(case);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: result, .. } = body.statements[1].kind else {
        panic!("array clone return")
    };
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &body.expressions[result as usize].kind
    else {
        panic!("array clone construction")
    };
    let projection = elements[ordinal];
    let projection_span = body.expressions[projection as usize].span;
    let start = projection_span.start;
    let end = projection_span.end;
    source.insert_str(usize::try_from(start).expect("clone start"), "clone(");
    source.insert(usize::try_from(end + 6).expect("clone end"), ')');
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let body = &mut raw.files[0].functions[0].body;
    let projected = &mut body.expressions[projection as usize];
    projected.span.end -= 1;
    let zryna_syntax::v4::RawExpressionKind::Index { close_bracket_span, .. } = &mut projected.kind
    else {
        panic!("array clone projection")
    };
    close_bracket_span.end -= 1;
    assert_eq!(result as usize + 1, body.expressions.len());
    let mut construction = body.expressions.pop().expect("array clone construction");
    let cloned = u32::try_from(body.expressions.len()).expect("array clone expression");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: projection,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
        open_bracket_span,
        elements,
        ..
    } = &mut construction.kind
    else {
        panic!("array clone construction")
    };
    if ordinal == 0 {
        open_bracket_span.end -= 6;
    }
    elements[ordinal] = cloned;
    let rebuilt = u32::try_from(body.expressions.len()).expect("rebuilt array construction");
    body.expressions.push(construction);
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array clone return")
    };
    *value = rebuilt;
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn owned_array_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCALS: &str = "const text: String = a[0]; const b: FixedArray<String, 2> = a; ";
    let (mut source, mut raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Disjoint, 1);
    let insertion = source.find("return FixedArray").expect("array transfer insertion");
    source.insert_str(insertion, LOCALS);
    let insertion = u32::try_from(insertion).expect("array transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCALS.len()).expect("array transfer locals length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let element_type = u32::try_from(raw.files[0].type_syntax.len()).expect("element type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 47, insertion + 53),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 47, insertion + 53) },
    });
    let array_type = u32::try_from(raw.files[0].type_syntax.len()).expect("array type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 36, insertion + 57),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(insertion + 36, insertion + 46),
            less_than_span: s(insertion + 46, insertion + 47),
            element: element_type,
            comma_span: s(insertion + 53, insertion + 54),
            length_span: s(insertion + 55, insertion + 56),
            length_spelling: "2".to_owned(),
            length: 2,
            greater_than_span: s(insertion + 56, insertion + 57),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: result, .. } = body.statements[1].kind else {
        panic!("array transfer return")
    };
    assert_eq!(result as usize + 1, body.expressions.len());
    let mut construction = body.expressions.pop().expect("array transfer construction");
    let (first_result, second_clone) = match &construction.kind {
        zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } => {
            (elements[0], elements[1])
        }
        _ => panic!("array transfer result"),
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base: old_base, index: old_index, .. } =
        body.expressions[first_result as usize].kind
    else {
        panic!("first array result projection")
    };
    let first_result_span = body.expressions[first_result as usize].span;
    body.expressions[old_base as usize] = RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    };
    body.expressions[old_index as usize] = RawExpressionSyntax {
        span: s(insertion + 23, insertion + 24),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    body.expressions[first_result as usize] = RawExpressionSyntax {
        span: s(insertion + 21, insertion + 25),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: old_base,
            open_bracket_span: s(insertion + 22, insertion + 23),
            index: old_index,
            close_bracket_span: s(insertion + 24, insertion + 25),
        },
    };
    let first_start = usize::try_from(first_result_span.start).expect("first result offset");
    source.replace_range(first_start..first_start + 4, "text");
    let return_text = u32::try_from(body.expressions.len()).expect("return text id");
    body.expressions.push(RawExpressionSyntax {
        span: first_result_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "text".to_owned(), span: first_result_span },
        },
    });
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut construction.kind
    else {
        unreachable!("array transfer result already matched")
    };
    elements[0] = return_text;
    let transfer_source = u32::try_from(body.expressions.len()).expect("transfer source id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 60, insertion + 61),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 60, insertion + 61),
            },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 26),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "text".to_owned(),
                    span: s(insertion + 6, insertion + 10),
                },
                type_syntax: string_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: first_result,
                semicolon_span: s(insertion + 25, insertion + 26),
            },
        },
    );
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion + 27, insertion + 62),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion + 27, insertion + 32),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "b".to_owned(),
                    span: s(insertion + 33, insertion + 34),
                },
                type_syntax: array_type,
                equals_span: s(insertion + 58, insertion + 59),
                initializer: transfer_source,
                semicolon_span: s(insertion + 61, insertion + 62),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];
    let zryna_syntax::v4::RawExpressionKind::Clone { value: cloned, .. } =
        body.expressions[second_clone as usize].kind
    else {
        panic!("second array result clone")
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base: second_base, .. } =
        body.expressions[cloned as usize].kind
    else {
        panic!("second array result projection")
    };
    let second_span = body.expressions[second_base as usize].span;
    let second_start = usize::try_from(second_span.start).expect("second result offset");
    source.replace_range(second_start..=second_start, "b");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut body.expressions[second_base as usize].kind
    else {
        panic!("second array result base")
    };
    name.text = "b".to_owned();
    let rebuilt = u32::try_from(body.expressions.len()).expect("rebuilt array transfer result");
    body.expressions.push(construction);
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("array transfer return")
    };
    *value = rebuilt;
    (source, raw)
}

fn owned_pair_partial_then_root_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const text: String = p.first; ";
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    let insertion = source.find("return p;").expect("Pair return insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("Pair insertion offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(LOCAL.len()).expect("projected local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let body = &mut raw.files[0].functions[0].body;
    let base = u32::try_from(body.expressions.len()).expect("projected base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    let projected = u32::try_from(body.expressions.len()).expect("projected value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 28),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: s(insertion + 22, insertion + 23),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 23, insertion + 28),
            },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 29),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "text".to_owned(),
                    span: s(insertion + 6, insertion + 10),
                },
                type_syntax: string_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: projected,
                semicolon_span: s(insertion + 28, insertion + 29),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn owned_pair_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str = "let q: OwnedPair = OwnedPair({ flag: false, first: \"old\" }); q = p; ";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let insertion = source.find("return p;").expect("partial assignment insertion");
    source.insert_str(insertion, INSERT);
    let return_value = source.rfind("p;").expect("partial assignment return value");
    source.replace_range(return_value..=return_value, "q");
    let insertion = u32::try_from(insertion).expect("partial assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("partial assignment length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(7, 16),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "OwnedPair".to_owned(), span: s(7, 16) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let flag = u32::try_from(body.expressions.len()).expect("assignment flag value");
    body.expressions.push(RawExpressionSyntax {
        span: s(37, 42),
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
    });
    let old = u32::try_from(body.expressions.len()).expect("assignment old String value");
    body.expressions.push(RawExpressionSyntax {
        span: s(51, 56),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"old\"".to_owned() },
    });
    let target_initializer =
        u32::try_from(body.expressions.len()).expect("assignment target initializer");
    body.expressions.push(RawExpressionSyntax {
        span: s(19, 59),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax { text: "OwnedPair".to_owned(), span: s(19, 28) },
            open_paren_span: s(28, 29),
            open_brace_span: s(29, 30),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(31, 42),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax { text: "flag".to_owned(), span: s(31, 35) },
                        colon_span: s(35, 36),
                        value: flag,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(44, 56),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax { text: "first".to_owned(), span: s(44, 49) },
                        colon_span: s(49, 50),
                        value: old,
                    },
                },
            ],
            close_brace_span: s(57, 58),
            close_paren_span: s(58, 59),
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("assignment target value");
    body.expressions.push(RawExpressionSyntax {
        span: s(61, 62),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "q".to_owned(), span: s(61, 62) },
        },
    });
    let partial_source = u32::try_from(body.expressions.len()).expect("partial assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(65, 66),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(65, 66) },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(0, 60),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 3),
                mutable: true,
                name: RawIdentifierSyntax { text: "q".to_owned(), span: s(4, 5) },
                type_syntax: pair_type,
                equals_span: s(17, 18),
                initializer: target_initializer,
                semicolon_span: s(59, 60),
            },
        },
    );
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(61, 67),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(63, 64),
                value: partial_source,
                semicolon_span: s(66, 67),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial assignment return")
    };
    let return_value = u32::try_from(return_value).expect("partial assignment return offset");
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_value, end: return_value + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: return_value,
                    end: return_value + 1,
                },
            },
        },
    };
    (source, raw)
}

fn owned_pair_partial_assignment_old_source_return_snapshot() -> (String, RawProjectSyntaxSnapshot)
{
    let (mut source, mut raw) = owned_pair_partial_assignment_snapshot();
    let start = source.rfind("q;").expect("assigned-root return");
    source.replace_range(start..=start, "p");
    let start = u32::try_from(start).expect("old-source return offset");
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial assignment return")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    (source, raw)
}

fn owned_pair_partial_self_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_partial_assignment_snapshot();
    let start = source.find("q = p;").expect("partial assignment target");
    source.replace_range(start..=start, "p");
    let start = u32::try_from(start).expect("partial self-assignment offset");
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { target, .. } = body.statements[3].kind else {
        panic!("partial assignment statement")
    };
    body.expressions[target as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    (source, raw)
}

fn owned_array_partial_then_root_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const text: String = a[0]; ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let insertion = source.find("return a;").expect("array return insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("array insertion offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(LOCAL.len()).expect("array projected local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let body = &mut raw.files[0].functions[0].body;
    let base = u32::try_from(body.expressions.len()).expect("array projected base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    let index = u32::try_from(body.expressions.len()).expect("array projected index id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 23, insertion + 24),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let projected = u32::try_from(body.expressions.len()).expect("array projected value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 25),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base,
            open_bracket_span: s(insertion + 22, insertion + 23),
            index,
            close_bracket_span: s(insertion + 24, insertion + 25),
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 26),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "text".to_owned(),
                    span: s(insertion + 6, insertion + 10),
                },
                type_syntax: string_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: projected,
                semicolon_span: s(insertion + 25, insertion + 26),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn owned_array_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str =
        "let b: FixedArray<String, 2> = FixedArray<String, 2>([\"old0\", \"old1\"]); b = a; ";
    let (mut source, mut raw) = owned_array_partial_then_root_snapshot();
    let insertion = source.find("return a;").expect("partial array assignment insertion");
    source.insert_str(insertion, INSERT);
    let return_value = source.rfind("a;").expect("partial array assignment return");
    source.replace_range(return_value..=return_value, "b");
    let insertion = u32::try_from(insertion).expect("partial array assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("partial array assignment length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let annotation_element =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array annotation element type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(18, 24),
        kind: RawTypeSyntaxKind::String { keyword_span: s(18, 24) },
    });
    let annotation_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array annotation type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(7, 28),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(7, 17),
            less_than_span: s(17, 18),
            element: annotation_element,
            comma_span: s(24, 25),
            length_span: s(26, 27),
            length: 2,
            length_spelling: "2".to_owned(),
            greater_than_span: s(27, 28),
        },
    });
    let constructor_element =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array constructor element type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(42, 48),
        kind: RawTypeSyntaxKind::String { keyword_span: s(42, 48) },
    });
    let constructor_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array constructor type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(31, 52),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(31, 41),
            less_than_span: s(41, 42),
            element: constructor_element,
            comma_span: s(48, 49),
            length_span: s(50, 51),
            length: 2,
            length_spelling: "2".to_owned(),
            greater_than_span: s(51, 52),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let old0 = u32::try_from(body.expressions.len()).expect("old array element zero");
    body.expressions.push(RawExpressionSyntax {
        span: s(54, 60),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
            spelling: "\"old0\"".to_owned(),
        },
    });
    let old1 = u32::try_from(body.expressions.len()).expect("old array element one");
    body.expressions.push(RawExpressionSyntax {
        span: s(62, 68),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
            spelling: "\"old1\"".to_owned(),
        },
    });
    let initializer = u32::try_from(body.expressions.len()).expect("old array initializer");
    body.expressions.push(RawExpressionSyntax {
        span: s(31, 70),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: constructor_type,
            open_paren_span: s(52, 53),
            open_bracket_span: s(53, 54),
            elements: vec![old0, old1],
            close_bracket_span: s(68, 69),
            close_paren_span: s(69, 70),
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("partial array assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(72, 73),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "b".to_owned(), span: s(72, 73) },
        },
    });
    let partial_source =
        u32::try_from(body.expressions.len()).expect("partial array assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(76, 77),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(76, 77) },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(0, 71),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 3),
                mutable: true,
                name: RawIdentifierSyntax { text: "b".to_owned(), span: s(4, 5) },
                type_syntax: annotation_type,
                equals_span: s(29, 30),
                initializer,
                semicolon_span: s(70, 71),
            },
        },
    );
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(72, 78),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(74, 75),
                value: partial_source,
                semicolon_span: s(77, 78),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial array assignment return")
    };
    let return_value = u32::try_from(return_value).expect("partial array return offset");
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_value, end: return_value + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "b".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: return_value,
                    end: return_value + 1,
                },
            },
        },
    };
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn owned_pair_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const q: OwnedPair = p; ";
    const PREFIX: &str = "OwnedPair({ flag: ";
    const SUFFIX: &str = ".flag, first: text })";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let insertion = source.find("return p;").expect("partial transfer insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("partial transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCAL.len()).expect("partial transfer local length"),
    );

    let return_start = source.find("return p;").expect("shifted Pair return");
    let expression_start = return_start + "return ".len();
    source.insert_str(expression_start, PREFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(expression_start).expect("return expression start"),
        u32::try_from(PREFIX.len()).expect("return prefix length"),
    );
    let q_start = expression_start + PREFIX.len();
    source.replace_range(q_start..=q_start, "q");
    source.insert_str(q_start + 1, SUFFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(q_start + 1).expect("return suffix start"),
        u32::try_from(SUFFIX.len()).expect("return suffix length"),
    );

    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("span start"),
        end: u32::try_from(end).expect("span end"),
    };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion as usize + 9, insertion as usize + 18),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(insertion as usize + 9, insertion as usize + 18),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_value = u32::try_from(body.expressions.len()).expect("partial source value");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion as usize + 21, insertion as usize + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: s(insertion as usize + 21, insertion as usize + 22),
            },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion as usize, insertion as usize + 23),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion as usize, insertion as usize + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "q".to_owned(),
                    span: s(insertion as usize + 6, insertion as usize + 7),
                },
                type_syntax: pair_type,
                equals_span: s(insertion as usize + 19, insertion as usize + 20),
                initializer: source_value,
                semicolon_span: s(insertion as usize + 22, insertion as usize + 23),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];

    let RawStatementKind::Return { value: old_return, .. } = body.statements[3].kind else {
        panic!("partial transfer return")
    };
    body.expressions[old_return as usize] = RawExpressionSyntax {
        span: s(q_start, q_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "q".to_owned(), span: s(q_start, q_start + 1) },
        },
    };
    let flag = u32::try_from(body.expressions.len()).expect("q.flag expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(q_start, q_start + 6),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: old_return,
            dot_span: s(q_start + 1, q_start + 2),
            field: RawIdentifierSyntax {
                text: "flag".to_owned(),
                span: s(q_start + 2, q_start + 6),
            },
        },
    });
    let text_start = q_start + 15;
    let text = u32::try_from(body.expressions.len()).expect("text expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(text_start, text_start + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(text_start, text_start + 4),
            },
        },
    });
    let expression_end = q_start + 1 + SUFFIX.len();
    let result = u32::try_from(body.expressions.len()).expect("rebuilt Pair expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(expression_start, expression_end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(expression_start, expression_start + 9),
            },
            open_paren_span: s(expression_start + 9, expression_start + 10),
            open_brace_span: s(expression_start + 10, expression_start + 11),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(expression_start + 12, q_start + 6),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(expression_start + 12, expression_start + 16),
                        },
                        colon_span: s(expression_start + 16, expression_start + 17),
                        value: flag,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(q_start + 8, text_start + 4),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(q_start + 8, q_start + 13),
                        },
                        colon_span: s(q_start + 13, q_start + 14),
                        value: text,
                    },
                },
            ],
            close_brace_span: s(expression_end - 2, expression_end - 1),
            close_paren_span: s(expression_end - 1, expression_end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("partial transfer return")
    };
    *value = result;
    (source, raw)
}

fn owned_pair_partial_transfer_then_use_source_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_partial_local_transfer_snapshot();
    let q_start = source.find("flag: q.flag").expect("transferred return owner") + "flag: ".len();
    source.replace_range(q_start..=q_start, "p");
    let q_start = u32::try_from(q_start).expect("return owner offset");
    let expression = raw.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find(|expression| {
            expression.span.start == q_start
                && matches!(
                    &expression.kind,
                    zryna_syntax::v4::RawExpressionKind::Reference { name }
                        if name.text == "q"
                )
        })
        .expect("transferred return owner expression");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expression.kind else {
        unreachable!("filtered return owner expression")
    };
    name.text = "p".to_owned();
    (source, raw)
}

fn owned_pair_repeated_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const r: OwnedPair = q; ";
    let (mut source, mut raw) = owned_pair_partial_local_transfer_snapshot();
    let insertion = source.find("return OwnedPair").expect("repeated transfer insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("repeated transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCAL.len()).expect("repeated transfer local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 9, insertion + 18),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(insertion + 9, insertion + 18),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_value = u32::try_from(body.expressions.len()).expect("repeated source value");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(insertion, insertion + 23),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "r".to_owned(),
                    span: s(insertion + 6, insertion + 7),
                },
                type_syntax: pair_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: source_value,
                semicolon_span: s(insertion + 22, insertion + 23),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let q_start = source.find("flag: q.flag").expect("repeated return owner") + "flag: ".len();
    source.replace_range(q_start..=q_start, "r");
    let q_start = u32::try_from(q_start).expect("repeated return owner offset");
    let expression = body
        .expressions
        .iter_mut()
        .find(|expression| {
            expression.span.start == q_start
                && matches!(
                    &expression.kind,
                    zryna_syntax::v4::RawExpressionKind::Reference { name }
                        if name.text == "q"
                )
        })
        .expect("repeated return owner expression");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expression.kind else {
        unreachable!("filtered repeated owner expression")
    };
    name.text = "r".to_owned();
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn nested_owned_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL_PREFIX: &str = "const o: Outer = ";
    const SUFFIX: &str = "const text: String = o.inner.text; const q: Outer = o; return Outer({ tail: \"c\", inner: Inner({ text: \"d\" }) }); ";
    let mut source = NESTED_OWNED_SOURCE.to_owned();
    let return_start = source.find("return Outer").expect("nested return");
    let initializer_start = return_start + "return ".len();
    source.replace_range(return_start..initializer_start, LOCAL_PREFIX);
    let prefix_growth =
        u32::try_from(LOCAL_PREFIX.len() - "return ".len()).expect("nested local prefix growth");
    let mut raw = shift_snapshot(
        response_snapshot(NESTED_OWNED_RESPONSE),
        u32::try_from(initializer_start).expect("nested initializer start"),
        prefix_growth,
    );
    let insertion = source.rfind('}').expect("nested function close");
    source.insert_str(insertion, SUFFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested suffix insertion"),
        u32::try_from(SUFFIX.len()).expect("nested suffix length"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested span start"),
        end: u32::try_from(end).expect("nested span end"),
    };
    let local_outer_start = return_start + "const o: ".len();
    let local_outer_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested local Outer type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(local_outer_start, local_outer_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(local_outer_start, local_outer_start + "Outer".len()),
            },
        },
    });
    let text_statement = insertion;
    let string_start = text_statement + "const text: ".len();
    let string_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested local String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(string_start, string_start + "String".len()),
        kind: RawTypeSyntaxKind::String {
            keyword_span: s(string_start, string_start + "String".len()),
        },
    });
    let q_statement = source[text_statement..]
        .find("const q:")
        .map(|offset| text_statement + offset)
        .expect("nested q statement");
    let q_type_start = q_statement + "const q: ".len();
    let q_type = u32::try_from(raw.files[0].type_syntax.len()).expect("nested q type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(q_type_start, q_type_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(q_type_start, q_type_start + "Outer".len()),
            },
        },
    });

    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: initializer, semicolon_span, .. } =
        body.statements[0].kind
    else {
        panic!("nested original return")
    };
    body.statements[0] = RawStatementSyntax {
        span: s(return_start, usize::try_from(semicolon_span.end).expect("nested local end")),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(return_start, return_start + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(return_start + "const ".len(), return_start + "const o".len()),
            },
            type_syntax: local_outer_type,
            equals_span: s(
                return_start + "const o: Outer ".len(),
                return_start + "const o: Outer =".len(),
            ),
            initializer,
            semicolon_span,
        },
    };

    let projection_start = text_statement + "const text: String = ".len();
    let outer_base = u32::try_from(body.expressions.len()).expect("nested outer base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(projection_start, projection_start + 1),
            },
        },
    });
    let inner = u32::try_from(body.expressions.len()).expect("nested inner projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + "o.inner".len()),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: outer_base,
            dot_span: s(projection_start + 1, projection_start + 2),
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: s(projection_start + 2, projection_start + "o.inner".len()),
            },
        },
    });
    let nested_text = u32::try_from(body.expressions.len()).expect("nested text projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + "o.inner.text".len()),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: inner,
            dot_span: s(projection_start + "o.inner".len(), projection_start + "o.inner.".len()),
            field: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(
                    projection_start + "o.inner.".len(),
                    projection_start + "o.inner.text".len(),
                ),
            },
        },
    });
    let text_semicolon = projection_start + "o.inner.text".len();
    body.statements.push(RawStatementSyntax {
        span: s(text_statement, text_semicolon + 1),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(text_statement, text_statement + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(text_statement + "const ".len(), text_statement + "const text".len()),
            },
            type_syntax: string_type,
            equals_span: s(projection_start - 2, projection_start - 1),
            initializer: nested_text,
            semicolon_span: s(text_semicolon, text_semicolon + 1),
        },
    });
    let q_source_start = q_statement + "const q: Outer = ".len();
    let q_source = u32::try_from(body.expressions.len()).expect("nested transfer source id");
    body.expressions.push(RawExpressionSyntax {
        span: s(q_source_start, q_source_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(q_source_start, q_source_start + 1),
            },
        },
    });
    body.statements.push(RawStatementSyntax {
        span: s(q_statement, q_source_start + 2),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(q_statement, q_statement + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(q_statement + "const ".len(), q_statement + "const q".len()),
            },
            type_syntax: q_type,
            equals_span: s(q_source_start - 2, q_source_start - 1),
            initializer: q_source,
            semicolon_span: s(q_source_start + 1, q_source_start + 2),
        },
    });

    let return_statement = source[q_source_start..]
        .find("return Outer")
        .map(|offset| q_source_start + offset)
        .expect("nested rebuilt return");
    let result_start = return_statement + "return ".len();
    let tail_start = source[result_start..]
        .find("\"c\"")
        .map(|offset| result_start + offset)
        .expect("nested rebuilt tail");
    let text_literal_start = source[tail_start + 3..]
        .find("\"d\"")
        .map(|offset| tail_start + 3 + offset)
        .expect("nested rebuilt text");
    let inner_start = source[result_start..]
        .find("Inner({")
        .map(|offset| result_start + offset)
        .expect("nested rebuilt Inner");
    let result_end = source[result_start..]
        .find(" });")
        .map(|offset| result_start + offset + " })".len())
        .expect("nested rebuilt result end");
    let tail = u32::try_from(body.expressions.len()).expect("nested rebuilt tail id");
    body.expressions.push(RawExpressionSyntax {
        span: s(tail_start, tail_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"c\"".to_owned() },
    });
    let rebuilt_text = u32::try_from(body.expressions.len()).expect("nested rebuilt text id");
    body.expressions.push(RawExpressionSyntax {
        span: s(text_literal_start, text_literal_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"d\"".to_owned() },
    });
    let rebuilt_inner = u32::try_from(body.expressions.len()).expect("nested rebuilt Inner id");
    body.expressions.push(RawExpressionSyntax {
        span: s(inner_start, inner_start + "Inner({ text: \"d\" })".len()),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "Inner".to_owned(),
                span: s(inner_start, inner_start + "Inner".len()),
            },
            open_paren_span: s(inner_start + 5, inner_start + 6),
            open_brace_span: s(inner_start + 6, inner_start + 7),
            fields: vec![zryna_syntax::v4::RawFieldInitializer {
                span: s(inner_start + 8, text_literal_start + 3),
                kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                    name: RawIdentifierSyntax {
                        text: "text".to_owned(),
                        span: s(inner_start + 8, inner_start + 12),
                    },
                    colon_span: s(inner_start + 12, inner_start + 13),
                    value: rebuilt_text,
                },
            }],
            close_brace_span: s(inner_start + 18, inner_start + 19),
            close_paren_span: s(inner_start + 19, inner_start + 20),
        },
    });
    let rebuilt_outer = u32::try_from(body.expressions.len()).expect("nested rebuilt Outer id");
    body.expressions.push(RawExpressionSyntax {
        span: s(result_start, result_end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(result_start, result_start + "Outer".len()),
            },
            open_paren_span: s(result_start + 5, result_start + 6),
            open_brace_span: s(result_start + 6, result_start + 7),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(result_start + 8, tail_start + 3),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "tail".to_owned(),
                            span: s(result_start + 8, result_start + 12),
                        },
                        colon_span: s(result_start + 12, result_start + 13),
                        value: tail,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(result_start + 19, inner_start + 20),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "inner".to_owned(),
                            span: s(result_start + 19, result_start + 24),
                        },
                        colon_span: s(result_start + 24, result_start + 25),
                        value: rebuilt_inner,
                    },
                },
            ],
            close_brace_span: s(result_end - 2, result_end - 1),
            close_paren_span: s(result_end - 1, result_end),
        },
    });
    let return_semicolon = result_end;
    body.statements.push(RawStatementSyntax {
        span: s(return_statement, return_semicolon + 1),
        kind: RawStatementKind::Return {
            keyword_span: s(return_statement, return_statement + "return".len()),
            value: rebuilt_outer,
            semicolon_span: s(return_semicolon, return_semicolon + 1),
        },
    });
    body.blocks[0].statements = vec![0, 1, 2, 3];
    (source, raw)
}

fn nested_owned_partial_return_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL_PREFIX: &str = "const unused: Outer = ";
    const RETURN: &str = "return q; ";
    let (mut source, mut raw) = nested_owned_partial_local_transfer_snapshot();
    let return_start = source.rfind("return Outer").expect("nested partial return source");
    let initializer_start = return_start + "return ".len();
    source.replace_range(return_start..initializer_start, LOCAL_PREFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(initializer_start).expect("nested partial initializer start"),
        u32::try_from(LOCAL_PREFIX.len() - "return ".len()).expect("nested partial local growth"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested partial span start"),
        end: u32::try_from(end).expect("nested partial span end"),
    };
    let type_start = return_start + "const unused: ".len();
    let outer_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested partial unused Outer type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(type_start, type_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(type_start, type_start + "Outer".len()),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let last = body.statements.len() - 1;
    let RawStatementKind::Return { value: initializer, semicolon_span, .. } =
        body.statements[last].kind
    else {
        panic!("nested partial original return")
    };
    body.statements[last] = RawStatementSyntax {
        span: s(
            return_start,
            usize::try_from(semicolon_span.end).expect("nested partial local end"),
        ),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(return_start, return_start + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(return_start + "const ".len(), return_start + "const unused".len()),
            },
            type_syntax: outer_type,
            equals_span: s(
                initializer_start + LOCAL_PREFIX.len() - "return ".len() - 2,
                initializer_start + LOCAL_PREFIX.len() - "return ".len() - 1,
            ),
            initializer,
            semicolon_span,
        },
    };

    let insertion = source.rfind('}').expect("nested partial function close");
    source.insert_str(insertion, RETURN);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested partial return insertion"),
        u32::try_from(RETURN.len()).expect("nested partial return length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let returned = u32::try_from(body.expressions.len()).expect("nested partial q value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + "return ".len(), insertion + "return q".len()),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + "return ".len(), insertion + "return q".len()),
            },
        },
    });
    let return_id =
        u32::try_from(body.statements.len()).expect("nested partial return statement id");
    body.statements.push(RawStatementSyntax {
        span: s(insertion, insertion + "return q;".len()),
        kind: RawStatementKind::Return {
            keyword_span: s(insertion, insertion + "return".len()),
            value: returned,
            semicolon_span: s(insertion + "return q".len(), insertion + "return q;".len()),
        },
    });
    body.blocks[0].statements.push(return_id);
    (source, raw)
}

fn nested_owned_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "unused = q; ";
    let (mut source, mut raw) = nested_owned_partial_return_snapshot();
    let local_start = source.rfind("const unused").expect("nested assignment target local");
    source.replace_range(local_start..local_start + 5, "let  ");
    let insertion = source.rfind("return q;").expect("nested assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested assignment offset"),
        u32::try_from(ASSIGNMENT.len()).expect("nested assignment length"),
    );
    let return_value = insertion + ASSIGNMENT.len() + "return ".len();
    source.replace_range(return_value..=return_value, "unused");
    raw = shift_snapshot(
        raw,
        u32::try_from(return_value + 1).expect("nested assignment return growth start"),
        u32::try_from("unused".len() - 1).expect("nested assignment return growth"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested assignment span start"),
        end: u32::try_from(end).expect("nested assignment span end"),
    };
    let body = &mut raw.files[0].functions[0].body;
    let target_statement = body
        .statements
        .iter_mut()
        .find(|statement| {
            matches!(
                &statement.kind,
                RawStatementKind::LocalDeclaration { name, .. } if name.text == "unused"
            )
        })
        .expect("nested assignment target declaration");
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut target_statement.kind
    else {
        unreachable!("filtered target declaration")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let target = u32::try_from(body.expressions.len()).expect("nested assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 6),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(insertion, insertion + 6),
            },
        },
    });
    let partial_source = u32::try_from(body.expressions.len()).expect("nested assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 9, insertion + 10),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + 9, insertion + 10),
            },
        },
    });
    let return_index = body.statements.len() - 1;
    body.statements.insert(
        return_index,
        RawStatementSyntax {
            span: s(insertion, insertion + 11),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 7, insertion + 8),
                value: partial_source,
                semicolon_span: s(insertion + 10, insertion + 11),
            },
        },
    );
    let return_start = insertion + ASSIGNMENT.len();
    let return_statement = &mut body.statements[return_index + 1];
    return_statement.span = s(return_start, return_value + 7);
    let RawStatementKind::Return { value, keyword_span, semicolon_span } =
        &mut return_statement.kind
    else {
        panic!("nested assignment return")
    };
    let value = *value;
    *keyword_span = s(return_start, return_start + 6);
    *semicolon_span = s(return_value + 6, return_value + 7);
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(return_value, return_value + 6),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(return_value, return_value + 6),
            },
        },
    };
    body.blocks[0].statements =
        (0..u32::try_from(body.statements.len()).expect("nested assignment statements")).collect();
    (source, raw)
}

fn owned_pair_moved_projection_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "p.first = \"b\"; ";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let mutable = source.find("const p").expect("owned Pair local");
    source.replace_range(mutable..mutable + 5, "let  ");
    let insertion = source.find("return p;").expect("moved projection assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("moved projection assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("moved projection assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let base = u32::try_from(body.expressions.len()).expect("moved target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("moved target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: s(insertion + 1, insertion + 2),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 2, insertion + 7),
            },
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("moved target replacement");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 10, insertion + 13),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion, insertion + 14),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 8, insertion + 9),
                value,
                semicolon_span: s(insertion + 13, insertion + 14),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];
    (source, raw)
}

fn struct_index_wrong_base_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_projected_return_snapshot("first");
    let start = source.rfind("p.first").expect("Struct wrong-base projection");
    source.replace_range(start..start + 7, "p[0]");
    let start = u32::try_from(start).expect("Struct projection offset");
    let mut raw = shift_snapshot_signed(raw, start + 7, -3);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[6] = RawExpressionSyntax {
        span: s(start + 2, start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    body.expressions.insert(
        7,
        RawExpressionSyntax {
            span: s(start, start + 4),
            kind: zryna_syntax::v4::RawExpressionKind::Index {
                base: 5,
                open_bracket_span: s(start + 1, start + 2),
                index: 6,
                close_bracket_span: s(start + 3, start + 4),
            },
        },
    );
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut body.expressions[8].kind
    else {
        panic!("Struct wrong-base result")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } = &mut fields[1].kind
    else {
        panic!("Struct wrong-base initializer")
    };
    *value = 7;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("Struct wrong-base return")
    };
    *value = 8;
    (source, raw)
}

fn fixed_array_field_wrong_base_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Disjoint);
    let start = source.rfind("a[0]").expect("array wrong-base projection");
    source.replace_range(start..start + 4, "a.foo");
    let start = u32::try_from(start).expect("array projection offset");
    let mut raw = shift_snapshot(raw, start + 4, 1);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions.remove(4);
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[4] = RawExpressionSyntax {
        span: s(start, start + 5),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 3,
            dot_span: s(start + 1, start + 2),
            field: RawIdentifierSyntax { text: "foo".to_owned(), span: s(start + 2, start + 5) },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base, index, .. } =
        &mut body.expressions[7].kind
    else {
        panic!("second array projection")
    };
    *base = 5;
    *index = 6;
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[8].kind
    else {
        panic!("array wrong-base result")
    };
    *elements = vec![4, 7];
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array wrong-base return")
    };
    *value = 8;
    (source, raw)
}

fn fixed_array_oob_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const FINAL_RETURN: &str = "return a; ";
    let (mut source, mut raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::OutOfBounds);
    let fresh = source.rfind("a[1]").expect("fresh array assignment element");
    source.replace_range(fresh..fresh + 4, "\"b\"");
    let fresh = u32::try_from(fresh).expect("fresh element offset");
    raw = shift_snapshot_signed(raw, fresh + 4, -1);
    {
        let body = &mut raw.files[0].functions[0].body;
        body.expressions.remove(6);
        body.expressions.remove(6);
        body.expressions[6] = RawExpressionSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: fresh, end: fresh + 3 },
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: "\"b\"".to_owned(),
            },
        };
        let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
            &mut body.expressions[7].kind
        else {
            panic!("fresh array assignment result")
        };
        *elements = vec![5, 6];
        let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
            panic!("fresh array assignment return")
        };
        *value = 7;
    }
    source.replace_range(41..46, "let  ");
    let assignment = source.find("return FixedArray").expect("array assignment return");
    source.replace_range(assignment..assignment + 7, "a = ");
    let assignment = u32::try_from(assignment).expect("array assignment offset");
    let mut raw = shift_snapshot_signed(raw, assignment + 7, -3);
    let insertion = source.rfind('}').expect("array function close");
    source.insert_str(insertion, FINAL_RETURN);
    let insertion = u32::try_from(insertion).expect("final return offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(FINAL_RETURN.len()).expect("final return length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("array assignment local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let RawStatementKind::Return { value: replacement, semicolon_span, .. } =
        body.statements[1].kind
    else {
        panic!("array replacement expression")
    };
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let target = u32::try_from(body.expressions.len()).expect("array assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(assignment, assignment + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(assignment, assignment + 1) },
        },
    });
    body.statements[1] = RawStatementSyntax {
        span: s(assignment, semicolon_span.end),
        kind: RawStatementKind::Assignment {
            target,
            equals_span: s(assignment + 2, assignment + 3),
            value: replacement,
            semicolon_span,
        },
    };
    let returned = u32::try_from(body.expressions.len()).expect("array final return");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 7, insertion + 8),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 7, insertion + 8),
            },
        },
    });
    body.statements.push(RawStatementSyntax {
        span: s(insertion, insertion + 9),
        kind: RawStatementKind::Return {
            keyword_span: s(insertion, insertion + 6),
            value: returned,
            semicolon_span: s(insertion + 8, insertion + 9),
        },
    });
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

fn sources_for(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("source map")
}

fn nth_untrusted_span(text: &str, needle: &str, ordinal: usize) -> zryna_source::UntrustedSpan {
    let start =
        text.match_indices(needle).nth(ordinal).map(|(start, _)| start).expect("fixture token");
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(start + needle.len()).expect("fixture offset"),
    }
}

fn untrusted_range(
    text: &str,
    start: (&str, usize),
    end: (&str, usize),
) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: nth_untrusted_span(text, start.0, start.1).start,
        end: nth_untrusted_span(text, end.0, end.1).end,
    }
}

#[allow(clippy::too_many_lines)]
fn private_vec_clone_fixture(element: &str) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;

    let elements = if element == "String" { "[\"a\", \"b\", \"c\"]" } else { "[]" };
    let source = format!(
        "function copy(): Vec<{element}> {{ const source: Vec<{element}> = Vec<{element}>({elements}); return clone(source); }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&source, (start, start_ordinal), (end, end_ordinal))
    };
    let spelling = format!("Vec<{element}>");
    let mut types = Vec::new();
    let mut vec_types = Vec::new();
    for ordinal in 0..3 {
        let vec_span = token(&spelling, ordinal);
        let element_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 4,
            end: vec_span.end - 1,
        };
        let element_type = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: if element == "String" {
                RawTypeSyntaxKind::String { keyword_span: element_span }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: element.to_owned(), span: element_span },
                }
            },
        });
        let vec_type = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: vec_span,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.start,
                    end: vec_span.start + 3,
                },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.start + 3,
                    end: vec_span.start + 4,
                },
                argument: element_type,
                greater_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.end - 1,
                    end: vec_span.end,
                },
            },
        });
        vec_types.push(vec_type);
    }
    let root = range("{", 0, "}", 0);
    let local = range("const", 0, ";", 0);
    let returned = range("return", 0, ";", 1);
    let mut expressions = Vec::new();
    let element_ids = if element == "String" {
        ["\"a\"", "\"b\"", "\"c\""]
            .into_iter()
            .map(|spelling| {
                let id = u32::try_from(expressions.len()).expect("expression id");
                expressions.push(RawExpressionSyntax {
                    span: token(spelling, 0),
                    kind: RawExpressionKind::StringLiteral { spelling: spelling.to_owned() },
                });
                id
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let construct = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: range(&spelling, 2, ")", 1),
        kind: RawExpressionKind::VecConstruction {
            type_syntax: vec_types[2],
            open_paren_span: token("(", 1),
            open_bracket_span: token("[", 0),
            elements: element_ids,
            close_bracket_span: token("]", 0),
            close_paren_span: token(")", 1),
        },
    });
    let reference = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: token("source", 1),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "source".to_owned(), span: token("source", 1) },
        },
    });
    let cloned = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: range("clone", 0, ")", 2),
        kind: RawExpressionKind::Clone {
            keyword_span: token("clone", 0),
            open_paren_span: token("(", 2),
            value: reference,
            close_paren_span: token(")", 2),
        },
    });
    let statements = vec![
        RawStatementSyntax {
            span: local,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "source".to_owned(), span: token("source", 0) },
                type_syntax: vec_types[1],
                equals_span: token("=", 0),
                initializer: construct,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: returned,
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: cloned,
                semicolon_span: token(";", 1),
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(source.len()).expect("source length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "copy".to_owned(), span: token("copy", 0) },
        parameters: Vec::new(),
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: root,
                open_brace_span: token("{", 0),
                statements: vec![0, 1],
                close_brace_span: token("}", 0),
            }],
            statements,
            expressions,
        },
    };
    (
        source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_string_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let text = "function choose(flag: bool): String { const own: String = \"keep\"; if (flag) { const first: String = \"a\"; const second: String = \"b\"; } else { const third: String = clone(own); } return own; }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..5).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
    let types = std::iter::once(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    })
    .chain(string_spans.iter().copied().map(|keyword_span| RawTypeSyntax {
        span: keyword_span,
        kind: RawTypeSyntaxKind::String { keyword_span },
    }))
    .collect();
    let root_open = token("{", 0);
    let then_open = token("{", 1);
    let else_open = token("{", 2);
    let then_close = token("}", 0);
    let else_close = token("}", 1);
    let root_close = token("}", 2);
    let outer_statement = range("const own", 0, ";", 0);
    let if_statement = range("if", 0, "}", 1);
    let first_statement = range("const first", 0, ";", 1);
    let second_statement = range("const second", 0, ";", 2);
    let third_statement = range("const third", 0, ";", 3);
    let return_statement = range("return", 0, ";", 4);
    let expressions = vec![
        RawExpressionSyntax {
            span: token("\"keep\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"keep\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("\"b\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("own", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 1) },
            },
        },
        RawExpressionSyntax {
            span: range("clone", 0, ")", 2),
            kind: RawExpressionKind::Clone {
                keyword_span: token("clone", 0),
                open_paren_span: token("(", 2),
                value: 4,
                close_paren_span: token(")", 2),
            },
        },
        RawExpressionSyntax {
            span: token("own", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 2) },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: outer_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: if_statement,
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 1),
                condition: 1,
                close_paren_span: token(")", 1),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: first_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: 3,
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: second_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: 4,
                equals_span: token("=", 2),
                initializer: 3,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: third_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 3),
                mutable: false,
                name: RawIdentifierSyntax { text: "third".to_owned(), span: token("third", 0) },
                type_syntax: 5,
                equals_span: token("=", 3),
                initializer: 5,
                semicolon_span: token(";", 3),
            },
        },
        RawStatementSyntax {
            span: return_statement,
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 6,
                semicolon_span: token(";", 4),
            },
        },
    ];
    let body_span =
        zryna_source::UntrustedSpan { file: 0, start: root_open.start, end: root_close.end };
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: 1,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: root_open,
                    statements: vec![0, 1, 5],
                    close_brace_span: root_close,
                },
                RawBlockSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: then_open.start,
                        end: then_close.end,
                    },
                    open_brace_span: then_open,
                    statements: vec![2, 3],
                    close_brace_span: then_close,
                },
                RawBlockSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: else_open.start,
                        end: else_close.end,
                    },
                    open_brace_span: else_open,
                    statements: vec![4],
                    close_brace_span: else_close,
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_vec_if_fixture(push_outer: bool, element: &str) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let target = if push_outer { "own" } else { "branch" };
    let pushed = if element == "String" { "\"p\"" } else { "7" };
    let text = format!(
        "function choose(flag: bool): Vec<{element}> {{ const own: Vec<{element}> = Vec<{element}>([]); if (flag) {{ let branch: Vec<{element}> = Vec<{element}>([]); push({target}, {pushed}); }} else {{ const value: String = \"e\"; }} return own; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let mut types = Vec::new();
    let bool_span = token("bool", 0);
    types.push(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    });
    let mut vec_type_ids = Vec::new();
    let vec_spelling = format!("Vec<{element}>");
    for ordinal in 0..5 {
        let vec_span = token(&vec_spelling, ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: vec_span.start, end: vec_span.start + 3 };
        let less_than_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 3,
            end: vec_span.start + 4,
        };
        let element_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 4,
            end: vec_span.start + 4 + u32::try_from(element.len()).expect("element length"),
        };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: element_span.end, end: vec_span.end };
        let element_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: if element == "String" {
                RawTypeSyntaxKind::String { keyword_span: element_span }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
                }
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: vec_span,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element_id,
                greater_than_span,
            },
        });
        vec_type_ids.push(vec_id);
    }
    let scalar_span = token("String", usize::from(element == "String") * 5);
    let scalar_ty = u32::try_from(types.len()).expect("type id");
    types.push(RawTypeSyntax {
        span: scalar_span,
        kind: RawTypeSyntaxKind::String { keyword_span: scalar_span },
    });
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let target_span = if push_outer { token("own", 1) } else { token("branch", 1) };
    let expressions = vec![
        RawExpressionSyntax {
            span: range(&vec_spelling, 2, ")", 1),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_type_ids[2],
                open_paren_span: token("(", 1),
                open_bracket_span: token("[", 0),
                elements: Vec::new(),
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 1),
            },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: range(&vec_spelling, 4, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_type_ids[4],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: Vec::new(),
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: target_span,
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: target.to_owned(), span: target_span },
            },
        },
        RawExpressionSyntax {
            span: token(pushed, 0),
            kind: if element == "String" {
                RawExpressionKind::StringLiteral { spelling: pushed.to_owned() }
            } else {
                RawExpressionKind::I32Literal { spelling: pushed.to_owned() }
            },
        },
        RawExpressionSyntax {
            span: range("push", 0, ")", 4),
            kind: RawExpressionKind::VecPush {
                keyword_span: token("push", 0),
                open_paren_span: token("(", 4),
                vector: 3,
                comma_span: token(",", 0),
                value: 4,
                close_paren_span: token(")", 4),
            },
        },
        RawExpressionSyntax {
            span: token("\"e\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"e\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("own", usize::from(push_outer) + 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "own".to_owned(),
                    span: token("own", usize::from(push_outer) + 1),
                },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("const own", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 0) },
                type_syntax: vec_type_ids[1],
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("if", 0, "}", 1),
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 2),
                condition: 1,
                close_paren_span: token(")", 2),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: range("let branch", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("let", 0),
                mutable: true,
                name: RawIdentifierSyntax { text: "branch".to_owned(), span: token("branch", 0) },
                type_syntax: vec_type_ids[3],
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("push", 0, ";", 2),
            kind: RawStatementKind::ExpressionStatement {
                expression: 5,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("const value", 0, ";", 3),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "value".to_owned(), span: token("value", 0) },
                type_syntax: scalar_ty,
                equals_span: token("=", 2),
                initializer: 6,
                semicolon_span: token(";", 3),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 4),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 7,
                semicolon_span: token(";", 4),
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: vec_type_ids[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 5],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![4],
                    close_brace_span: token("}", 1),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn terminal_string_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};
    let text = "function choose(flag: bool): String { if (flag) { return \"a\"; } else { return \"b\"; } }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_span = token("String", 0);
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: 1,
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![1],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![2],
                    close_brace_span: token("}", 1),
                },
            ],
            statements: vec![
                RawStatementSyntax {
                    span: range("if", 0, "}", 1),
                    kind: RawStatementKind::If {
                        keyword_span: token("if", 0),
                        open_paren_span: token("(", 1),
                        condition: 0,
                        close_paren_span: token(")", 1),
                        then_block: 1,
                        else_clause: Some(RawElseSyntax {
                            keyword_span: token("else", 0),
                            block: 2,
                        }),
                    },
                },
                RawStatementSyntax {
                    span: range("return", 0, ";", 0),
                    kind: RawStatementKind::Return {
                        keyword_span: token("return", 0),
                        value: 1,
                        semicolon_span: token(";", 0),
                    },
                },
                RawStatementSyntax {
                    span: range("return", 1, ";", 1),
                    kind: RawStatementKind::Return {
                        keyword_span: token("return", 1),
                        value: 2,
                        semicolon_span: token(";", 1),
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: token("flag", 1),
                    kind: RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: token("flag", 1),
                        },
                    },
                },
                RawExpressionSyntax {
                    span: token("\"a\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
                },
                RawExpressionSyntax {
                    span: token("\"b\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
                },
            ],
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: vec![
                    RawTypeSyntax {
                        span: bool_span,
                        kind: RawTypeSyntaxKind::Named {
                            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
                        },
                    },
                    RawTypeSyntax {
                        span: string_span,
                        kind: RawTypeSyntaxKind::String { keyword_span: string_span },
                    },
                ],
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn terminal_vec_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};
    let text = "function choose(flag: bool): Vec<i32> { if (flag) { return Vec<i32>([1]); } else { return Vec<i32>([2]); } }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let mut types = vec![RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    }];
    let mut vec_types = Vec::new();
    for ordinal in 0..3 {
        let full = token("Vec<i32>", ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start, end: full.start + 3 };
        let less_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 3, end: full.start + 4 };
        let element_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 4, end: full.start + 7 };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 7, end: full.end };
        let argument = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: full,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        vec_types.push(vec_id);
    }
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let expressions = vec![
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: token("1", 0),
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 1, ")", 2),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[1],
                open_paren_span: token("(", 2),
                open_bracket_span: token("[", 0),
                elements: vec![1],
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 2),
            },
        },
        RawExpressionSyntax {
            span: token("2", 3),
            kind: RawExpressionKind::I32Literal { spelling: "2".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 2, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[2],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: vec![3],
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("if", 0, "}", 1),
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 1),
                condition: 0,
                close_paren_span: token(")", 1),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 0),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 2,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("return", 1, ";", 1),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 1),
                value: 4,
                semicolon_span: token(";", 1),
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![1],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![2],
                    close_brace_span: token("}", 1),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_string_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_string_loop_fixture_with_options(false, false, false)
}

#[allow(clippy::too_many_lines)]
fn private_string_loop_fixture_with_incoming_move(
    move_incoming: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    private_string_loop_fixture_with_options(move_incoming, false, false)
}

#[allow(clippy::too_many_lines)]
fn private_string_loop_fixture_with_options(
    move_incoming: bool,
    non_bool_condition: bool,
    false_condition: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let first_initializer = if move_incoming { "outer" } else { "\"a\"" };
    let condition = if non_bool_condition {
        "outer"
    } else if false_condition {
        "false"
    } else {
        "flag"
    };
    let text = format!(
        "function keep(flag: bool): String {{ const outer: String = \"keep\"; while ({condition}) {{ const first: String = {first_initializer}; const second: String = \"b\"; }} return outer; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..4).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
    let types = std::iter::once(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    })
    .chain(string_spans.iter().copied().map(|keyword_span| RawTypeSyntax {
        span: keyword_span,
        kind: RawTypeSyntaxKind::String { keyword_span },
    }))
    .collect();
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let statements = vec![
        RawStatementSyntax {
            span: range("const outer", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 1),
                condition: 1,
                close_paren_span: token(")", 1),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("const first", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: 3,
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("const second", 0, ";", 2),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: 4,
                equals_span: token("=", 2),
                initializer: 3,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 3),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 4,
                semicolon_span: token(";", 3),
            },
        },
    ];
    let expressions = vec![
        RawExpressionSyntax {
            span: token("\"keep\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"keep\"".to_owned() },
        },
        if non_bool_condition {
            RawExpressionSyntax {
                span: token("outer", 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
                },
            }
        } else if false_condition {
            RawExpressionSyntax {
                span: token("false", 0),
                kind: RawExpressionKind::BoolLiteral { value: false },
            }
        } else {
            RawExpressionSyntax {
                span: token("flag", 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
                },
            }
        },
        if move_incoming {
            RawExpressionSyntax {
                span: token("outer", usize::from(non_bool_condition) + 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "outer".to_owned(),
                        span: token("outer", usize::from(non_bool_condition) + 1),
                    },
                },
            }
        } else {
            RawExpressionSyntax {
                span: token("\"a\"", 0),
                kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
            }
        },
        RawExpressionSyntax {
            span: token("\"b\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("outer", usize::from(non_bool_condition) + usize::from(move_incoming) + 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "outer".to_owned(),
                    span: token(
                        "outer",
                        usize::from(non_bool_condition) + usize::from(move_incoming) + 1,
                    ),
                },
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "keep".to_owned(), span: token("keep", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: 1,
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 4],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
                    close_brace_span: token("}", 0),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_vec_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let text = "function keep(flag: bool): Vec<i32> { const outer: Vec<i32> = Vec<i32>([]); while (flag) { const first: Vec<i32> = Vec<i32>([1]); const second: Vec<i32> = Vec<i32>([2]); } return outer; }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let mut types = vec![RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    }];
    let mut vec_types = Vec::new();
    for ordinal in 0..7 {
        let full = token("Vec<i32>", ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start, end: full.start + 3 };
        let less_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 3, end: full.start + 4 };
        let element_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 4, end: full.start + 7 };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 7, end: full.end };
        let argument = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: full,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        vec_types.push(vec_id);
    }
    let one_brackets = token("[1]", 0);
    let one_span = zryna_source::UntrustedSpan {
        file: 0,
        start: one_brackets.start + 1,
        end: one_brackets.end - 1,
    };
    let two_brackets = token("[2]", 0);
    let two_span = zryna_source::UntrustedSpan {
        file: 0,
        start: two_brackets.start + 1,
        end: two_brackets.end - 1,
    };
    let expressions = vec![
        RawExpressionSyntax {
            span: range("Vec<i32>", 2, ")", 1),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[2],
                open_paren_span: token("(", 1),
                open_bracket_span: token("[", 0),
                elements: Vec::new(),
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 1),
            },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: one_span,
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 4, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[4],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: vec![2],
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: two_span,
            kind: RawExpressionKind::I32Literal { spelling: "2".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 6, ")", 4),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[6],
                open_paren_span: token("(", 4),
                open_bracket_span: token("[", 2),
                elements: vec![4],
                close_bracket_span: token("]", 2),
                close_paren_span: token(")", 4),
            },
        },
        RawExpressionSyntax {
            span: token("outer", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("const outer", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: vec_types[1],
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 2),
                condition: 1,
                close_paren_span: token(")", 2),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("const first", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: vec_types[3],
                equals_span: token("=", 1),
                initializer: 3,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("const second", 0, ";", 2),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: vec_types[5],
                equals_span: token("=", 2),
                initializer: 5,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 3),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 6,
                semicolon_span: token(";", 3),
            },
        },
    ];
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "keep".to_owned(), span: token("keep", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 4],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
                    close_brace_span: token("}", 0),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_string_mutation_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::Literal)
}

#[derive(Clone, Copy)]
enum StringLoopReplacement {
    Literal,
    Move,
    Call,
    CloneRead,
    ConcatRead,
    CloneCall,
    ConcatCall,
}

#[allow(clippy::too_many_lines)]
fn private_string_mutation_loop_fixture_with_options(
    mutable: bool,
    replacement: StringLoopReplacement,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let declaration = if mutable { "let" } else { "const" };
    let replacement_source = match replacement {
        StringLoopReplacement::Literal => "\"after\"",
        StringLoopReplacement::Move => "outer",
        StringLoopReplacement::Call => "take(outer)",
        StringLoopReplacement::CloneRead => "clone(outer)",
        StringLoopReplacement::ConcatRead => "concat(outer, \"x\")",
        StringLoopReplacement::CloneCall => "clone(take(outer))",
        StringLoopReplacement::ConcatCall => "concat(take(outer), \"x\")",
    };
    let text = format!(
        "function keep(flag: bool): String {{ {declaration} outer: String = \"before\"; while (flag) {{ outer = {replacement_source}; }} return outer; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..2).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
    let types = std::iter::once(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    })
    .chain(string_spans.iter().copied().map(|keyword_span| RawTypeSyntax {
        span: keyword_span,
        kind: RawTypeSyntaxKind::String { keyword_span },
    }))
    .collect();
    let mut expressions = vec![
        RawExpressionSyntax {
            span: token("\"before\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"before\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: token("outer", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
            },
        },
    ];
    let mut push_outer_reference = || {
        let id = u32::try_from(expressions.len()).expect("outer expression id");
        expressions.push(RawExpressionSyntax {
            span: token("outer", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 2) },
            },
        });
        id
    };
    let replacement_id = match replacement {
        StringLoopReplacement::Literal => {
            let id = u32::try_from(expressions.len()).expect("literal expression id");
            expressions.push(RawExpressionSyntax {
                span: token("\"after\"", 0),
                kind: RawExpressionKind::StringLiteral { spelling: "\"after\"".to_owned() },
            });
            id
        }
        StringLoopReplacement::Move => push_outer_reference(),
        StringLoopReplacement::Call
        | StringLoopReplacement::CloneRead
        | StringLoopReplacement::ConcatRead
        | StringLoopReplacement::CloneCall
        | StringLoopReplacement::ConcatCall => {
            let outer = push_outer_reference();
            let consumed = matches!(
                replacement,
                StringLoopReplacement::Call
                    | StringLoopReplacement::CloneCall
                    | StringLoopReplacement::ConcatCall
            );
            let operand = if consumed {
                let id = u32::try_from(expressions.len()).expect("call expression id");
                let (open, close) = if matches!(replacement, StringLoopReplacement::Call) {
                    (2, 2)
                } else {
                    (3, 2)
                };
                expressions.push(RawExpressionSyntax {
                    span: range("take", 0, ")", close),
                    kind: RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "take".to_owned(),
                            span: token("take", 0),
                        },
                        open_paren_span: token("(", open),
                        arguments: vec![outer],
                        close_paren_span: token(")", close),
                    },
                });
                id
            } else {
                outer
            };
            if matches!(replacement, StringLoopReplacement::Call) {
                operand
            } else if matches!(
                replacement,
                StringLoopReplacement::CloneRead | StringLoopReplacement::CloneCall
            ) {
                let id = u32::try_from(expressions.len()).expect("clone expression id");
                let close = if consumed { 3 } else { 2 };
                expressions.push(RawExpressionSyntax {
                    span: range("clone", 0, ")", close),
                    kind: RawExpressionKind::Clone {
                        keyword_span: token("clone", 0),
                        open_paren_span: token("(", 2),
                        value: operand,
                        close_paren_span: token(")", close),
                    },
                });
                id
            } else {
                let literal = u32::try_from(expressions.len()).expect("literal expression id");
                expressions.push(RawExpressionSyntax {
                    span: token("\"x\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
                });
                let id = u32::try_from(expressions.len()).expect("concat expression id");
                let close = if consumed { 3 } else { 2 };
                expressions.push(RawExpressionSyntax {
                    span: range("concat", 0, ")", close),
                    kind: RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "concat".to_owned(),
                            span: token("concat", 0),
                        },
                        open_paren_span: token("(", 2),
                        arguments: vec![operand, literal],
                        close_paren_span: token(")", close),
                    },
                });
                id
            }
        }
    };
    let return_id = u32::try_from(expressions.len()).expect("return expression id");
    let return_ordinal = usize::from(!matches!(replacement, StringLoopReplacement::Literal)) + 2;
    expressions.push(RawExpressionSyntax {
        span: token("outer", return_ordinal),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "outer".to_owned(),
                span: token("outer", return_ordinal),
            },
        },
    });
    let declaration_start = format!("{declaration} outer");
    let statements = vec![
        RawStatementSyntax {
            span: range(&declaration_start, 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token(declaration, 0),
                mutable,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 1),
                condition: 1,
                close_paren_span: token(")", 1),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("outer", 1, ";", 1),
            kind: RawStatementKind::Assignment {
                target: 2,
                equals_span: token("=", 1),
                value: replacement_id,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 2),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: return_id,
                semicolon_span: token(";", 2),
            },
        },
    ];
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "keep".to_owned(), span: token("keep", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: 1,
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 3],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2],
                    close_brace_span: token("}", 0),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn private_nested_string_mutation_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) =
        private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::ConcatRead);
    let old = "concat(outer, \"x\")";
    let replacement = "identity(concat(outer, \"x\"))";
    let start = u32::try_from(source.find(old).expect("loop concat")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let body = &mut raw.files[0].functions[0].body;
    let outer = token("outer", 2);
    body.expressions[3] = RawExpressionSyntax {
        span: outer,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "outer".to_owned(), span: outer },
        },
    };
    let literal = token("\"x\"", 0);
    body.expressions[4] = RawExpressionSyntax {
        span: literal,
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
    };
    let concat_start = token("concat", 0);
    let concat_close = token(")", 2);
    body.expressions[5] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: concat_start.start,
            end: concat_close.end,
        },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "concat".to_owned(), span: concat_start },
            open_paren_span: token("(", 3),
            arguments: vec![3, 4],
            close_paren_span: concat_close,
        },
    };
    let return_expression = body.expressions.pop().expect("return expression");
    let identity_id = u32::try_from(body.expressions.len()).expect("identity id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 28 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "identity".to_owned(), span: token("identity", 0) },
            open_paren_span: token("(", 2),
            arguments: vec![5],
            close_paren_span: token(")", 3),
        },
    });
    let return_id = u32::try_from(body.expressions.len()).expect("return id");
    body.expressions.push(return_expression);
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("loop assignment")
    };
    *value = identity_id;
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("loop return")
    };
    *value = return_id;

    let (call_source, call_raw) = private_string_call_fixture();
    let identity = call_raw.files[0].functions[1].clone();
    let identity_text = &call_source[usize::try_from(identity.span.start).expect("start")
        ..usize::try_from(identity.span.end).expect("end")];
    source.push(' ');
    let appended_start = u32::try_from(source.len()).expect("offset");
    source.push_str(identity_text);
    let shift = i32::try_from(appended_start).expect("offset")
        - i32::try_from(identity.span.start).expect("offset");
    let mut identity_snapshot = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                call_raw.files[0].type_syntax
                    [usize::try_from(identity.parameters[0].type_syntax).expect("type")]
                .clone(),
                call_raw.files[0].type_syntax[usize::try_from(identity.result_type).expect("type")]
                    .clone(),
            ],
            data_declarations: Vec::new(),
            functions: vec![identity],
        }],
        diagnostics: Vec::new(),
    };
    identity_snapshot.files[0].functions[0].parameters[0].type_syntax = 0;
    identity_snapshot.files[0].functions[0].result_type = 1;
    identity_snapshot = shift_snapshot_signed(identity_snapshot, 0, shift);
    let type_base = u32::try_from(raw.files[0].type_syntax.len()).expect("type base");
    let appended = &mut identity_snapshot.files[0];
    appended.functions[0].parameters[0].type_syntax += type_base;
    appended.functions[0].result_type += type_base;
    raw.files[0].type_syntax.append(&mut appended.type_syntax);
    raw.files[0].functions.append(&mut appended.functions);
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn private_vec_push_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_vec_push_loop_fixture_with_mutability(true)
}

#[allow(clippy::too_many_lines)]
fn private_vec_push_loop_fixture_with_mutability(
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let declaration = if mutable { "let" } else { "const" };
    let text = format!(
        "function keep(flag: bool): Vec<i32> {{ {declaration} outer: Vec<i32> = Vec<i32>([]); while (flag) {{ push(outer, 1); }} return outer; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let mut types = vec![RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    }];
    let mut vec_types = Vec::new();
    for ordinal in 0..3 {
        let full = token("Vec<i32>", ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start, end: full.start + 3 };
        let less_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 3, end: full.start + 4 };
        let element_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 4, end: full.start + 7 };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 7, end: full.end };
        let argument = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: full,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        vec_types.push(vec_id);
    }
    let one_span = token("1", 0);
    let expressions = vec![
        RawExpressionSyntax {
            span: range("Vec<i32>", 2, ")", 1),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[2],
                open_paren_span: token("(", 1),
                open_bracket_span: token("[", 0),
                elements: Vec::new(),
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 1),
            },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: token("outer", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
            },
        },
        RawExpressionSyntax {
            span: one_span,
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("push", 0, ")", 3),
            kind: RawExpressionKind::VecPush {
                keyword_span: token("push", 0),
                open_paren_span: token("(", 3),
                vector: 2,
                comma_span: token(",", 0),
                value: 3,
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: token("outer", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 2) },
            },
        },
    ];
    let declaration_start = format!("{declaration} outer");
    let statements = vec![
        RawStatementSyntax {
            span: range(&declaration_start, 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token(declaration, 0),
                mutable,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: vec_types[1],
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 2),
                condition: 1,
                close_paren_span: token(")", 2),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("push", 0, ";", 1),
            kind: RawStatementKind::ExpressionStatement {
                expression: 4,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 2),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 5,
                semicolon_span: token(";", 2),
            },
        },
    ];
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "keep".to_owned(), span: token("keep", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 3],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2],
                    close_brace_span: token("}", 0),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn private_string_if_moves_outer_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;

    let (source, mut raw) = private_string_if_fixture();
    let source = source.replacen("\"a\"", "own", 1);
    let expression = &mut raw.files[0].functions[0].body.expressions[2];
    expression.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "own".to_owned(), span: expression.span },
    };
    (source, raw)
}

fn private_string_if_non_bool_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (source, mut raw) = private_string_if_fixture();
    let source = source.replacen("if (flag)", "if (own )", 1);
    let expression = &mut raw.files[0].functions[0].body.expressions[1];
    expression.span.end -= 1;
    expression.kind = zryna_syntax::v4::RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "own".to_owned(), span: expression.span },
    };
    (source, raw)
}

fn private_string_if_without_else_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = private_string_if_fixture();
    let clause = " else { const third: String = clone(own); }";
    let start = source.find(clause).expect("else clause");
    source.replace_range(start..start + clause.len(), &" ".repeat(clause.len()));
    let body = &mut raw.files[0].functions[0].body;
    let then_end = body.blocks[1].close_brace_span.end;
    let RawStatementKind::If { else_clause, .. } = &mut body.statements[1].kind else {
        panic!("if statement")
    };
    *else_clause = None;
    body.statements[1].span.end = then_end;
    body.blocks.truncate(2);
    body.statements.remove(4);
    body.statements[4].kind = match body.statements[4].kind.clone() {
        RawStatementKind::Return { keyword_span, semicolon_span, .. } => {
            RawStatementKind::Return { keyword_span, value: 4, semicolon_span }
        }
        _ => panic!("return statement"),
    };
    body.blocks[0].statements = vec![0, 1, 4];
    body.expressions.drain(4..6);
    raw.files[0].type_syntax.pop();
    (source, raw)
}

fn private_string_if_nested_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let (mut source, mut raw) = private_string_if_fixture();
    let original = "const first: String = \"a\";";
    let nested = "if (true) { }";
    let start = source.find(original).expect("first branch local");
    let replacement = format!("{nested}{}", " ".repeat(original.len() - nested.len()));
    source.replace_range(start..start + original.len(), &replacement);
    let nested_keyword = nth_untrusted_span(&source, "if", 1);
    let nested_open_paren = nth_untrusted_span(&source, "(", 2);
    let nested_close_paren = nth_untrusted_span(&source, ")", 2);
    let nested_open = nth_untrusted_span(&source, "{", 2);
    let nested_close = nth_untrusted_span(&source, "}", 0);
    let nested_span =
        zryna_source::UntrustedSpan { file: 0, start: nested_keyword.start, end: nested_close.end };
    raw.files[0].type_syntax.remove(3);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::If { else_clause: Some(RawElseSyntax { block, .. }), .. } =
        &mut body.statements[1].kind
    else {
        panic!("outer if")
    };
    *block = 3;
    body.statements[2] = RawStatementSyntax {
        span: nested_span,
        kind: RawStatementKind::If {
            keyword_span: nested_keyword,
            open_paren_span: nested_open_paren,
            condition: 2,
            close_paren_span: nested_close_paren,
            then_block: 2,
            else_clause: None,
        },
    };
    body.expressions[2] = RawExpressionSyntax {
        span: nth_untrusted_span(&source, "true", 0),
        kind: RawExpressionKind::BoolLiteral { value: true },
    };
    body.blocks.insert(
        2,
        RawBlockSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: nested_open.start,
                end: nested_close.end,
            },
            open_brace_span: nested_open,
            statements: Vec::new(),
            close_brace_span: nested_close,
        },
    );
    for statement in &mut body.statements[3..=4] {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind else {
            panic!("remaining branch local")
        };
        *type_syntax -= 1;
    }
    (source, raw)
}

#[allow(clippy::too_many_lines)]
fn private_vec_nested_string_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = VEC_PUSH_SOURCE.to_owned();
    let mut raw = response_snapshot(VEC_PUSH_RESPONSE);
    let replacements = [("\"a\"", "concat(\"a\", \"b\")"), ("\"b\"", "concat(\"c\", \"d\")")];
    for (ordinal, (old, replacement)) in replacements.into_iter().enumerate() {
        let found = if ordinal == 0 { source.find(old) } else { source.rfind(old) };
        let start = u32::try_from(found.expect("Vec String literal")).expect("offset");
        let old_end = start + u32::try_from(old.len()).expect("length");
        let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
        source.replace_range(
            usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
            replacement,
        );
        raw = shift_snapshot(raw, old_end, extra);
        let body = &mut raw.files[0].functions[0].body;
        let inner = if ordinal == 0 { 0 } else { 3 };
        let first = zryna_source::UntrustedSpan { file: 0, start: start + 7, end: start + 10 };
        body.expressions[inner] = RawExpressionSyntax {
            span: first,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: if ordinal == 0 { "\"a\"" } else { "\"c\"" }.to_owned(),
            },
        };
        let second_id = u32::try_from(body.expressions.len()).expect("second literal id");
        let second = zryna_source::UntrustedSpan { file: 0, start: start + 12, end: start + 15 };
        body.expressions.push(RawExpressionSyntax {
            span: second,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: if ordinal == 0 { "\"b\"" } else { "\"d\"" }.to_owned(),
            },
        });
        let concat_id = u32::try_from(body.expressions.len()).expect("concat id");
        body.expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start, end: start + 16 },
            kind: zryna_syntax::v4::RawExpressionKind::Call {
                callee: RawIdentifierSyntax {
                    text: "concat".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start, end: start + 6 },
                },
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 6,
                    end: start + 7,
                },
                arguments: vec![u32::try_from(inner).expect("inner id"), second_id],
                close_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 15,
                    end: start + 16,
                },
            },
        });
        if ordinal == 0 {
            let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } =
                &mut body.expressions[1].kind
            else {
                panic!("Vec construction")
            };
            *elements = vec![concat_id];
        } else {
            let zryna_syntax::v4::RawExpressionKind::VecPush { value, .. } =
                &mut body.expressions[4].kind
            else {
                panic!("Vec push")
            };
            *value = concat_id;
        }
    }
    let body = &mut raw.files[0].functions[0].body;
    let mut vec_construct = body.expressions[1].clone();
    let vector_reference = body.expressions[2].clone();
    let mut push = body.expressions[4].clone();
    let return_reference = body.expressions[5].clone();
    let mut first_concat = body.expressions[7].clone();
    let mut second_concat = body.expressions[9].clone();
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut first_concat.kind else {
        panic!("first concat")
    };
    *arguments = vec![0, 1];
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } =
        &mut vec_construct.kind
    else {
        panic!("Vec construction")
    };
    *elements = vec![2];
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut second_concat.kind
    else {
        panic!("second concat")
    };
    *arguments = vec![5, 6];
    let zryna_syntax::v4::RawExpressionKind::VecPush { vector, value, .. } = &mut push.kind else {
        panic!("Vec push")
    };
    *vector = 4;
    *value = 7;
    body.expressions = vec![
        body.expressions[0].clone(),
        body.expressions[6].clone(),
        first_concat,
        vec_construct,
        vector_reference,
        body.expressions[3].clone(),
        body.expressions[8].clone(),
        second_concat,
        push,
        return_reference,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[0].kind
    else {
        panic!("Vec declaration")
    };
    *initializer = 3;
    let RawStatementKind::ExpressionStatement { expression, .. } = &mut body.statements[1].kind
    else {
        panic!("push statement")
    };
    *expression = 8;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("Vec return")
    };
    *value = 9;
    (source, raw)
}

fn survivor_construct_type_span(types: &[RawTypeSyntax], id: u32) -> zryna_source::UntrustedSpan {
    types[usize::try_from(id).expect("type")].span
}

fn unresolved_push_without_vec_snapshot() -> RawProjectSyntaxSnapshot {
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![RawTypeSyntax {
                span: s(16, 19),
                kind: RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: "i32".to_owned(), span: s(16, 19) },
                },
            }],
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: s(0, 51),
                export_span: None,
                function_span: s(0, 8),
                name: RawIdentifierSyntax { text: "bad".to_owned(), span: s(9, 12) },
                parameters: Vec::new(),
                result_type: 0,
                body: RawFunctionBodySyntax {
                    span: s(20, 51),
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: s(20, 51),
                        open_brace_span: s(20, 21),
                        statements: vec![0, 1],
                        close_brace_span: s(50, 51),
                    }],
                    statements: vec![
                        RawStatementSyntax {
                            span: s(22, 39),
                            kind: RawStatementKind::ExpressionStatement {
                                expression: 2,
                                semicolon_span: s(38, 39),
                            },
                        },
                        RawStatementSyntax {
                            span: s(40, 49),
                            kind: RawStatementKind::Return {
                                keyword_span: s(40, 46),
                                value: 3,
                                semicolon_span: s(48, 49),
                            },
                        },
                    ],
                    expressions: vec![
                        RawExpressionSyntax {
                            span: s(27, 34),
                            kind: zryna_syntax::v4::RawExpressionKind::Reference {
                                name: RawIdentifierSyntax {
                                    text: "missing".to_owned(),
                                    span: s(27, 34),
                                },
                            },
                        },
                        RawExpressionSyntax {
                            span: s(36, 37),
                            kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                                spelling: "1".to_owned(),
                            },
                        },
                        RawExpressionSyntax {
                            span: s(22, 38),
                            kind: zryna_syntax::v4::RawExpressionKind::VecPush {
                                keyword_span: s(22, 26),
                                open_paren_span: s(26, 27),
                                vector: 0,
                                comma_span: s(34, 35),
                                value: 1,
                                close_paren_span: s(37, 38),
                            },
                        },
                        RawExpressionSyntax {
                            span: s(47, 48),
                            kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                                spelling: "0".to_owned(),
                            },
                        },
                    ],
                },
            }],
        }],
        diagnostics: Vec::new(),
    }
}

fn shift_snapshot(
    raw: RawProjectSyntaxSnapshot,
    cutoff: u32,
    amount: u32,
) -> RawProjectSyntaxSnapshot {
    fn visit(value: &mut serde_json::Value, cutoff: u32, amount: u32) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end")
                {
                    for key in ["start", "end"] {
                        if let Some(number) = object.get_mut(key) {
                            let current = u32::try_from(number.as_u64().expect("span number"))
                                .expect("u32 span");
                            if current >= cutoff {
                                *number = serde_json::Value::from(current + amount);
                            }
                        }
                    }
                } else {
                    for child in object.values_mut() {
                        visit(child, cutoff, amount);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, cutoff, amount);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    visit(&mut value, cutoff, amount);
    serde_json::from_value(value).expect("shifted snapshot")
}

fn shift_snapshot_signed(
    raw: RawProjectSyntaxSnapshot,
    cutoff: u32,
    amount: i32,
) -> RawProjectSyntaxSnapshot {
    fn visit(value: &mut serde_json::Value, cutoff: u32, amount: i32) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end")
                {
                    for key in ["start", "end"] {
                        let number = object.get_mut(key).expect("span field");
                        let current =
                            i64::try_from(number.as_u64().expect("span number")).expect("i64 span");
                        if current >= i64::from(cutoff) {
                            *number = serde_json::Value::from(
                                u64::try_from(current + i64::from(amount)).expect("shifted span"),
                            );
                        }
                    }
                } else {
                    for child in object.values_mut() {
                        visit(child, cutoff, amount);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, cutoff, amount);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    visit(&mut value, cutoff, amount);
    serde_json::from_value(value).expect("shifted snapshot")
}

fn pair_sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: PAIR_SOURCE.to_owned(),
    }])
    .expect("Pair source map")
}

fn pair_input<'a>(
    syntax: &'a zryna_syntax::v4::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
) -> SemanticInput<'a> {
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = sources.file_id(&path).expect("entry");
    SemanticInput::try_new(syntax, sources, entry).expect("authenticated Pair input")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnedFaultInjection {
    Runtime { operation: LogicalOperation, status: RuntimeStatus },
    VecCloneElement { status: RuntimeStatus, source_length: u64, completed_prefix: u64 },
    AggregateCloneElement { status: RuntimeStatus, completed_prefix: u64 },
    Bounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnedFaultDisposition {
    ControlledTrap(VerifiedTrapIdentity),
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedFaultTrace {
    kind: VerifiedInstructionKind,
    span: FaultSpan,
    block: u32,
    instruction: u32,
    disposition: OwnedFaultDisposition,
    result_committed: bool,
    uncommitted_result: Option<FaultValueIdentity>,
    retained_roots: Vec<FaultPlaceIdentity>,
    reverse_cleanup: Vec<FaultPlaceIdentity>,
    prefix_owner: Option<FaultPlaceIdentity>,
    reverse_prefix: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnedFaultOracleError {
    StatusMismatch,
    SuccessStatus,
    MissingPrepareCleanup,
    AtomicityMismatch,
    EventLimit,
    InvalidVecClonePrefix,
    InvalidAggregateClonePrefix,
}

fn runtime_operation(kind: VerifiedInstructionKind) -> Option<LogicalOperation> {
    match kind {
        VerifiedInstructionKind::StringFromUtf8 => Some(LogicalOperation::StringFromUtf8Copy),
        VerifiedInstructionKind::StringClone => Some(LogicalOperation::StringClone),
        VerifiedInstructionKind::StringConcat => Some(LogicalOperation::StringConcat),
        VerifiedInstructionKind::VecClone | VerifiedInstructionKind::VecConstruct => {
            Some(LogicalOperation::VecAllocate)
        }
        VerifiedInstructionKind::VecPush => Some(LogicalOperation::VecReserve),
        _ => None,
    }
}

fn runtime_fault_disposition(
    abi: &VerifiedOwnershipRuntimeAbi,
    status: RuntimeStatus,
) -> Option<OwnedFaultDisposition> {
    let declaration =
        abi.status_declarations().find(|declaration| declaration.status() == status)?;
    match (declaration.disposition(), declaration.trap_identity()) {
        (VerifiedStatusDisposition::ControlledTrap, Some(trap)) => {
            let identity = match trap {
                VerifiedStatusTrapIdentity::AllocationV1 => VerifiedTrapIdentity::AllocationV1,
                VerifiedStatusTrapIdentity::CapacityV1 => VerifiedTrapIdentity::CapacityV1,
                VerifiedStatusTrapIdentity::RefcountV1 => VerifiedTrapIdentity::RefcountV1,
                VerifiedStatusTrapIdentity::Utf8V1 => VerifiedTrapIdentity::Utf8V1,
            };
            Some(OwnedFaultDisposition::ControlledTrap(identity))
        }
        (VerifiedStatusDisposition::HostFailure, None) => Some(OwnedFaultDisposition::HostFailure),
        _ => None,
    }
}

fn owned_fault_root(
    function: VerifiedFunction<'_>,
    mut place: FaultPlaceIdentity,
) -> Result<FaultPlaceIdentity, OwnedFaultOracleError> {
    let limit = function.places().count();
    for _ in 0..=limit {
        let verified = function
            .places()
            .find(|candidate| candidate.id() == place)
            .ok_or(OwnedFaultOracleError::AtomicityMismatch)?;
        place = match verified.kind() {
            VerifiedPlaceKind::StructField { base, .. }
            | VerifiedPlaceKind::EnumPayload { base, .. }
            | VerifiedPlaceKind::FixedArrayConstant { base, .. } => base,
            VerifiedPlaceKind::Parameter(_)
            | VerifiedPlaceKind::Local(_)
            | VerifiedPlaceKind::Temporary(_) => return Ok(place),
        };
    }
    Err(OwnedFaultOracleError::AtomicityMismatch)
}

#[allow(clippy::too_many_lines)]
fn owned_fault_trace(
    abi: &VerifiedOwnershipRuntimeAbi,
    function: VerifiedFunction<'_>,
    instruction: FaultVerifiedInstruction<'_>,
    injection: OwnedFaultInjection,
    retained_events: usize,
    event_limit: usize,
) -> Result<OwnedFaultTrace, OwnedFaultOracleError> {
    let prefix_events = match injection {
        OwnedFaultInjection::VecCloneElement { source_length, completed_prefix, .. } => {
            if source_length > MAX_VEC_ELEMENTS || completed_prefix >= source_length {
                return Err(OwnedFaultOracleError::InvalidVecClonePrefix);
            }
            usize::try_from(completed_prefix).map_err(|_| OwnedFaultOracleError::EventLimit)?
        }
        OwnedFaultInjection::AggregateCloneElement { completed_prefix, .. } => {
            let leaf_count = instruction
                .aggregate_clone_fallible_leaf_count()
                .ok_or(OwnedFaultOracleError::StatusMismatch)?;
            if completed_prefix >= leaf_count {
                return Err(OwnedFaultOracleError::InvalidAggregateClonePrefix);
            }
            usize::try_from(completed_prefix).map_err(|_| OwnedFaultOracleError::EventLimit)?
        }
        _ => 0,
    };
    let new_events = prefix_events.checked_add(1).ok_or(OwnedFaultOracleError::EventLimit)?;
    if retained_events.checked_add(new_events).is_none_or(|total| total > event_limit) {
        return Err(OwnedFaultOracleError::EventLimit);
    }
    let kind = instruction.kind();
    let disposition = match injection {
        OwnedFaultInjection::Bounds if kind == VerifiedInstructionKind::VecIndexCopy => {
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::BoundsV1)
        }
        OwnedFaultInjection::Bounds => return Err(OwnedFaultOracleError::StatusMismatch),
        OwnedFaultInjection::Runtime { status: RuntimeStatus::Ok, .. }
        | OwnedFaultInjection::VecCloneElement { status: RuntimeStatus::Ok, .. }
        | OwnedFaultInjection::AggregateCloneElement { status: RuntimeStatus::Ok, .. } => {
            return Err(OwnedFaultOracleError::SuccessStatus);
        }
        OwnedFaultInjection::Runtime { operation, status } => {
            let Some(expected) = runtime_operation(kind) else {
                return Err(OwnedFaultOracleError::StatusMismatch);
            };
            if operation != expected || !operation_accepts_status(operation, status) {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(operation, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
        OwnedFaultInjection::VecCloneElement { status, .. } => {
            if kind != VerifiedInstructionKind::VecClone
                || !operation_accepts_status(LogicalOperation::StringClone, status)
            {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(LogicalOperation::StringClone, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
        OwnedFaultInjection::AggregateCloneElement { status, .. } => {
            if kind != VerifiedInstructionKind::ClonePlace
                || !operation_accepts_status(LogicalOperation::StringClone, status)
            {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(LogicalOperation::StringClone, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
    };
    let vec_element_failure = matches!(injection, OwnedFaultInjection::VecCloneElement { .. });
    let aggregate_element_failure =
        matches!(injection, OwnedFaultInjection::AggregateCloneElement { .. });
    let element_failure = vec_element_failure || aggregate_element_failure;
    let cleanup = if vec_element_failure {
        instruction
            .vec_clone_element_cleanup()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    } else if aggregate_element_failure {
        instruction
            .aggregate_clone_element_cleanup()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    } else {
        instruction.cleanup().ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    };
    let plan = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?;
    let site = plan.site();
    let expected_role = if vec_element_failure {
        VerifiedCleanupRole::VecCloneElementFailure
    } else if aggregate_element_failure {
        VerifiedCleanupRole::AggregateCloneElementFailure
    } else {
        VerifiedCleanupRole::PrepareFailure
    };
    if site.role() != expected_role {
        return Err(OwnedFaultOracleError::MissingPrepareCleanup);
    }
    let actions = if vec_element_failure {
        instruction.vec_clone_element_failure_drop_actions().collect::<Vec<_>>()
    } else if aggregate_element_failure {
        instruction.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>()
    } else {
        instruction.derived_drop_actions().collect::<Vec<_>>()
    };
    let (prefix_owner, reverse_cleanup) = if element_failure {
        let Some((prefix, remaining)) = actions.split_first() else {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        };
        let expected_prefix = if vec_element_failure {
            VerifiedDropActionKind::VecInitializedPrefix
        } else {
            VerifiedDropActionKind::AggregateInitializedPrefix
        };
        if prefix.kind() != expected_prefix
            || remaining.iter().any(|action| action.kind() != VerifiedDropActionKind::Place)
        {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
        (
            Some(prefix.root()),
            remaining
                .iter()
                .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
                .collect::<Vec<_>>(),
        )
    } else {
        if actions.iter().any(|action| action.kind() != VerifiedDropActionKind::Place) {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
        (
            None,
            actions
                .iter()
                .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
                .collect::<Vec<_>>(),
        )
    };
    let mut retained_roots = Vec::new();
    for place in instruction.place_operands() {
        let owner = owned_fault_root(function, place)?;
        if !retained_roots.contains(&owner) {
            retained_roots.push(owner);
        }
    }
    for value in instruction.value_operands() {
        let candidates = function
            .places()
            .filter(|place| place.kind() == VerifiedPlaceKind::Temporary(value))
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [] => {}
            [candidate] if candidate.is_copy() => {}
            [candidate] => {
                let owner = candidate.id();
                if !retained_roots.contains(&owner) {
                    retained_roots.push(owner);
                }
            }
            _ => return Err(OwnedFaultOracleError::AtomicityMismatch),
        }
    }
    if retained_roots.iter().any(|owner| !reverse_cleanup.contains(owner)) {
        return Err(OwnedFaultOracleError::AtomicityMismatch);
    }
    if let Some(result) = instruction.result()
        && function.places().any(|place| {
            place.kind() == VerifiedPlaceKind::Temporary(result)
                && reverse_cleanup.contains(&place.id())
        })
    {
        return Err(OwnedFaultOracleError::AtomicityMismatch);
    }
    if let (Some(result), Some(prefix)) = (instruction.result(), prefix_owner) {
        let matches_result = function.places().any(|place| {
            place.kind() == VerifiedPlaceKind::Temporary(result) && place.id() == prefix
        });
        if !matches_result {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
    }
    let reverse_prefix = match injection {
        OwnedFaultInjection::VecCloneElement { completed_prefix, .. }
        | OwnedFaultInjection::AggregateCloneElement { completed_prefix, .. } => {
            (0..completed_prefix).rev().collect()
        }
        _ => Vec::new(),
    };
    Ok(OwnedFaultTrace {
        kind,
        span: instruction.span(),
        block: site.block().index(),
        instruction: site
            .instruction_index()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?,
        disposition,
        result_committed: false,
        uncommitted_result: instruction.result(),
        retained_roots,
        reverse_cleanup,
        prefix_owner,
        reverse_prefix,
    })
}

fn assert_all_runtime_faults(
    abi: &VerifiedOwnershipRuntimeAbi,
    function: VerifiedFunction<'_>,
    instruction: FaultVerifiedInstruction<'_>,
    operation: LogicalOperation,
    expected: &[(RuntimeStatus, OwnedFaultDisposition)],
) {
    let all = [
        RuntimeStatus::Allocation,
        RuntimeStatus::Capacity,
        RuntimeStatus::Refcount,
        RuntimeStatus::Utf8,
        RuntimeStatus::Expired,
        RuntimeStatus::AbiViolation,
    ];
    let admitted = all
        .into_iter()
        .filter(|status| operation_accepts_status(operation, *status))
        .collect::<Vec<_>>();
    assert_eq!(admitted, expected.iter().map(|(status, _)| *status).collect::<Vec<_>>());
    for &(status, expected_disposition) in expected {
        let injection = OwnedFaultInjection::Runtime { operation, status };
        let first = owned_fault_trace(abi, function, instruction, injection, 0, 1)
            .expect("admitted runtime fault");
        let second = owned_fault_trace(abi, function, instruction, injection, 0, 1)
            .expect("deterministic admitted runtime fault");
        assert_eq!(first, second);
        assert_eq!(first.kind, instruction.kind());
        assert_eq!(first.span, instruction.span());
        assert_eq!(first.disposition, expected_disposition);
        assert!(!first.result_committed);
        assert_eq!(first.uncommitted_result, instruction.result());
        assert!(
            first.retained_roots.iter().all(|owner| first.reverse_cleanup.contains(owner)),
            "every precommit operand owner remains cleanup-authoritative"
        );
    }
}

#[test]
fn private_owned_struct_prepares_in_source_order_and_commits_in_declaration_order() {
    let sources = sources_for(OWNED_PAIR_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_PAIR_RESPONSE), &sources)
        .expect("source-faithful owned Pair v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Pair must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::BoolLiteral,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    assert_eq!(
        instructions[2]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![1, 0],
        "constructor operands follow declaration order after source-order preparation",
    );
    assert_eq!(instructions[1].derived_drop_actions().count(), 0);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn string_bearing_struct_clone_retains_source_and_seals_recursive_prefix_cleanup() {
    let (source, raw) = clone_final_return_snapshot(OWNED_PAIR_SOURCE, OWNED_PAIR_RESPONSE);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful aggregate clone v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned aggregate clone must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::BoolLiteral,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::ClonePlace,
        ]
    );
    let clone = instructions[4];
    let source_owner = clone.place_operands().next().expect("source owner");
    let result_owner = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if Some(value) == clone.result()))
        .expect("distinct clone result owner")
        .id();
    assert_ne!(source_owner, result_owner);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![source_owner],
    );
    let failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(failure.len(), 2);
    assert_eq!(failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(failure[0].root(), result_owner);
    assert_eq!(failure[1].kind(), VerifiedDropActionKind::Place);
    assert_eq!(failure[1].root(), source_owner);
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![source_owner],
        "successful return transfers only the distinct clone and retains the source",
    );
}

#[test]
fn string_bearing_fixed_array_and_enum_clone_use_the_same_recursive_failure_authority() {
    for (source, response, label, leaf_count, active_variant) in [
        (OWNED_ARRAY_SOURCE, OWNED_ARRAY_RESPONSE, "fixed array", 2, None),
        (OWNED_ENUM_STRING_SOURCE, OWNED_ENUM_STRING_RESPONSE, "active enum variant", 1, Some(1)),
    ] {
        let (source, raw) = clone_final_return_snapshot(source, response);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful structural clone v4");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let clone = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
            .expect("structural clone");
        let source_owner = clone.place_operands().next().expect("source owner");
        let result_owner = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if Some(value) == clone.result())
            })
            .expect("clone owner")
            .id();
        let failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(leaf_count),);
        assert_eq!(failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
        assert_eq!(failure[0].root(), result_owner);
        assert_eq!(failure[0].active_variant(), active_variant);
        assert_eq!(
            failure[0]
                .active_variants()
                .find(|variant| variant.place() == result_owner)
                .map(VerifiedActiveVariant::variant),
            active_variant,
        );
        assert!(failure.iter().skip(1).any(|action| action.root() == source_owner));
        assert!(
            block.terminator().derived_drop_actions().any(|action| action.root() == source_owner)
        );
    }
}

#[test]
fn string_bearing_struct_assignment_prepares_before_replacing_the_exact_root() {
    let (source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::Fresh, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful aggregate assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    let replace = instructions[replace_index];
    let target = replace.place_operands().next().expect("replacement target");
    let prepared_string = instructions[..replace_index]
        .iter()
        .rev()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("prepared String leaf");
    assert!(
        prepared_string.derived_drop_actions().any(|action| action.root() == target),
        "fallible RHS preparation retains the old aggregate root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    assert_eq!(replace_index + 2, instructions.len(), "commit precedes only the final return move");
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_string_assignment_prepares_before_replacing_the_exact_leaf() {
    let (source, raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected ReplacePlace");
    let replace = instructions[replace_index];
    let target = replace.place_operands().next().expect("projected replacement target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
    ));
    let prepared = instructions[..replace_index]
        .iter()
        .rev()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("prepared replacement String");
    assert!(
        prepared.derived_drop_actions().any(|action| action.root() == root),
        "fallible preparation retains the enclosing aggregate root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the replaced String leaf",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_string_clone_assignment_prepares_before_replacing_the_exact_leaf() {
    let (source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("source-faithful projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected StringClone");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected ReplacePlace");
    assert!(clone_index < replace_index, "clone preparation precedes assignment commit");
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source = clone.place_operands().next().expect("projected clone source");
    let target = replace.place_operands().next().expect("projected replacement target");
    assert_eq!(source, target, "self-clone reads the exact leaf replaced at commit");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [root],
        "fallible clone preparation retains the enclosing root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the exact old leaf",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_string_clone_fault_trace_retains_the_enclosing_root() {
    let (source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected clone fault");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected StringClone");
    let first = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("authenticated projected clone fault");
    let replay = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("deterministic projected clone fault");
    assert_eq!(first, replay);
    assert!(!first.result_committed);
    assert_eq!(first.uncommitted_result, clone.result());
    assert_eq!(first.retained_roots, [root]);
    assert_eq!(first.reverse_cleanup, [root]);
}

#[test]
fn fixed_array_projected_string_assignment_uses_the_same_exact_leaf_commit() {
    let (source, raw) = owned_array_projected_string_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("array String-leaf assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let replace = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("array projected ReplacePlace");
    let target = replace.place_operands().next().expect("array replacement target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == root
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
}

#[test]
fn fixed_array_projected_string_clone_reads_and_replaces_the_exact_element() {
    let (source, raw) = owned_array_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("array String clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected array StringClone");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("array projected ReplacePlace");
    assert!(clone_index < replace_index);
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source = clone.place_operands().next().expect("array clone source");
    let target = replace.place_operands().next().expect("array replacement target");
    assert_eq!(source, target);
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == root
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [root],
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
}

#[test]
fn projected_string_assignment_rejects_immutable_and_self_consuming_targets() {
    for (rhs, mutable, needle, reference_ordinal, label) in [
        (OwnedPairProjectedStringAssignmentRhs::Fresh, false, "p.first", 0, "immutable projection"),
        (
            OwnedPairProjectedStringAssignmentRhs::TargetMove,
            true,
            "p",
            2,
            "self-consuming projection",
        ),
    ] {
        let (source, raw) = owned_pair_projected_string_assignment_snapshot(rhs, mutable);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful rejected projection");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3014", "{label}");
        assert_eq!(
            diagnostics[0].primary_span(),
            Some(span(&sources, nth_untrusted_span(&source, needle, reference_ordinal))),
            "{label}",
        );
    }
}

#[test]
fn projected_string_assignment_does_not_reinitialize_a_moved_leaf() {
    let (source, raw) = owned_pair_moved_projection_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved projection");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved projection target");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "p.first", 1))),
    );
}

#[test]
fn projected_string_assignment_rejects_a_copy_leaf_target_as_m3013() {
    let (source, raw) = owned_pair_copy_projection_assignment_target_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Copy target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Copy target shape");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "p.flag", 0))),
    );
}

#[test]
fn aggregate_clone_target_assignment_retains_source_until_replace_commit() {
    let (source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::CloneTarget, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful clone-target assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("ClonePlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(clone_index < replace_index, "clone preparation must precede commit");
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source_owner = clone.place_operands().next().expect("clone source");
    assert_eq!(replace.place_operands().next(), Some(source_owner));
    assert!(clone.derived_drop_actions().any(|action| action.root() == source_owner));
    assert!(
        clone
            .aggregate_clone_element_failure_drop_actions()
            .any(|action| action.root() == source_owner),
        "recursive clone failure retains the assignment target",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source_owner],
        "the old source is dropped only by the replacement commit",
    );
}

#[test]
fn partial_struct_assignment_prepares_then_replaces_and_returns_the_exact_mask() {
    let (source, raw) = owned_pair_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf = roots[&1];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial assignment preparation");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assignment commit");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assigned-root return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
        "the fully initialized old destination is dropped exactly at commit",
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("partial assignment temporary")
        .id();
    let returned_value = instructions[return_move].result().expect("return value");
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned_value)
        })
        .expect("partial return temporary")
        .id();
    let fields = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        assert_eq!(fields(root).keys().copied().collect::<Vec<_>>(), [0, 1]);
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
        "only the moved String leaf survives reverse return cleanup",
    );
}

#[test]
fn partial_struct_assignment_invalidates_the_source_owner() {
    let (source, raw) = owned_pair_partial_assignment_old_source_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful old assignment source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("assigned source must move");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("assigned source replay");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "aggregate value 'p' is moved or only partially available",
    );
}

#[test]
fn partial_struct_assignment_rejects_a_partial_destination_before_rhs_mutation() {
    let (source, raw) = owned_pair_partial_self_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial destination");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("partial destination");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("partial destination replay");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "owned aggregate assignment target is immutable, moved, or only partially available",
    );
}

#[test]
fn partial_fixed_array_assignment_preserves_exact_elements_and_old_value_drop() {
    let (source, raw) = owned_array_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial array assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf = roots[&1];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array assignment move");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial array assignment replacement");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assigned array return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let returned_value = instructions[return_move].result().expect("return value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("partial array assignment temporary")
        .id();
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned_value)
        })
        .expect("partial array return temporary")
        .id();
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        assert_eq!(
            function
                .places()
                .filter_map(|place| match place.kind() {
                    VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                        Some(index)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0, 1],
        );
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
    );
}

#[test]
fn nested_partial_struct_assignment_preserves_recursive_topology_and_cleanup() {
    let (source, raw) = nested_owned_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let moved_leaf = roots[&1];
    let source_root = roots[&2];
    let target_root = roots[&3];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("nested assignment preparation");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("nested assignment replacement");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("nested assignment return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let return_value = instructions[return_move].result().expect("return value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("nested assignment temporary")
        .id();
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == return_value)
        })
        .expect("nested return temporary")
        .id();
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        assert_eq!(
            function
                .places()
                .filter_map(|place| match place.kind() {
                    VerifiedPlaceKind::StructField { base, ordinal } if base == fields[&0] => {
                        Some(ordinal)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0],
        );
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
    );
}

#[test]
fn string_fixed_array_clone_assignment_replaces_one_mutable_whole_root() {
    let (source, raw) = owned_fixed_array_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful FixedArray assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("owned FixedArray assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("ClonePlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(clone_index < replace_index);
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let target = clone.place_operands().next().expect("array source root");
    assert_eq!(replace.place_operands().next(), Some(target));
    assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(2));
    assert!(clone.derived_drop_actions().any(|action| action.root() == target));
    assert!(
        clone.aggregate_clone_element_failure_drop_actions().any(|action| action.root() == target),
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn owned_struct_projections_copy_and_move_with_a_root_relative_cleanup_mask() {
    let (source, raw) = owned_pair_projected_return_snapshot("first");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned Struct projections");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct projections");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let first = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
            )
        })
        .expect("String field projection")
        .id();
    let flag = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 1 } if base == root
            )
        })
        .expect("Copy field projection")
        .id();
    let block = function.blocks().next().expect("block");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::CopyFromPlace
            && instruction.place_operands().next() == Some(flag)
    }));
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next() == Some(first)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved root cleanup");
    assert_eq!(
        cleanup.moved_projections().map(FaultPlaceIdentity::index).collect::<Vec<_>>(),
        vec![first.index()],
    );
    assert_eq!(
        cleanup.initialized_projections().map(FaultPlaceIdentity::index).collect::<Vec<_>>(),
        vec![flag.index()],
    );
}

#[test]
fn partial_struct_owner_transfers_through_temporary_into_one_local() {
    let (source, raw) = owned_pair_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial local transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct local transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let source_first = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal }
                    if base == source_root && ordinal == 0
            )
        })
        .expect("source first field")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let projected_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_first)
        })
        .expect("projected String move");
    let whole_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial whole-root move");
    let initialize = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial target initialization");
    assert!(projected_move < whole_move && whole_move < initialize);
    let transfer_value = instructions[whole_move].result().expect("whole transfer value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == transfer_value)
        })
        .expect("partial transfer temporary")
        .id();
    for root in [source_root, temporary, target_root] {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => Some(ordinal),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(fields, [0, 1], "complete exact topology for root {root:?}");
    }
    let target_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == target_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == target_root)
        .expect("transferred partial target cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_fields[&0]]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_fields[&1]]);
    assert!(
        block
            .terminator()
            .derived_drop_actions()
            .all(|action| { action.root() != source_root && action.root() != temporary })
    );
}

#[test]
fn partial_struct_transfer_invalidates_the_old_source_owner() {
    let (source, raw) = owned_pair_partial_transfer_then_use_source_snapshot();
    let use_start = source.find("flag: p.flag").expect("old source use") + "flag: ".len();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful old source use");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("old source must be moved");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("old source replay must be moved");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "owned projection is unavailable or overlaps an already moved subobject"
    );
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: 0,
                start: u32::try_from(use_start).expect("old source offset"),
                end: u32::try_from(use_start + 6).expect("old source end"),
            },
        ))
    );
}

#[test]
fn partial_struct_owner_can_transfer_repeatedly_without_mask_drift() {
    let (source, raw) = owned_pair_repeated_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful repeated partial transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("repeated partial transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let final_root = roots[&3];
    let final_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == final_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let block = function.blocks().next().expect("block");
    assert_eq!(
        block
            .instructions()
            .filter(|instruction| {
                instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                    && instruction
                        .place_operands()
                        .next()
                        .is_some_and(|place| place == roots[&0] || place == roots[&2])
            })
            .count(),
        2
    );
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == final_root)
        .expect("final repeated-transfer cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [final_fields[&0]]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [final_fields[&1]]);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() == final_root));
}

#[test]
fn nested_partial_struct_transfer_preserves_recursive_topology_and_mask() {
    let (source, raw) = nested_owned_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested partial transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("nested partial whole-root move");
    let temporary = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::Temporary(value) if Some(value) == whole_move.result()
            )
        })
        .expect("nested partial transfer temporary")
        .id();
    let topology = |root| {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        let inner = fields[&0];
        let tail = fields[&1];
        let inner_fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == inner => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(inner_fields.keys().copied().collect::<Vec<_>>(), [0]);
        let text = inner_fields[&0];
        (inner, text, tail)
    };
    let source_topology = topology(source_root);
    let temporary_topology = topology(temporary);
    let target_topology = topology(target_root);
    assert_ne!(source_topology, temporary_topology);
    assert_ne!(temporary_topology, target_topology);
    let cleanup_actions = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    let cleanup = cleanup_actions
        .iter()
        .find(|action| action.root() == target_root)
        .expect("nested transferred-root cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_topology.1]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_topology.2]);
    assert!(
        cleanup_actions
            .iter()
            .all(|action| action.root() != source_root && action.root() != temporary)
    );
}

#[test]
fn static_struct_subobject_moves_into_one_exact_direct_local() {
    let sources = sources_for(PROJECTED_INNER_MOVE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(PROJECTED_INNER_MOVE_RESPONSE), &sources)
        .expect("source-faithful projected Inner move");
    let program = lower(pair_input(&syntax, &sources)).expect("projected Inner move");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_local = roots[&1];
    let source_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == source_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(source_fields.keys().copied().collect::<Vec<_>>(), [0]);
    let moved_inner = source_fields[&0];
    let moved_text = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal }
                if base == moved_inner && ordinal == 0 =>
            {
                Some(place.id())
            }
            _ => None,
        })
        .expect("moved Inner text projection");
    let block = function.blocks().next().expect("block");
    let projection_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(moved_inner)
        })
        .expect("projected aggregate move");
    let moved_value = projection_move.result().expect("projected move result");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::InitializePlace
            && instruction.place_operands().next() == Some(moved_local)
            && instruction.value_operands().next() == Some(moved_value)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == source_root)
        .expect("partial source cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved_inner, moved_text]);
    assert_eq!(cleanup.initialized_projections().count(), 0);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() != moved_local));
}

#[test]
fn static_fixed_array_subobject_move_preserves_the_disjoint_element() {
    let sources = sources_for(PROJECTED_ARRAY_ELEMENT_MOVE_SOURCE);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_ARRAY_ELEMENT_MOVE_RESPONSE), &sources)
            .expect("source-faithful projected fixed-array element move");
    let program = lower(pair_input(&syntax, &sources)).expect("projected fixed-array move");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_local = roots[&1];
    let elements = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == source_root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(elements.keys().copied().collect::<Vec<_>>(), [0]);
    let block = function.blocks().next().expect("block");
    let moved_element = elements[&0];
    let moved_text = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal }
                if base == moved_element && ordinal == 0 =>
            {
                Some(place.id())
            }
            _ => None,
        })
        .expect("moved array element text projection");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next() == Some(moved_element)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == source_root)
        .expect("partial array cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved_element, moved_text]);
    assert_eq!(cleanup.initialized_projections().count(), 0);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() != moved_local));
}

#[test]
#[allow(clippy::too_many_lines)]
fn static_projected_aggregate_clone_initializes_one_exact_local_and_retains_source() {
    for (source, response, label) in [
        (PROJECTED_INNER_MOVE_SOURCE, PROJECTED_INNER_MOVE_RESPONSE, "StructField"),
        (
            PROJECTED_ARRAY_ELEMENT_MOVE_SOURCE,
            PROJECTED_ARRAY_ELEMENT_MOVE_RESPONSE,
            "FixedArrayConstant",
        ),
    ] {
        let (source, raw) = projected_aggregate_clone_local_snapshot(source, response);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected clone");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let abi = program.runtime_abi();
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let roots = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let source_root = roots[&0];
        let cloned_local = roots[&1];
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let clone_index = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
            .expect("projected aggregate clone");
        let clone = instructions[clone_index];
        let projected = clone.place_operands().next().expect("projected source");
        let source_kind = function
            .places()
            .find(|place| place.id() == projected)
            .expect("projected place")
            .kind();
        assert!(
            matches!(
                (label, source_kind),
                ("StructField", VerifiedPlaceKind::StructField { base, ordinal: 0 })
                    if base == source_root
            ) || matches!(
                (label, source_kind),
                (
                    "FixedArrayConstant",
                    VerifiedPlaceKind::FixedArrayConstant { base, index: 0 }
                ) if base == source_root
            )
        );
        let result = clone.result().expect("clone result");
        let temporary = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == result)
            })
            .expect("clone temporary")
            .id();
        let initialize = instructions.get(clone_index + 1).expect("immediate local initialization");
        assert_eq!(initialize.kind(), VerifiedInstructionKind::InitializePlace);
        assert_eq!(initialize.place_operands().next(), Some(cloned_local));
        assert_eq!(initialize.value_operands().next(), Some(result));
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(1));
        assert_eq!(
            clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
            [source_root],
        );
        let element_failure =
            clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
        assert_eq!(element_failure[0].root(), temporary);
        assert_eq!(element_failure[1].kind(), VerifiedDropActionKind::Place);
        assert_eq!(element_failure[1].root(), source_root);
        let first = owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 0,
            },
            0,
            1,
        )
        .expect("projected aggregate clone fault");
        let replay = owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 0,
            },
            0,
            1,
        )
        .expect("deterministic projected aggregate clone fault");
        assert_eq!(first, replay);
        assert!(!first.result_committed);
        assert_eq!(first.prefix_owner, Some(temporary));
        assert_eq!(first.retained_roots, [source_root]);
        assert_eq!(first.reverse_cleanup, [source_root]);
    }
}

#[test]
fn projected_aggregate_clone_stays_excluded_from_direct_return() {
    let (source, raw) = projected_aggregate_clone_direct_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful direct projected clone");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("direct projected clone return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "o.inner", 0),)),
    );
}

#[test]
fn projected_aggregate_assignment_moves_one_complete_root_into_a_static_field() {
    let sources = sources_for(PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected aggregate assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected aggregate assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected aggregate replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(moved.place_operands().next(), Some(source));
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("static target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = moved.result().and_then(|value| {
        function.places().find(
            |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
        )
    });
    assert!(temporary.is_some(), "whole-root move has one distinct temporary owner");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the old projected aggregate subtree",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(instructions[replace_index + 1].place_operands().next(), Some(destination));
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_aggregate_assignment_clones_one_complete_root_into_a_static_field() {
    let (source, raw) = projected_aggregate_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful clone assignment");
    let program =
        lower(pair_input(&syntax, &sources)).expect("projected aggregate clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected aggregate replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    assert_eq!(clone.place_operands().next(), Some(source));
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("static target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
        "clone allocation failure retains source and destination",
    );
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        element_failure[1..]
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [source, destination],
        "prefix failure retains both pre-existing roots",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the old projected subtree",
    );
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source],
        "successful clone retains the source root until function exit",
    );
}

#[test]
fn projected_aggregate_assignment_moves_one_static_subobject_between_distinct_roots() {
    let source = PROJECTED_SUBOBJECT_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected subobject assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected subobject assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    let source_projection = moved.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source projection place")
            .kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == source
    ));
    let source_leaf = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 }
                    if base == source_projection
            )
        })
        .expect("complete source descendant topology")
        .id();
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target subtree",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source]
    );
    assert_eq!(
        exit[0].moved_projections().collect::<Vec<_>>(),
        [source_projection, source_leaf],
        "source parent remains pending with the complete moved subtree masked",
    );
}

#[test]
fn projected_aggregate_assignment_moves_one_fixed_array_element_between_distinct_roots() {
    let source = FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful fixed-array subobject assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("fixed-array subobject assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("fixed-array projected replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    let source_projection = moved.place_operands().next().expect("source array projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source array projection place")
            .kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 1 } if base == source
    ));
    let source_leaf = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 }
                    if base == source_projection
            )
        })
        .expect("complete source array element topology")
        .id();
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("target array projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == destination
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target array element",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source]
    );
    assert_eq!(
        exit[0].moved_projections().collect::<Vec<_>>(),
        [source_projection, source_leaf],
        "source array remains pending with the moved element subtree masked",
    );
}

#[test]
fn projected_aggregate_assignment_clones_one_struct_subobject_between_distinct_roots() {
    let source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected clone replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    let source_projection = clone.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source projection place")
            .kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == source
    ));
    assert!(
        !function.places().any(|place| matches!(
            place.kind(),
            VerifiedPlaceKind::StructField { base, .. } if base == source_projection
        )),
        "projected clone materializes only the canonical source path",
    );
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
        "prepare failure retains source and destination roots",
    );
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        element_failure[1..]
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [source, destination],
        "prefix failure retains both pre-existing roots",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target subtree",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source],
    );
    assert_eq!(
        exit[0].moved_projections().count(),
        0,
        "successful clone retains the complete source subtree",
    );
}

#[test]
fn projected_aggregate_assignment_clones_one_fixed_array_subobject_between_distinct_roots() {
    let source = FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax = verify_snapshot(
        response_snapshot(FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE),
        &sources,
    )
    .expect("source-faithful fixed-array projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("fixed-array projected clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("fixed-array projected clone replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    let source_projection = clone.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source place")
            .kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 1 } if base == source
    ));
    assert!(
        !function.places().any(|place| matches!(
            place.kind(),
            VerifiedPlaceKind::StructField { base, .. } if base == source_projection
        )),
        "fixed-array clone does not materialize descendant places",
    );
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == destination
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
    );
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source],
    );
    assert_eq!(exit[0].moved_projections().count(), 0);
}

#[test]
fn projected_aggregate_clone_local_then_assignment_rejects_the_second_global_site() {
    let (source, raw) = two_projected_aggregate_clone_sites_snapshot(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful combined projected clones");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("second projected aggregate clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "dst.inner = clone(src.inner);", 0),)),
    );
}

#[test]
fn projected_aggregate_clone_assignment_then_local_rejects_the_second_global_site() {
    let (source, raw) = two_projected_aggregate_clone_sites_snapshot(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed projected clones");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("second projected aggregate clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "clone(src.inner)", 1),)),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_a_same_root_source() {
    let mut source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end().to_owned();
    let source_start = source.rfind("src.inner").expect("clone source projection");
    source.replace_range(source_start..source_start + "src".len(), "dst");
    let mut raw = response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { value: clone, .. } = body.statements[2].kind else {
        panic!("assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::Clone { value: operand, .. } =
        body.expressions[clone as usize].kind
    else {
        panic!("clone")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { base, .. } =
        body.expressions[operand as usize].kind
    else {
        panic!("source projection")
    };
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut body.expressions[base as usize].kind
    else {
        panic!("source root")
    };
    name.text = "dst".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful same-root clone");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("same-root clone source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "dst.inner", 1))),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_a_moved_source_subtree() {
    let source = PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_RESPONSE), &sources)
            .expect("source-faithful moved projected clone source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved clone source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(source, "src.inner", 1))),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_function_parameters() {
    let source = PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax = verify_snapshot(
        response_snapshot(PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_RESPONSE),
        &sources,
    )
    .expect("source-faithful parameterized projected clone");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("parameterized clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(source, "clone(src.inner)", 0))),
    );
}

#[test]
fn projected_aggregate_assignment_rejects_a_same_root_projected_source() {
    let mut source = PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE.to_owned();
    let start = source.rfind("replacement").expect("assignment source");
    let end = start + "replacement".len();
    source.replace_range(start..end, "o.inner");
    let start = u32::try_from(start).expect("source start");
    let end = u32::try_from(end).expect("source end");
    let mut raw = shift_snapshot_signed(
        response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE),
        end,
        i32::try_from("o.inner".len()).expect("replacement length")
            - i32::try_from("replacement".len()).expect("source length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    body.expressions[8] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    let projected = u32::try_from(body.expressions.len()).expect("projected source");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 8,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: start + 1, end: start + 2 },
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 2, end: start + 7 },
            },
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("assignment")
    };
    *value = projected;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("same-root projected source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "o.inner", 1))),
    );
}

#[test]
fn static_subobject_move_rejects_the_wrong_exact_contextual_type() {
    let mut source = PROJECTED_INNER_MOVE_SOURCE.to_owned();
    let type_start =
        source.find("const moved: Inner").expect("moved declaration") + "const moved: ".len();
    source.replace_range(type_start..type_start + "Inner".len(), "Outer");
    let mut raw = response_snapshot(PROJECTED_INNER_MOVE_RESPONSE);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[5].kind else {
        panic!("moved local named type")
    };
    name.text = "Outer".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong projected type");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong projected type");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
}

#[test]
fn static_subobject_move_rejects_child_reuse_after_parent_transfer() {
    let (source, raw) = projected_inner_child_after_parent_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful child-after-parent use");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("child after aggregate parent move");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
}

#[test]
fn complete_static_subobject_moves_directly_into_the_final_return() {
    for (source, response, label) in [
        (
            PROJECTED_INNER_DIRECT_RETURN_SOURCE,
            PROJECTED_INNER_DIRECT_RETURN_RESPONSE,
            "StructField",
        ),
        (
            FIXED_ARRAY_SUBOBJECT_RETURN_SOURCE.trim_end(),
            FIXED_ARRAY_SUBOBJECT_RETURN_RESPONSE,
            "FixedArrayConstant",
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful direct projected return");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let source_root = function
            .places()
            .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
            .expect("source root")
            .id();
        let block = function.blocks().next().expect("block");
        let projection_move = block
            .instructions()
            .find(|instruction| {
                instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                    && instruction.place_operands().next().is_some_and(|place| {
                        function.places().find(|candidate| candidate.id() == place).is_some_and(
                            |source| match (label, source.kind()) {
                                (
                                    "StructField",
                                    VerifiedPlaceKind::StructField { base, ordinal },
                                ) => base == source_root && ordinal == 0,
                                (
                                    "FixedArrayConstant",
                                    VerifiedPlaceKind::FixedArrayConstant { base, index },
                                ) => base == source_root && index == 0,
                                _ => false,
                            },
                        )
                    })
            })
            .expect("projected aggregate move");
        let returned = block.terminator().value_operands().next().expect("returned value");
        assert_eq!(projection_move.result(), Some(returned));
        let returned_owner = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == returned)
            })
            .expect("returned temporary")
            .id();
        let source_cleanup = block
            .terminator()
            .derived_drop_actions()
            .find(|action| action.root() == source_root)
            .expect("masked source cleanup");
        let moved = source_cleanup.moved_projections().collect::<Vec<_>>();
        assert_eq!(moved.len(), 2, "{label}");
        assert!(
            block.terminator().derived_drop_actions().all(|action| action.root() != returned_owner)
        );
    }
}

#[test]
fn direct_static_subobject_return_rejects_parameters_before_lowering_the_move() {
    let (source, raw) = projected_aggregate_direct_return_with_parameter_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful parameterized return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("parameterized return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "return o.inner;", 0)))
    );
}

#[test]
fn direct_static_subobject_return_rejects_a_nonfinal_first_site_before_lowering() {
    const INSERTION: u32 = 231;
    const SUFFIX: &str = " return o.inner;";
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    source.insert_str(usize::try_from(INSERTION).expect("insertion"), SUFFIX);
    let mut raw = shift_snapshot(
        response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE),
        INSERTION,
        u32::try_from(SUFFIX.len()).expect("suffix length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let reference = u32::try_from(body.expressions.len()).expect("reference id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 240 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 240 },
            },
        },
    });
    let projection = u32::try_from(body.expressions.len()).expect("projection id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 246 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: reference,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: 240, end: 241 },
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 241, end: 246 },
            },
        },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 232, end: 247 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start: 232, end: 238 },
            value: projection,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 246, end: 247 },
        },
    });
    body.blocks[0].statements.push(statement);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nonfinal return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nonfinal projected return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "return o.inner;", 1)))
    );
}

#[test]
fn direct_static_subobject_return_resource_preflight_is_exact_and_checked() {
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!projected_subobject_return_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        plans - 1,
        actions - 2,
        2,
        0,
        2,
    ));
    assert!(!projected_subobject_return_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        plans - 1,
        actions - 2,
        2,
        1,
        1,
    ));
    for violation in [
        projected_subobject_return_budget_violation(
            values,
            places - 3,
            transitions - 1,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 2,
            transitions - 1,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions - 1,
            0,
            plans,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions - 1,
            0,
            plans - 1,
            actions - 1,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(0, 0, 0, 0, 0, 0, 0, usize::MAX, usize::MAX),
    ] {
        assert!(violation);
    }
}

#[test]
fn static_subobject_move_resource_preflight_is_exact_and_checked() {
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!projected_subobject_move_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        2,
    ));
    assert!(projected_subobject_move_budget_violation(values, places - 3, transitions - 1, 0, 2,));
    assert!(projected_subobject_move_budget_violation(
        values - 1,
        places - 2,
        transitions - 1,
        0,
        2,
    ));
    assert!(projected_subobject_move_budget_violation(values - 1, places - 3, transitions, 0, 2,));
    assert!(projected_subobject_move_budget_violation(0, 0, 0, 0, usize::MAX,));
}

#[test]
fn nested_partial_struct_return_preserves_recursive_topology_and_reverse_survivors() {
    let (source, raw) = nested_owned_partial_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested partial return");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let transferred_root = roots[&2];
    let survivor_root = roots[&3];
    let block = function.blocks().next().expect("block");
    let returned = block.terminator().value_operands().next().expect("returned value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
        })
        .expect("nested partial return temporary")
        .id();
    let topology = |root| {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        let inner_fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == fields[&0] => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(inner_fields.keys().copied().collect::<Vec<_>>(), [0]);
        (inner_fields[&0], fields[&1])
    };
    let source_topology = topology(source_root);
    let transferred_topology = topology(transferred_root);
    let returned_topology = topology(temporary);
    assert_ne!(source_topology, transferred_topology);
    assert_ne!(transferred_topology, returned_topology);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        cleanup
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [survivor_root, moved_leaf_owner,]
    );
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != transferred_root
            && action.root() != temporary
            && action.root() != returned_topology.0
    }));
}

#[test]
fn partial_transfer_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_transfer_place_delta(0, 0), Some(2));
    assert_eq!(partial_transfer_place_delta(2, 0), Some(8));
    assert_eq!(partial_transfer_place_delta(2, 1), Some(7));
    assert_eq!(partial_transfer_place_delta(2, 2), Some(6));
    assert_eq!(partial_transfer_place_delta(1, 2), None);
    assert_eq!(partial_transfer_place_delta(usize::MAX, 0), None);

    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 2, 0, 2, 1),
        Ok(7),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values, places - 7, transitions - 2, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 6, transitions - 2, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 2, 1, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_transfer_budget_preflight(0, 0, 0, 0, usize::MAX, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
    assert_eq!(
        partial_transfer_budget_preflight(usize::MAX, 0, 0, 0, 0, 0),
        Err(PartialTransferBudgetViolation::Values),
    );
}

#[test]
fn partial_return_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_return_place_delta(0, 0), Some(1));
    assert_eq!(partial_return_place_delta(2, 0), Some(5));
    assert_eq!(partial_return_place_delta(2, 1), Some(4));
    assert_eq!(partial_return_place_delta(2, 2), Some(3));
    assert_eq!(partial_return_place_delta(1, 2), None);
    assert_eq!(partial_return_place_delta(usize::MAX, 0), None);

    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions - 1, 0, 2, 1),
        Ok(4),
    );
    assert_eq!(
        partial_return_budget_preflight(values, places - 4, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 3, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions - 1, 1, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_return_budget_preflight(0, 0, 0, 0, usize::MAX, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
    assert_eq!(
        partial_return_budget_preflight(usize::MAX, 0, 0, 0, 0, 0),
        Err(PartialTransferBudgetViolation::Values),
    );
}

#[test]
fn partial_assignment_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_assignment_place_delta(0, 0, 0), Some(1));
    assert_eq!(partial_assignment_place_delta(2, 0, 0), Some(7));
    assert_eq!(partial_assignment_place_delta(2, 1, 0), Some(6));
    assert_eq!(partial_assignment_place_delta(2, 1, 1), Some(5));
    assert_eq!(partial_assignment_place_delta(2, 2, 2), Some(3));
    assert_eq!(partial_assignment_place_delta(1, 2, 0), None);
    assert_eq!(partial_assignment_place_delta(1, 0, 2), None);
    assert_eq!(partial_assignment_place_delta(usize::MAX, 0, 0), None);
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 2, 0, 2, 1, 1,),
        Ok(5),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values, places - 5, transitions - 2, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 4, transitions - 2, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 1, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 2, 1, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_assignment_budget_preflight(0, 0, 0, 0, usize::MAX, 0, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
}

#[test]
fn owned_fixed_array_accepts_disjoint_string_projection_moves() {
    let (source, raw) = owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Disjoint);
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("source-faithful disjoint array projections");
    let program = lower(pair_input(&syntax, &sources)).expect("disjoint array projection moves");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let projected = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(projected.iter().map(|(index, _)| *index).collect::<Vec<_>>(), vec![0, 1]);
    let moved = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::MoveFromPlace)
        .filter_map(|instruction| instruction.place_operands().next())
        .filter(|place| projected.iter().any(|(_, projected)| projected == place))
        .count();
    assert_eq!(moved, 2);
}

#[test]
fn projected_string_clone_preserves_a_disjoint_partial_root_mask() {
    let (source, raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Disjoint, 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful disjoint projected clone");
    let program = lower(pair_input(&syntax, &sources)).expect("disjoint projected String clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let projected = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let moved = projected.iter().find(|(index, _)| *index == 0).expect("moved element").1;
    let cloned = projected.iter().find(|(index, _)| *index == 1).expect("cloned element").1;
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let move_index = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(moved)
        })
        .expect("first element move");
    let clone_index = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::StringClone
                && instruction.place_operands().next() == Some(cloned)
        })
        .expect("second element clone");
    let construct_index = instructions
        .iter()
        .rposition(|instruction| instruction.kind() == VerifiedInstructionKind::FixedArrayConstruct)
        .expect("result array construction");
    assert!(
        move_index < clone_index && clone_index < construct_index,
        "move={move_index}, clone={clone_index}, construct={construct_index}, kinds={:?}",
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
    );
    let cleanup = instructions[clone_index]
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved root clone cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [cloned]);
    let exit = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved source root exit cleanup");
    assert_eq!(exit.moved_projections().collect::<Vec<_>>(), [moved]);
    assert_eq!(exit.initialized_projections().collect::<Vec<_>>(), [cloned]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn partial_fixed_array_owner_transfers_with_exact_topology_and_mask() {
    let (source, raw) = owned_array_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray transfer");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let elements = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                    Some((index, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let source_elements = elements(source_root);
    let target_elements = elements(target_root);
    assert_eq!(source_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(target_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let projected_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_elements[&0])
        })
        .expect("first element move");
    let whole_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array whole move");
    let initialize = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial array target initialization");
    let clone = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::StringClone
                && instruction.place_operands().next() == Some(target_elements[&1])
        })
        .expect("target second element clone");
    let construct = instructions
        .iter()
        .rposition(|instruction| instruction.kind() == VerifiedInstructionKind::FixedArrayConstruct)
        .expect("result array construction");
    assert!(projected_move < whole_move && whole_move < initialize);
    assert!(initialize < clone && clone < construct);
    let transfer_value = instructions[whole_move].result().expect("array transfer value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == transfer_value)
        })
        .expect("array transfer temporary")
        .id();
    assert_eq!(elements(temporary).keys().copied().collect::<Vec<_>>(), [0, 1]);
    for actions in [
        instructions[clone].derived_drop_actions().collect::<Vec<_>>(),
        block.terminator().derived_drop_actions().collect::<Vec<_>>(),
    ] {
        let cleanup = actions
            .iter()
            .find(|action| action.root() == target_root)
            .expect("transferred array cleanup");
        assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_elements[&0]]);
        assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_elements[&1]]);
        assert!(
            actions.iter().all(|action| action.root() != source_root && action.root() != temporary)
        );
    }
    let clone_instruction = instructions[clone];
    for status in [RuntimeStatus::Allocation, RuntimeStatus::Capacity, RuntimeStatus::AbiViolation]
    {
        let injection =
            OwnedFaultInjection::Runtime { operation: LogicalOperation::StringClone, status };
        let first = owned_fault_trace(abi, function, clone_instruction, injection, 0, 1)
            .expect("transferred array clone fault");
        let replay = owned_fault_trace(abi, function, clone_instruction, injection, 0, 1)
            .expect("deterministic transferred array clone fault");
        assert_eq!(first, replay);
        assert!(!first.result_committed);
        assert_eq!(first.uncommitted_result, clone_instruction.result());
        assert!(first.retained_roots.contains(&target_root));
        assert!(first.reverse_cleanup.contains(&target_root));
        assert!(!first.retained_roots.contains(&source_root));
        assert!(!first.retained_roots.contains(&temporary));
    }
}

#[test]
fn projected_string_clone_rejects_a_moved_overlapping_leaf() {
    let (source, raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Repeat, 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful repeated projected clone");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("clone of moved projection must fail");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "clone(a[0])", 0))),
    );
}

#[test]
fn projected_string_clone_rejects_copy_and_nonconstant_array_leaves() {
    let (copy_source, copy_raw) = owned_pair_copy_projection_clone_snapshot();
    let copy_sources = sources_for(&copy_source);
    let copy_syntax =
        verify_snapshot(copy_raw, &copy_sources).expect("source-faithful Copy projection clone");
    let copy = lower(pair_input(&copy_syntax, &copy_sources))
        .expect_err("Copy projection is not a String clone source");
    assert_eq!(copy.len(), 1);
    assert_eq!(copy[0].code(), "ZRYNA-M3012");
    assert_eq!(
        copy[0].primary_span(),
        Some(span(&copy_sources, nth_untrusted_span(&copy_source, "clone(p.flag)", 0),)),
    );

    for (case, needle, label) in [
        (OwnedArrayProjectionCase::Dynamic, "a[a]", "dynamic"),
        (OwnedArrayProjectionCase::Negative, "a[-1]", "negative"),
        (OwnedArrayProjectionCase::OutOfBounds, "a[2]", "out of bounds"),
    ] {
        let (source, raw) = owned_array_projected_clone_return_snapshot(case, 0);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("source-faithful invalid projected clone");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let child = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: projection.end - 1,
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, child)), "{label}");
    }
}

#[test]
fn owned_projection_repeat_is_m3014() {
    let (repeat_source, repeat_raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Repeat);
    let repeat_sources = sources_for(&repeat_source);
    let repeat_syntax = verify_snapshot(repeat_raw, &repeat_sources)
        .expect("source-faithful repeated array projection");
    let repeat =
        lower(pair_input(&repeat_syntax, &repeat_sources)).expect_err("repeated projection move");
    assert_eq!(repeat[0].code(), "ZRYNA-M3014");
    assert_eq!(
        repeat[0].primary_span(),
        Some(span(&repeat_sources, nth_untrusted_span(&repeat_source, "a[0]", 1))),
    );
}

#[test]
fn partial_struct_owner_returns_with_exact_topology_mask_and_survivor_cleanup() {
    let (source, raw) = owned_pair_partial_then_root_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial Struct return");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial root return move");
    let returned = block.terminator().value_operands().next().expect("returned value");
    assert_eq!(whole_move.result(), Some(returned));
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
        })
        .expect("partial return temporary")
        .id();
    let fields = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let source_fields = fields(source_root);
    let returned_fields = fields(temporary);
    assert_eq!(source_fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(returned_fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].root(), moved_leaf_owner);
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != temporary
            && action.root() != returned_fields[&0]
    }));
}

#[test]
fn partial_fixed_array_owner_returns_with_exact_topology_and_survivor_cleanup() {
    let (source, raw) = owned_array_partial_then_root_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial array return");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array return move");
    let returned = block.terminator().value_operands().next().expect("returned value");
    assert_eq!(whole_move.result(), Some(returned));
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
        })
        .expect("partial array return temporary")
        .id();
    let elements = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                    Some((index, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let source_elements = elements(source_root);
    let returned_elements = elements(temporary);
    assert_eq!(source_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(returned_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].root(), moved_leaf_owner);
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != temporary
            && action.root() != returned_elements[&0]
    }));
}

#[test]
fn owned_projection_invalid_field_and_index_diagnostics_use_the_projection_child() {
    let (field_source, field_raw) = owned_pair_projected_return_snapshot("nope");
    let field_sources = sources_for(&field_source);
    let field_syntax =
        verify_snapshot(field_raw, &field_sources).expect("source-faithful invalid owned field");
    let field = lower(pair_input(&field_syntax, &field_sources)).expect_err("invalid owned field");
    assert_eq!(field[0].code(), "ZRYNA-M3006");
    assert_eq!(
        field[0].primary_span(),
        Some(span(&field_sources, nth_untrusted_span(&field_source, "nope", 0))),
    );

    for (case, needle, label) in [
        (OwnedArrayProjectionCase::Dynamic, "a[a]", "dynamic"),
        (OwnedArrayProjectionCase::Negative, "a[-1]", "negative"),
        (OwnedArrayProjectionCase::OutOfBounds, "a[2]", "out of bounds"),
    ] {
        let (source, raw) = owned_array_projected_return_snapshot(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful invalid owned index");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let expected = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: projection.end - 1,
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, expected)), "{label}");
    }
}

#[test]
fn aggregate_projection_wrong_base_kinds_are_symmetric_m3006() {
    for (source, raw, needle, label) in [
        {
            let (source, raw) = struct_index_wrong_base_snapshot();
            (source, raw, "p[0]", "Struct indexed as FixedArray")
        },
        {
            let (source, raw) = fixed_array_field_wrong_base_snapshot();
            (source, raw, "a.foo", "FixedArray accessed as Struct")
        },
    ] {
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-base projection");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let child = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: if needle == "p[0]" { projection.start + 3 } else { projection.end },
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, child)), "{label}");
    }
}

#[test]
fn aggregate_assignment_rejects_direct_self_move_and_immutable_target() {
    for (rhs, mutable, reference_ordinal, label) in [
        (OwnedPairAssignmentRhs::SelfMove, true, 2, "direct self move"),
        (OwnedPairAssignmentRhs::Fresh, false, 1, "immutable target"),
    ] {
        let (source, raw) = owned_pair_assignment_snapshot(rhs, mutable);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful rejected assignment");
        let first = lower(pair_input(&syntax, &sources)).expect_err(label);
        let second = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(first.len(), 1, "{label}");
        assert_eq!(first[0].code(), "ZRYNA-M3014", "{label}");
        assert_eq!(
            first[0].primary_span(),
            Some(span(&sources, nth_untrusted_span(&source, "p", reference_ordinal))),
            "{label}",
        );
        assert_eq!(first[0].message(), second[0].message(), "{label}");
        assert_eq!(first[0].primary_span(), second[0].primary_span(), "{label}");
    }
}

#[test]
fn aggregate_assignment_may_copy_project_from_its_preserved_destination() {
    let (source, raw) =
        owned_pair_projection_assignment_snapshot(OwnedPairProjectionAssignmentRhs::CopyField);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful Copy projection aggregate assignment");
    let program = lower(pair_input(&syntax, &sources))
        .expect("Copy projection must not consume the preserved assignment destination");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let copy_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
        .expect("CopyFromPlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(copy_index < replace_index);
    let projected = instructions[copy_index].place_operands().next().expect("Copy projection");
    assert!(matches!(
        function.places().find(|place| place.id() == projected).expect("projected place").kind(),
        VerifiedPlaceKind::StructField { ordinal: 1, .. }
    ));
}

#[test]
fn aggregate_assignment_rejects_owned_projection_consumption_from_destination() {
    let (source, raw) =
        owned_pair_projection_assignment_snapshot(OwnedPairProjectionAssignmentRhs::MoveField);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful consuming projection aggregate assignment");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("destination projection consumption");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    let projection = nth_untrusted_span(&source, "p.first", 0);
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: projection.file,
                start: projection.start,
                end: projection.start + 1,
            },
        )),
    );
}

#[test]
fn fixed_array_assignment_reports_invalid_projection_before_consumption() {
    let (source, raw) = fixed_array_oob_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful out-of-bounds projection assignment");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("out-of-bounds assignment projection");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let projection = nth_untrusted_span(&source, "a[2]", 0);
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: projection.file,
                start: projection.start + 2,
                end: projection.start + 3,
            },
        )),
    );
}

#[test]
fn root_enum_assignment_replaces_with_authenticated_old_variant_drop() {
    let (source, raw) = owned_enum_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful enum assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("root enum assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    let target = replace.place_operands().next().expect("enum target");
    let actions = replace.derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].root(), target);
    assert_eq!(actions[0].active_variant(), Some(1));
    assert_eq!(
        actions[0]
            .active_variants()
            .find(|variant| variant.place() == target)
            .map(VerifiedActiveVariant::variant),
        Some(1),
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 1);
}

#[test]
fn aggregate_assignment_transition_budget_is_exact_plus_one_and_overflow_checked() {
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!aggregate_transition_budget_violation(maximum, 0, 0));
    assert!(!aggregate_transition_budget_violation(maximum - 2, 1, 1));
    assert!(aggregate_transition_budget_violation(maximum - 2, 1, 2));
    assert!(aggregate_transition_budget_violation(0, usize::MAX, 1));
    assert!(aggregate_transition_budget_violation(usize::MAX, 0, 1));
}

#[test]
#[allow(clippy::too_many_lines)]
fn structural_clone_fault_oracle_authenticates_recursive_string_leaf_failure() {
    let (source, raw) = clone_final_return_snapshot(OWNED_PAIR_SOURCE, OWNED_PAIR_RESPONSE);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful aggregate clone v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned aggregate clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("clone");
    let source_owner = clone.place_operands().next().expect("source");
    for status in [RuntimeStatus::Allocation, RuntimeStatus::Capacity, RuntimeStatus::AbiViolation]
    {
        let completed_prefix = 0;
        let injection = OwnedFaultInjection::AggregateCloneElement { status, completed_prefix };
        let event_limit = usize::try_from(completed_prefix).expect("small prefix") + 1;
        let first = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
            .expect("recursive StringClone failure");
        let replay = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
            .expect("deterministic recursive failure");
        assert_eq!(first, replay);
        assert!(!first.result_committed);
        assert_eq!(first.uncommitted_result, clone.result());
        assert!(first.retained_roots.contains(&source_owner));
        assert!(first.reverse_cleanup.contains(&source_owner));
        assert_eq!(first.reverse_prefix, (0..completed_prefix).rev().collect::<Vec<_>>());
    }
    assert_eq!(
        owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 1,
            },
            0,
            2,
        ),
        Err(OwnedFaultOracleError::InvalidAggregateClonePrefix),
    );
    assert_eq!(
        owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 0,
            },
            0,
            0,
        ),
        Err(OwnedFaultOracleError::EventLimit),
    );

    let (array_source, array_raw) =
        clone_final_return_snapshot(OWNED_ARRAY_SOURCE, OWNED_ARRAY_RESPONSE);
    let array_sources = sources_for(&array_source);
    let array_syntax =
        verify_snapshot(array_raw, &array_sources).expect("source-faithful array clone");
    let array_program =
        lower(pair_input(&array_syntax, &array_sources)).expect("owned array clone");
    let array_function =
        array_program.modules().next().expect("module").functions().next().expect("function");
    let array_clone = array_function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("array clone");
    let last_valid = OwnedFaultInjection::AggregateCloneElement {
        status: RuntimeStatus::Allocation,
        completed_prefix: 1,
    };
    assert_eq!(
        owned_fault_trace(
            array_program.runtime_abi(),
            array_function,
            array_clone,
            last_valid,
            0,
            1,
        ),
        Err(OwnedFaultOracleError::EventLimit),
        "event bound is checked before materializing the recursive prefix trace",
    );
    let trace = owned_fault_trace(
        array_program.runtime_abi(),
        array_function,
        array_clone,
        last_valid,
        0,
        2,
    )
    .expect("last valid fixed-array String leaf prefix");
    assert_eq!(trace.reverse_prefix, vec![0]);
    assert_eq!(
        owned_fault_trace(
            array_program.runtime_abi(),
            array_function,
            array_clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 2,
            },
            0,
            3,
        ),
        Err(OwnedFaultOracleError::InvalidAggregateClonePrefix),
    );
}

#[test]
fn structural_clone_resource_preflight_accepts_exact_limits_and_rejects_excess_or_overflow() {
    assert!(!aggregate_clone_budget_violation(
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 3,
        1,
    ));
    assert!(aggregate_clone_budget_violation(
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
        0,
        0,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
        0,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
        1,
    ));
    assert!(aggregate_clone_budget_violation(0, 0, 0, 0, usize::MAX, 0));
    assert!(aggregate_clone_budget_violation(0, 0, 0, 0, 0, usize::MAX));
}

#[test]
fn projected_aggregate_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 4,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        3,
    ));
    for (values, places, transitions, reserved, missing) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 3, 0, 0, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0),
        (0, usize::MAX, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX),
    ] {
        assert!(projected_aggregate_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            missing,
        ));
    }
}

#[test]
fn projected_subobject_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_subobject_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 10,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        2,
        3,
        4,
    ));
    for (values, places, transitions, reserved, source_path, descendants, target_path) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 9, 0, 0, 2, 3, 4),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX, 0, 0),
        (0, 0, 0, 0, 0, usize::MAX, 0),
        (0, 0, 0, 0, 0, 0, usize::MAX),
    ] {
        assert!(projected_subobject_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            source_path,
            descendants,
            target_path,
        ));
    }
}

#[test]
fn projected_aggregate_clone_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_clone_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 6,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        MAX_DROP_ACTIONS_PER_FUNCTION - 5,
        2,
        2,
        3,
    ));
    for (values, places, transitions, reserved, plans, actions, pending, source, target) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 5, 0, 0, 0, 0, 0, 2, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION - 1, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 4, 2, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, 0, 0, usize::MAX, 0),
        (0, 0, 0, 0, 0, 0, 0, 0, usize::MAX),
        (0, 0, 0, 0, 0, 0, usize::MAX, 0, 0),
    ] {
        assert!(projected_aggregate_clone_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
            source,
            target,
        ));
    }
}

#[test]
fn projected_aggregate_clone_resource_preflight_is_exact_plus_one_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_clone_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 5,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        MAX_DROP_ACTIONS_PER_FUNCTION - 5,
        2,
        3,
    ));
    for (values, places, transitions, reserved, plans, actions, pending, missing) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 4, 0, 0, 0, 0, 0, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION - 1, 0, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 4, 2, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, 0, 0, usize::MAX),
        (0, 0, 0, 0, 0, 0, usize::MAX, 0),
    ] {
        assert!(projected_aggregate_clone_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
            missing,
        ));
    }
}

#[test]
fn projected_string_clone_resource_preflight_is_exact_plus_one_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_string_clone_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 1,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
        MAX_DROP_ACTIONS_PER_FUNCTION - 2,
        2,
    ));
    for (values, places, transitions, reserved, plans, actions, pending) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 1, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 1, 2),
        (usize::MAX, 0, 0, 0, 0, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0),
        (0, 0, usize::MAX, 0, 0, 0, 0),
        (0, 0, 0, usize::MAX, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX, 0, 0),
        (0, 0, 0, 0, 0, usize::MAX, 1),
        (0, 0, 0, 0, 0, 1, usize::MAX),
    ] {
        assert!(projected_string_clone_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
        ));
    }
}

#[test]
fn private_owned_fixed_array_prepares_indices_and_moves_whole_result() {
    let sources = sources_for(OWNED_ARRAY_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ARRAY_RESPONSE), &sources)
        .expect("source-faithful owned array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned array must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::FixedArrayConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    assert_eq!(instructions[0].derived_drop_actions().count(), 0);
    assert_eq!(instructions[1].derived_drop_actions().count(), 1);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(
        instructions[2]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![0, 1],
    );
}

#[test]
fn nested_owned_structs_consume_inner_owner_once_and_preserve_failure_cleanup() {
    let sources = sources_for(NESTED_OWNED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(NESTED_OWNED_RESPONSE), &sources)
        .expect("source-faithful nested owned aggregate v4");
    let program = lower(pair_input(&syntax, &sources)).expect("nested owned aggregate must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::StructConstruct,
        ]
    );
    assert_eq!(instructions[1].derived_drop_actions().count(), 1);
    assert_eq!(
        instructions[3]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![2, 0],
        "outer operands are reordered after source-order tail/inner evaluation",
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn reversed_owned_fields_have_reverse_prepare_cleanup_and_canonical_commit_operands() {
    let sources = sources_for(OWNED_TRIO_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_TRIO_RESPONSE), &sources)
        .expect("source-faithful reversed owned fields v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Trio must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1, 0],
        "third fallible leaf drops the prepared prefix in reverse completion order",
    );
    assert_eq!(
        instructions[3]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![2, 1, 0],
        "commit reorders c/b/a source evaluation into a/b/c declaration order",
    );
    assert_eq!(instructions[3].cleanup(), None);
}

#[test]
fn owned_struct_with_fixed_array_child_commits_each_nested_owner_once() {
    let sources = sources_for(OWNED_CROSS_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_CROSS_RESPONSE), &sources)
        .expect("source-faithful Struct/FixedArray v4");
    let program = lower(pair_input(&syntax, &sources)).expect("cross aggregate must verify");
    let instructions = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        instructions,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::FixedArrayConstruct,
            VerifiedInstructionKind::StructConstruct,
        ]
    );
}

#[test]
fn private_owned_enum_payloadless_and_copy_payloads_commit_infallibly() {
    for (source, response, expected) in [
        (
            OWNED_ENUM_NONE_SOURCE,
            OWNED_ENUM_NONE_RESPONSE,
            vec![VerifiedInstructionKind::EnumConstruct],
        ),
        (
            OWNED_ENUM_COPY_SOURCE,
            OWNED_ENUM_COPY_RESPONSE,
            vec![VerifiedInstructionKind::I32Literal, VerifiedInstructionKind::EnumConstruct],
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful owned enum v4");
        let program = lower(pair_input(&syntax, &sources)).expect("owned enum must verify");
        let block = program
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .next()
            .expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
            expected,
        );
        let construct = instructions.last().expect("enum construction");
        assert_eq!(construct.cleanup(), None);
        assert_eq!(construct.variant(), Some(u32::from(instructions.len() == 2)));
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    }
}

#[test]
fn private_owned_enum_string_move_and_survivor_cleanup_are_exact() {
    let sources = sources_for(OWNED_ENUM_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_STRING_RESPONSE), &sources)
        .expect("source-faithful String enum v4");
    let program = lower(pair_input(&syntax, &sources)).expect("String enum must verify");
    let block = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::EnumConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ],
    );
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1],
        "payload preparation failure retains the preceding survivor",
    );
    assert_eq!(instructions[3].cleanup(), None);
    assert_eq!(instructions[3].variant(), Some(1));
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1],
        "return transfer excludes only the returned enum and drops survivors in reverse order",
    );
}

#[test]
fn private_owned_enum_accepts_supported_nested_aggregate_payload() {
    let sources = sources_for(OWNED_ENUM_NESTED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_NESTED_RESPONSE), &sources)
        .expect("source-faithful nested enum payload v4");
    let program = lower(pair_input(&syntax, &sources)).expect("nested enum payload must verify");
    let instructions = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::EnumConstruct,
        ],
    );
    assert_eq!(instructions[1].cleanup(), None);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(instructions[2].variant(), Some(1));
}

#[test]
fn private_owned_enum_use_after_move_and_exclusions_fail_closed() {
    let sources = sources_for(OWNED_ENUM_MOVED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_MOVED_RESPONSE), &sources)
        .expect("source-faithful moved enum v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("second enum move");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((155, 156)),
    );

    let sources = sources_for(OWNED_ENUM_VEC_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_VEC_RESPONSE), &sources)
        .expect("source-faithful excluded Vec payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec enum payload excluded");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
}

#[test]
fn private_owned_enum_wrong_payload_shape_uses_enum_diagnostic() {
    let source = OWNED_ENUM_NONE_SOURCE.replace("Maybe.none()", "Maybe.some()");
    let mut raw = response_snapshot(OWNED_ENUM_NONE_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut raw.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("enum construction")
    };
    variant.text = "some".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful missing payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing enum payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");

    let source = OWNED_ENUM_COPY_SOURCE.replace("Maybe.some(7)", "Maybe.none(7)");
    let mut raw = response_snapshot(OWNED_ENUM_COPY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum construction")
    };
    variant.text = "none".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful extra payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("extra enum payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
}

#[test]
fn private_owned_aggregate_requires_exactly_one_final_return() {
    for (source, response, expected_span) in [
        (OWNED_ENUM_DUP_RETURN_SOURCE, OWNED_ENUM_DUP_RETURN_RESPONSE, (115, 135)),
        (OWNED_ENUM_LOCAL_AFTER_RETURN_SOURCE, OWNED_ENUM_LOCAL_AFTER_RETURN_RESPONSE, (115, 145)),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful invalid return structure v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("return structure");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
        assert_eq!(
            diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
            Some(expected_span),
        );
    }
}

#[test]
fn owned_aggregate_unavailable_and_excluded_shape_diagnostics_are_stable() {
    let mut unavailable_source = OWNED_PAIR_SOURCE.to_owned();
    unavailable_source.replace_range(167..168, "P");
    let mut unavailable = response_snapshot(OWNED_PAIR_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut unavailable.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("return reference");
    };
    name.text = "P".to_owned();
    let sources = sources_for(&unavailable_source);
    let syntax = verify_snapshot(unavailable, &sources).expect("wrong-case source-faithful v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unavailable aggregate");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((167, 168)),
    );

    let mut duplicate_source = OWNED_TRIO_SOURCE.to_owned();
    duplicate_source.replace_range(118..119, "z");
    let mut duplicate = response_snapshot(OWNED_TRIO_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut duplicate.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("Trio constructor");
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { name, .. } = &mut fields[1].kind
    else {
        panic!("explicit field");
    };
    name.text = "z".to_owned();
    let sources = sources_for(&duplicate_source);
    let syntax = verify_snapshot(duplicate, &sources).expect("unknown field source-faithful v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("excluded unknown field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((118, 124)),
    );
}

#[test]
fn reversed_struct_fields_evaluate_and_construct_in_declaration_order() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..148, "right, left");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("first reference")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    expressions[0].span.end = 142;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("second reference")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    expressions[1].span.start = 144;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("constructor")
    };
    fields[0].span.end = 142;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("first field")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    fields[1].span.start = 144;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("second field")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed fields");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed fields must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let copies = instructions
        .iter()
        .copied()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
        .take(2)
        .collect::<Vec<_>>();
    let first_place = copies[0].place_operands().next().expect("first source operand");
    let second_place = copies[1].place_operands().next().expect("second source operand");
    assert!(matches!(
        places[usize::try_from(first_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(0)
    ));
    assert!(matches!(
        places[usize::try_from(second_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(1)
    ));
    let construct = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StructConstruct)
        .expect("construct");
    let operands = construct
        .value_operands()
        .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(
        operands,
        vec![
            copies[0].result().expect("left result").index(),
            copies[1].result().expect("right result").index()
        ]
    );
}

#[test]
fn private_string_if_restores_outer_owner_and_drops_branch_locals_in_reverse() {
    let (source, raw) = private_string_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned String if");
    let program = lower(pair_input(&syntax, &sources)).expect("owned String if must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(function.parameters().next().expect("bool parameter").id().index(), 0);
    assert_eq!(blocks.iter().map(|block| block.id().index()).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let then_kinds = blocks[1]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        then_kinds,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let then_drops = blocks[1]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(then_drops, vec![6, 4]);
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringClone,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks
            .iter()
            .flat_map(|block| block.instructions())
            .filter_map(|instruction| {
                instruction.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            })
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    let plans = function
        .cleanup_plans()
        .map(|plan| {
            (
                plan.id().index(),
                plan.site().role(),
                plan.actions()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plans,
        vec![
            (0, VerifiedCleanupRole::PrepareFailure, vec![]),
            (1, VerifiedCleanupRole::PrepareFailure, vec![2]),
            (2, VerifiedCleanupRole::PrepareFailure, vec![4, 2]),
            (3, VerifiedCleanupRole::PrepareFailure, vec![2]),
            (4, VerifiedCleanupRole::Return, vec![]),
        ]
    );
    let cleanup_spans = function
        .cleanup_plans()
        .map(zryna_ir::data_ownership_v1::VerifiedCleanupPlan::span)
        .collect::<Vec<_>>();
    assert_eq!(cleanup_spans[1], span(&sources, nth_untrusted_span(&source, "\"a\"", 0)));
    assert_eq!(cleanup_spans[2], span(&sources, nth_untrusted_span(&source, "\"b\"", 0)));
    assert_eq!(cleanup_spans[3], span(&sources, untrusted_range(&source, ("clone", 0), (")", 2))));
}

#[test]
fn private_string_if_rejects_one_arm_moving_an_outer_owner() {
    let (source, raw) = private_string_if_moves_outer_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful outer move in branch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("outer move must not join");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3015"));
}

#[test]
fn private_string_if_rejects_non_bool_reference_condition() {
    let (source, raw) = private_string_if_non_bool_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful i32 branch condition");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("i32 condition must reject");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"),
        "{diagnostics:?}"
    );
}

#[test]
fn private_string_if_without_else_synthesizes_empty_false_path() {
    let (source, raw) = private_string_if_without_else_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful if without else");
    let program = lower(pair_input(&syntax, &sources)).expect("omitted else must be empty path");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[2].instructions().count(), 0);
    assert_eq!(blocks[2].terminator().kind(), VerifiedTerminatorKind::Jump);
}

#[test]
fn private_string_if_rejects_nested_owned_control_flow() {
    let (source, raw) = private_string_if_nested_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested owned if");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nested owned if rejects");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));
}

#[test]
fn private_vec_if_restores_outer_owner_and_drops_branch_vec() {
    let (source, raw) = private_vec_if_fixture(false, "i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned Vec if");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Vec if must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    assert_eq!(
        blocks[1]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecPush,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let then_drop = blocks[1]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .expect("branch Vec drop");
    assert_eq!(
        then_drop
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![4]
    );
    assert!(function.cleanup_plans().all(|plan| {
        plan.actions().all(|place| place.index() != 4)
            || plan.site().role() == VerifiedCleanupRole::PrepareFailure
    }));
}

#[test]
fn private_vec_if_rejects_push_into_incoming_vec_before_rhs() {
    let (source, raw) = private_vec_if_fixture(true, "i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful outer Vec push");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("outer Vec push must reject");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3015")
        .expect("join-safety diagnostic");
    assert_eq!(
        diagnostic.primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "own", 1)))
    );
}

#[test]
fn private_vec_string_if_constructs_pushes_and_drops_branch_owner_once() {
    let (source, raw) = private_vec_if_fixture(false, "String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> if");
    let program = lower(pair_input(&syntax, &sources)).expect("Vec<String> branch must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        blocks[1]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecPush,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks[1]
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
            .count(),
        1
    );
    assert_eq!(
        blocks[2]
            .instructions()
            .filter(|instruction| { instruction.kind() == VerifiedInstructionKind::DropPlace })
            .count(),
        1
    );
}

#[test]
fn terminal_string_if_joins_owned_results_through_one_block_parameter() {
    let (source, raw) = terminal_string_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful terminal String if");
    let program = lower(pair_input(&syntax, &sources)).expect("terminal String phi must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let branch = blocks[0].terminator();
    assert_eq!(branch.value_operands().next().expect("condition").index(), 1);
    let (when_true, when_false) = branch.branch_edges().expect("entry branch");
    assert_eq!((when_true.target().index(), when_false.target().index()), (1, 2));
    assert_eq!((when_true.arguments().count(), when_false.arguments().count()), (0, 0));
    let then_value =
        blocks[1].instructions().next().expect("then String").result().expect("result");
    let else_value =
        blocks[2].instructions().next().expect("else String").result().expect("result");
    assert_eq!((then_value.index(), else_value.index()), (2, 3));
    let then_jump = blocks[1].terminator().edges().next().expect("then jump");
    let else_jump = blocks[2].terminator().edges().next().expect("else jump");
    assert_eq!((then_jump.target().index(), else_jump.target().index()), (3, 3));
    assert_eq!(then_jump.arguments().next(), Some(then_value));
    assert_eq!(else_jump.arguments().next(), Some(else_value));
    let joined = blocks[3].parameters().next().expect("String join parameter").id();
    assert_eq!(joined.index(), 4);
    assert_eq!(blocks[3].instructions().count(), 0);
    assert_eq!(blocks[3].terminator().value_operands().next(), Some(joined));
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
    assert!(function.places().any(
        |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == joined)
    ));
    assert_eq!(
        function
            .places()
            .filter(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(_)))
            .count(),
        3
    );
    assert_eq!(function.cleanup_plans().last().expect("join return cleanup").actions().count(), 0);
}

#[test]
fn terminal_vec_if_joins_exact_vec_results_through_one_block_parameter() {
    let (source, raw) = terminal_vec_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful terminal Vec if");
    let program = lower(pair_input(&syntax, &sources)).expect("terminal Vec phi must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(function.parameters().next().expect("bool parameter").id().index(), 0);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let branch = blocks[0].terminator();
    assert_eq!(branch.value_operands().next().expect("condition").index(), 1);
    let (when_true, when_false) = branch.branch_edges().expect("entry branch");
    assert_eq!((when_true.target().index(), when_false.target().index()), (1, 2));
    assert_eq!((when_true.arguments().count(), when_false.arguments().count()), (0, 0));
    let then_results = blocks[1]
        .instructions()
        .map(|instruction| instruction.result().expect("then result").index())
        .collect::<Vec<_>>();
    let else_results = blocks[2]
        .instructions()
        .map(|instruction| instruction.result().expect("else result").index())
        .collect::<Vec<_>>();
    assert_eq!(then_results, vec![2, 3]);
    assert_eq!(else_results, vec![4, 5]);
    let then_jump = blocks[1].terminator().edges().next().expect("then jump");
    let else_jump = blocks[2].terminator().edges().next().expect("else jump");
    assert_eq!((then_jump.target().index(), else_jump.target().index()), (3, 3));
    assert_eq!(
        then_jump.arguments().next().map(zryna_ir::data_ownership_v1::ValueIdentity::index),
        Some(3)
    );
    assert_eq!(
        else_jump.arguments().next().map(zryna_ir::data_ownership_v1::ValueIdentity::index),
        Some(5)
    );
    let joined = blocks[3].parameters().next().expect("owned Vec join parameter").id();
    assert_eq!(joined.index(), 6);
    assert!(function.places().any(|place| {
        matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == joined)
    }));
    assert_eq!(blocks[3].terminator().value_operands().next(), Some(joined));
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
    assert_eq!(function.cleanup_plans().last().expect("join return cleanup").actions().count(), 0);
}

#[test]
fn terminal_owned_if_rejects_missing_else_and_arm_fallthrough() {
    let (source, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    let RawStatementKind::If { else_clause, .. } = &mut function.body.statements[0].kind else {
        unreachable!("fixture root if")
    };
    *else_clause = None;
    let sources = sources_for(&source);
    let mut errors = Errors::new(&sources);
    assert!(terminal_owned_if(function, &sources, &mut errors).is_none());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    let missing_else_span = diagnostics[0].primary_span().expect("missing else span");
    let expected = untrusted_range(&source, ("if", 0), ("}", 1));
    assert_eq!(
        (missing_else_span.start(), missing_else_span.end()),
        (expected.start, expected.end)
    );

    let (source, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    let RawStatementKind::Return { value, semicolon_span, .. } = function.body.statements[1].kind
    else {
        unreachable!("fixture then return")
    };
    function.body.statements[1].kind =
        RawStatementKind::ExpressionStatement { expression: value, semicolon_span };
    let sources = sources_for(&source);
    let mut errors = Errors::new(&sources);
    assert!(terminal_owned_if(function, &sources, &mut errors).is_none());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    let fallthrough_span = diagnostics[0].primary_span().expect("fallthrough span");
    let expected = untrusted_range(&source, ("return", 0), (";", 0));
    assert_eq!((fallthrough_span.start(), fallthrough_span.end()), (expected.start, expected.end));
}

#[test]
fn terminal_owned_phi_routing_is_narrowly_private_string_or_vec() {
    let (_, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    assert!(is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, false));
    assert!(is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::Vec, true));
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, true));
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::Bool, false));
    assert!(!is_terminal_owned_phi_candidate(
        function,
        zryna_layout::TypeCategory::FixedArray,
        false,
    ));
    function.export_span = Some(function.function_span);
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, false));
}

#[test]
fn private_string_loop_restores_incoming_owner_and_reverse_drops_body_locals() {
    let (source, raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private String loop");
    let program = lower(pair_input(&syntax, &sources)).expect("private String loop must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let entry = blocks[0].terminator().edges().next().expect("preheader edge");
    assert_eq!(entry.target().index(), 1);
    assert_eq!(entry.arguments().count(), 0);
    let header = blocks[1].terminator();
    assert_eq!(header.value_operands().next().expect("loop condition").index(), 2);
    let (body, exit) = header.branch_edges().expect("header branch");
    assert_eq!((body.target().index(), exit.target().index()), (2, 3));
    assert_eq!((body.arguments().count(), exit.arguments().count()), (0, 0));
    let backedge = blocks[2].terminator().edges().next().expect("loop backedge");
    assert_eq!(backedge.target().index(), 1);
    assert_eq!(backedge.arguments().count(), 0);
    let body_kinds = blocks[2]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        body_kinds,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let dropped = blocks[2]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(dropped, vec![6, 4]);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("returned owner").index(), 5);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_loop_restores_incoming_owner_and_reverse_drops_body_locals() {
    let (source, raw) = private_vec_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private Vec loop");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec loop must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let entry = blocks[0].terminator().edges().next().expect("preheader edge");
    assert_eq!(entry.target().index(), 1);
    assert_eq!(entry.arguments().count(), 0);
    let header = blocks[1].terminator();
    assert_eq!(header.value_operands().next().expect("loop condition").index(), 2);
    let (body, exit) = header.branch_edges().expect("header branch");
    assert_eq!((body.target().index(), exit.target().index()), (2, 3));
    assert_eq!((body.arguments().count(), exit.arguments().count()), (0, 0));
    let backedge = blocks[2].terminator().edges().next().expect("loop backedge");
    assert_eq!(backedge.target().index(), 1);
    assert_eq!(backedge.arguments().count(), 0);
    let body_kinds = blocks[2]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        body_kinds,
        vec![
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let dropped = blocks[2]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(dropped, vec![6, 4]);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("returned owner").index(), 7);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_string_loop_replaces_one_stable_outer_place_with_failure_cleanup() {
    let (source, raw) = private_string_mutation_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String mutation loop");
    let program = lower(pair_input(&syntax, &sources)).expect("String mutation loop must verify");
    let replay = lower(pair_input(&syntax, &sources)).expect("String mutation replay must verify");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(blocks[1].terminator().value_operands().next().expect("condition").index(), 2);
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::StringFromUtf8, VerifiedInstructionKind::ReplacePlace]
    );
    let prepare = blocks[2].instructions().next().expect("replacement prepare");
    let cleanup = prepare.cleanup().expect("prepare failure cleanup");
    let actions = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .expect("prepare cleanup plan")
        .actions()
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(actions, vec![2]);
    let replacement = blocks[2].instructions().nth(1).expect("replacement commit");
    assert_eq!(
        replacement
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(blocks[2].terminator().edges().next().expect("backedge").target().index(), 1);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("return").index(), 4);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_loop_pushes_into_one_stable_outer_place_with_failure_cleanup() {
    let (source, raw) = private_vec_push_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec push loop");
    let program = lower(pair_input(&syntax, &sources)).expect("Vec push loop must verify");
    let replay = lower(pair_input(&syntax, &sources)).expect("Vec push replay must verify");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(blocks[1].terminator().value_operands().next().expect("condition").index(), 2);
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::I32Literal, VerifiedInstructionKind::VecPush]
    );
    let push = blocks[2].instructions().nth(1).expect("VecPush commit");
    assert_eq!(
        push.place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let cleanup = push.cleanup().expect("VecPush failure cleanup");
    let actions = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .expect("VecPush cleanup plan")
        .actions()
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(actions, vec![2]);
    assert_eq!(blocks[2].terminator().edges().next().expect("backedge").target().index(), 1);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("return").index(), 4);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_string_mutation_loop_rejects_immutable_and_self_move_before_rhs() {
    for (mutable, replacement) in
        [(false, StringLoopReplacement::Literal), (true, StringLoopReplacement::Move)]
    {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(mutable, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful mutation negative");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mutation must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        let target = nth_untrusted_span(
            &source,
            "outer",
            if matches!(replacement, StringLoopReplacement::Move) { 2 } else { 1 },
        );
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, target)));
    }

    let (source, raw) =
        private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::Call);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming-call negative");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("incoming call move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
    let argument = nth_untrusted_span(&source, "outer", 2);
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, argument)));
}

#[test]
fn private_string_mutation_loop_finds_nested_consumers_but_allows_direct_reads() {
    for replacement in [StringLoopReplacement::CloneCall, StringLoopReplacement::ConcatCall] {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(true, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested consumer");
        let diagnostics =
            lower(pair_input(&syntax, &sources)).expect_err("nested move must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        let inner = nth_untrusted_span(&source, "outer", 2);
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, inner)));
    }

    for replacement in [StringLoopReplacement::CloneRead, StringLoopReplacement::ConcatRead] {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(true, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful direct read");
        let program = lower(pair_input(&syntax, &sources))
            .expect("direct clone/concat read must remain admitted");
        if matches!(replacement, StringLoopReplacement::ConcatRead) {
            let function = program
                .verified_ir()
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("function");
            let body = function.blocks().nth(2).expect("loop body");
            let last = body.instructions().last().expect("temporary read drop");
            assert_eq!(last.kind(), VerifiedInstructionKind::DropPlace);
            assert_eq!(last.place_operands().next().expect("dropped literal").index(), 3);
        }
    }
}

#[test]
fn private_string_mutation_loop_accepts_nested_private_concat_call() {
    let (source, raw) = private_nested_string_mutation_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested loop call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested loop call");
    let body = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .nth(2)
        .expect("loop body");
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::StringConcat })
    );
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::DirectCall })
    );
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::ReplacePlace })
    );
}

#[test]
fn private_vec_mutation_loop_rejects_immutable_target_at_exact_reference() {
    let (source, raw) = private_vec_push_loop_fixture_with_mutability(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable Vec loop");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable Vec must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    let target = nth_untrusted_span(&source, "outer", 1);
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, target)));
}

#[test]
fn private_string_loop_rejects_incoming_owner_move_at_reference_before_lowering() {
    let (source, raw) = private_string_loop_fixture_with_incoming_move(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful incoming loop move");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("incoming move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
    let primary = diagnostics[0].primary_span().expect("incoming reference span");
    let expected = nth_untrusted_span(&source, "outer", 1);
    assert_eq!((primary.start(), primary.end()), (expected.start, expected.end));
}

#[test]
fn private_string_loop_rejects_non_bool_condition_at_exact_reference() {
    let (source, raw) = private_string_loop_fixture_with_options(false, true, false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful non-bool loop condition");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("non-bool loop must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3012");
    let primary = diagnostics[0].primary_span().expect("condition reference span");
    let expected = nth_untrusted_span(&source, "outer", 1);
    assert_eq!((primary.start(), primary.end()), (expected.start, expected.end));
}

#[test]
fn private_string_false_loop_retains_reachable_exit_and_replays_deterministically() {
    let (source, raw) = private_string_loop_fixture_with_options(false, false, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful false loop");
    let first = lower(pair_input(&syntax, &sources)).expect("false loop must retain its exit");
    let second = lower(pair_input(&syntax, &sources)).expect("false loop replay must verify");
    assert_eq!(format!("{:?}", first.verified_ir()), format!("{:?}", second.verified_ir()));
    let function =
        first.verified_ir().modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[1].instructions().next().expect("header false").bool_literal(), Some(false));
    assert_eq!(blocks[3].terminator().kind(), VerifiedTerminatorKind::Return);
}

#[test]
fn owned_loop_shape_preflight_rejects_nested_return_repetition_and_post_effect() {
    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    let body_statement_span = function.body.statements[2].span;
    let RawStatementKind::LocalDeclaration { initializer, semicolon_span, .. } =
        function.body.statements[2].kind
    else {
        unreachable!("fixture body local")
    };
    function.body.statements[2].kind = RawStatementKind::Return {
        keyword_span: body_statement_span,
        value: initializer,
        semicolon_span,
    };
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_body(function, 1, false, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, body_statement_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    let body_statement_span = function.body.statements[2].span;
    function.body.statements[2].kind = RawStatementKind::While {
        keyword_span: body_statement_span,
        open_paren_span: body_statement_span,
        condition: 1,
        close_paren_span: body_statement_span,
        body_block: 1,
    };
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_body(function, 1, false, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, body_statement_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    function.body.blocks[0].statements = vec![0, 1, 1, 4];
    let repeated_span = function.body.statements[1].span;
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_exit(function, 1, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, repeated_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    function.body.blocks[0].statements = vec![0, 1, 2, 4];
    let effect_span = function.body.statements[2].span;
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_exit(function, 1, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, effect_span)));
}

#[test]
fn vec_cleanup_reservations_are_expression_aware_at_exact_boundaries() {
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert_eq!(cleanup_actions_after_preparation(maximum, false), maximum);
    assert!(!resource_budget_violation(
        0,
        cleanup_actions_after_preparation(maximum, false),
        maximum
    ));
    assert!(resource_budget_violation(
        0,
        cleanup_actions_after_preparation(maximum, true),
        maximum
    ));
    assert_eq!(cleanup_actions_after_transfer(maximum, true), maximum - 1);
    assert!(!resource_budget_violation(1, cleanup_actions_after_transfer(maximum, true), maximum));
    assert!(resource_budget_violation(1, cleanup_actions_after_transfer(maximum, false), maximum));
    assert_eq!(cleanup_actions_after_preparation(usize::MAX, true), usize::MAX);
    assert_eq!(cleanup_actions_after_transfer(0, true), 0);
    assert_eq!(cleanup_actions_after_additions(maximum, 0), maximum);
    assert!(resource_budget_violation(0, cleanup_actions_after_additions(maximum, 1), maximum));
}

fn private_string_branch_budget_lowerer<'a, 'e>(
    input: SemanticInput<'a>,
    function: &'a RawFunctionSyntax,
    ty: super::Ty,
    catalog: &'a FunctionCatalog,
    errors: &'e mut Errors<'a>,
    at: zryna_source::Span,
    cleanup_actions: usize,
) -> PrivateStringLowerer<'a, 'a, 'e> {
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0), raw::PlaceId(1), raw::PlaceId(2)],
        ..OwnerState::default()
    };
    let cfg = OwnedCfgState::single_block(at, errors).expect("entry block");
    PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog,
        errors,
        bindings: std::collections::BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners,
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    }
}

#[test]
fn private_string_branch_drop_budget_is_atomic_at_exact_plus_one() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let incoming = OwnedStringBranchState {
        bindings: std::collections::BTreeMap::new(),
        owners: OwnerState {
            pending: vec![raw::PlaceId(0)],
            value_owners: std::collections::BTreeMap::new(),
        },
        known_bytes: std::collections::BTreeMap::new(),
    };

    let mut exact_errors = Errors::new(&sources);
    let mut exact = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut exact_errors,
        at,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
    );
    exact.cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2;
    assert!(exact.restore_branch_scope(&incoming, at).is_some());
    assert_eq!(exact.cleanup_actions, zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION);
    assert_eq!(exact.owners, incoming.owners);
    drop(exact);
    assert!(exact_errors.finish().is_empty());

    let mut extra_errors = Errors::new(&sources);
    let mut extra = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut extra_errors,
        at,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1,
    );
    let before = extra.owners.clone();
    assert!(extra.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(
        extra.cleanup_actions,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1
    );
    assert_eq!(extra.owners, before);
    assert!(extra.cfg.current_block().expect("entry").instructions.is_empty());
    drop(extra);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut transition_errors = Errors::new(&sources);
    let mut transition = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut transition_errors,
        at,
        0,
    );
    transition.cfg.transitions =
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1;
    let before = transition.owners.clone();
    assert!(transition.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(transition.cleanup_actions, 0);
    assert_eq!(transition.owners, before);
    assert!(transition.cfg.current_block().expect("entry").instructions.is_empty());
    drop(transition);
    let diagnostics = transition_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut overflow_errors = Errors::new(&sources);
    let mut overflow = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut overflow_errors,
        at,
        0,
    );
    overflow.cfg.transitions = usize::MAX;
    let before = overflow.owners.clone();
    assert!(overflow.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(overflow.cleanup_actions, 0);
    assert_eq!(overflow.owners, before);
    assert!(overflow.cfg.current_block().expect("entry").instructions.is_empty());
    drop(overflow);
    let diagnostics = overflow_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn private_vec_assignment_rejects_a_copy_typed_source() {
    let source = VEC_ASSIGN_I32_SOURCE.replacen(
        "const y: Vec<i32> = Vec<i32>([]);",
        "const y: i32      = 0           ;",
        1,
    );
    let sources = sources_for(&source);
    let mut raw = response_snapshot(VEC_ASSIGN_I32_RESPONSE);
    raw.files[0].type_syntax.truncate(6);
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 66, end: 69 },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "i32".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 66, end: 69 },
            },
        },
    });
    let RawStatementKind::LocalDeclaration { type_syntax, .. } =
        &mut raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("second local")
    };
    *type_syntax = 6;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 77, end: 78 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Copy mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Copy assignment source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}

#[test]
fn private_vec_assignment_rejects_a_projection_target() {
    let source = VEC_ASSIGN_STRING_SOURCE.replacen("x = Vec", "x[0] = Vec", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(VEC_ASSIGN_STRING_RESPONSE), 71, 3);
    let body = &mut raw.files[0].functions[0].body;
    let initial_literal = body.expressions[0].clone();
    let initial_vec = body.expressions[1].clone();
    let rhs_literal = body.expressions[3].clone();
    let mut rhs_vec = body.expressions[4].clone();
    let returned = body.expressions[5].clone();
    let base = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    let index = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 71, end: 72 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let target = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 73 },
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 2,
            open_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 70, end: 71 },
            index: 3,
            close_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 72, end: 73 },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } = &mut rhs_vec.kind
    else {
        panic!("RHS Vec")
    };
    elements[0] = 5;
    body.expressions =
        vec![initial_literal, initial_vec, base, index, target, rhs_literal, rhs_vec, returned];
    let RawStatementKind::Assignment { target, value, .. } = &mut body.statements[1].kind else {
        panic!("assignment")
    };
    *target = 4;
    *value = 6;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 7;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projection target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("projection assignment");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3013")
        .expect("projection target diagnostic");
    let at = diagnostic.primary_span().expect("projection span");
    assert_eq!((at.start(), at.end()), (69, 73));
}

#[test]
fn private_empty_vec_has_one_empty_prepare_site_and_returns_its_owner() {
    let sources = sources_for(EMPTY_VEC_SOURCE);
    let syntax = verify_snapshot(response_snapshot(EMPTY_VEC_RESPONSE), &sources)
        .expect("source-faithful empty Vec<i32> v4");
    let program = lower(pair_input(&syntax, &sources)).expect("empty private Vec<i32>");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let construct = block.instructions().next().expect("VecConstruct");
    assert_eq!(construct.kind(), VerifiedInstructionKind::VecConstruct);
    assert_eq!(construct.value_operands().count(), 0);
    assert_eq!(construct.derived_drop_actions().count(), 0);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_rejects_reusing_a_moved_string_element_at_second_use() {
    let sources = sources_for(MOVED_VEC_ELEMENT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(MOVED_VEC_ELEMENT_RESPONSE), &sources)
        .expect("source-faithful repeated Vec element v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("reused String element");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3014")
        .expect("moved Vec element diagnostic");
    let at = diagnostic.primary_span().expect("second element reference");
    assert_eq!((at.start(), at.end()), (84, 89));
}

#[test]
fn private_vec_push_retains_vector_and_argument_on_failure_then_consumes_argument() {
    let sources = sources_for(VEC_PUSH_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_PUSH_RESPONSE), &sources)
        .expect("source-faithful Vec<String> push v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> push");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let push = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecPush)
        .expect("VecPush");
    assert_eq!(
        push.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_string_construct_push_full_single_block_shape_is_stable() {
    let sources = sources_for(VEC_PUSH_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_PUSH_RESPONSE), &sources)
        .expect("source-faithful Vec<String> push v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> push");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id().index(), 0);
    let instructions = blocks[0].instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecPush,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    assert_eq!(
        instructions
            .iter()
            .filter_map(|instruction| {
                instruction.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            })
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    let plans = function
        .cleanup_plans()
        .map(|plan| {
            (
                plan.id().index(),
                plan.site().role(),
                plan.actions()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plans,
        [
            (0, VerifiedCleanupRole::PrepareFailure, vec![]),
            (1, VerifiedCleanupRole::PrepareFailure, vec![0]),
            (2, VerifiedCleanupRole::PrepareFailure, vec![2]),
            (3, VerifiedCleanupRole::PrepareFailure, vec![3, 2]),
            (4, VerifiedCleanupRole::Return, vec![]),
        ]
    );
    let terminator = blocks[0].terminator();
    let return_start =
        u32::try_from(VEC_PUSH_SOURCE.find("return").expect("return")).expect("return offset");
    assert_eq!(terminator.span().start(), return_start);
    assert_eq!(terminator.span().end(), return_start + 14);
    assert_eq!(terminator.value_operands().next().expect("return value").index(), 3);
    assert_eq!(terminator.cleanup().expect("return cleanup").index(), 4);
    assert_eq!(terminator.derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_construct_and_push_accept_nested_string_elements() {
    let (source, raw) = private_vec_nested_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec elements");
    let program = lower(pair_input(&syntax, &sources)).expect("nested Vec String elements");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let kinds = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::StringConcat).count(),
        2
    );
    assert!(kinds.contains(&VerifiedInstructionKind::VecConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::VecPush));
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_vec_construct_and_push_cleanup_fail_before_lowering_mutation_at_first_extra() {
    let (source, raw) = private_vec_nested_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec elements");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = authenticated_type_capabilities(input, 0, 1).expect("Vec<String>");
    let string_ty = authenticated_type_capabilities(input, 0, 0).expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };

    for extra in [0, 1, 2] {
        let at = span(&sources, function.body.expressions[3].span);
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let mut lowerer = super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::new(),
            places: Vec::new(),
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState::default(),
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 0,
        };
        let estimate =
            lowerer.estimate_string_sequence(&[2], string_ty, at).expect("construct estimate");
        let outer_actions = estimate.end_pending;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.value(3, vec_ty);
        if extra == 0 {
            assert!(result.is_some(), "exact construct cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow construct cleanup must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }

    for extra in [0, 1, 2] {
        let at = span(&sources, function.body.expressions[8].span);
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let place = raw::Place {
            id: raw::PlaceId(0),
            ty: vec_ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(0),
        };
        let mut lowerer = super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "values".to_owned(),
                super::Binding { ty: vec_ty, place: raw::PlaceId(0), mutable: true },
            )]),
            places: vec![place],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState {
                pending: vec![raw::PlaceId(0)],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let estimate =
            lowerer.estimate_string_sequence(&[7], string_ty, at).expect("push estimate");
        let outer_actions = estimate.end_pending;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.lower_push_effect_with_policy(8, None, false);
        if extra == 0 {
            assert!(result.is_some(), "exact push cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow push cleanup must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_vec_direct_call_cleanup_is_exact_and_atomic_before_argument_lowering() {
    let (source, raw) = private_vec_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec call");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = authenticated_type_capabilities(input, 0, 1).expect("Vec<String>");
    let string_ty = authenticated_type_capabilities(input, 0, 0).expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let signature = |declaration, name: &str, parameters| FunctionSignature {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration },
        name: name.to_owned(),
        parameters,
        result: vec_ty,
        private: true,
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![vec_ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[5];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);

    for extra in [0, 1, 2] {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let place = raw::Place {
            id: raw::PlaceId(0),
            ty: vec_ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(0),
        };
        let mut lowerer = super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "survivor".to_owned(),
                super::Binding { ty: vec_ty, place: raw::PlaceId(0), mutable: false },
            )]),
            places: vec![place],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState {
                pending: vec![raw::PlaceId(0)],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let preparation = lowerer
            .estimate_vec_preparation(4, vec_ty, 1, at)
            .expect("nested Vec argument estimate");
        let outer_actions = preparation.end_pending - 1;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - preparation.resources.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - preparation.resources.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.direct_call(callee, arguments, vec_ty, at);
        if extra == 0 {
            assert!(result.is_some(), "exact nested Vec call cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow nested Vec call must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}

#[test]
fn private_vec_index_keeps_vector_for_fault_and_scalar_return_cleanup() {
    let sources = sources_for(VEC_INDEX_SOURCE);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let syntax = verify_snapshot(response_snapshot(&response), &sources)
        .expect("source-faithful Vec<i32> index v4");
    let program = lower(pair_input(&syntax, &sources)).expect("checked Vec<i32> index");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        function.parameters().len()
            + block.parameters().len()
            + block.instructions().filter(|instruction| instruction.result().is_some()).count(),
        5,
        "Vec construction emits three values and checked indexing emits index plus result",
    );
    let index = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecIndexCopy)
        .expect("VecIndexCopy");
    assert_eq!(
        index.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn owned_fault_oracle_covers_every_admitted_string_runtime_failure() {
    let allocation_capacity_host = [
        (
            RuntimeStatus::Allocation,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
        ),
        (
            RuntimeStatus::Capacity,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
        ),
        (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
    ];

    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources)
        .expect("source-faithful String literal");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let literal = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("StringFromUtf8");
    assert_all_runtime_faults(
        abi,
        function,
        literal,
        LogicalOperation::StringFromUtf8Copy,
        &[
            (
                RuntimeStatus::Allocation,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
            ),
            (
                RuntimeStatus::Capacity,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
            ),
            (
                RuntimeStatus::Utf8,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::Utf8V1),
            ),
            (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
        ],
    );

    let sources = sources_for(STRING_CLONE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CLONE_RESPONSE), &sources)
        .expect("source-faithful String clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private String clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("StringClone");
    assert_all_runtime_faults(
        abi,
        function,
        clone,
        LogicalOperation::StringClone,
        &allocation_capacity_host,
    );
    assert_eq!(
        owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::Runtime {
                operation: LogicalOperation::StringClone,
                status: RuntimeStatus::Allocation,
            },
            0,
            1,
        )
        .expect("clone allocation trace")
        .reverse_cleanup
        .iter()
        .map(|place| place.index())
        .collect::<Vec<_>>(),
        [1]
    );

    let sources = sources_for(STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful String concat");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let concat = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringConcat)
        .expect("StringConcat");
    assert_all_runtime_faults(
        abi,
        function,
        concat,
        LogicalOperation::StringConcat,
        &allocation_capacity_host,
    );
    let trace = owned_fault_trace(
        abi,
        function,
        concat,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringConcat,
            status: RuntimeStatus::Capacity,
        },
        0,
        1,
    )
    .expect("concat capacity trace");
    assert_eq!(trace.block, 0);
    assert_eq!(trace.instruction, 4);
    assert_eq!(trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [3, 1]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn owned_fault_oracle_covers_vec_failures_bounds_and_nested_cleanup() {
    let (source, raw) = private_vec_nested_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec<String>");
    let program = lower(pair_input(&syntax, &sources)).expect("nested Vec<String>");
    let abi = program.runtime_abi();
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    for (kind, operation) in [
        (VerifiedInstructionKind::VecConstruct, LogicalOperation::VecAllocate),
        (VerifiedInstructionKind::VecPush, LogicalOperation::VecReserve),
    ] {
        let instruction = instructions
            .iter()
            .copied()
            .find(|instruction| instruction.kind() == kind)
            .expect("Vec operation");
        assert_all_runtime_faults(
            abi,
            function,
            instruction,
            operation,
            &[
                (
                    RuntimeStatus::Allocation,
                    OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
                ),
                (
                    RuntimeStatus::Capacity,
                    OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
                ),
                (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
            ],
        );
    }
    let construct = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .expect("VecConstruct");
    let push = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecPush)
        .expect("VecPush");
    let construct_trace = owned_fault_trace(
        abi,
        function,
        construct,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("nested construct failure");
    let push_trace = owned_fault_trace(
        abi,
        function,
        push,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecReserve,
            status: RuntimeStatus::Capacity,
        },
        0,
        1,
    )
    .expect("nested push failure");
    assert_eq!(
        construct_trace.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(
        construct_trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [2, 1, 0],
        "nested concat result and both read temporaries reverse-drop on construct failure"
    );
    assert_eq!(
        push_trace.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4, 7]
    );
    assert_eq!(
        push_trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [7, 6, 5, 4, 1, 0],
        "push argument temporaries, vector survivor, and earlier nested survivors reverse-drop"
    );

    let sources = sources_for(VEC_INDEX_SOURCE);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let syntax =
        verify_snapshot(response_snapshot(&response), &sources).expect("source-faithful Vec index");
    let program = lower(pair_input(&syntax, &sources)).expect("checked Vec index");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let index = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecIndexCopy)
        .expect("VecIndexCopy");
    let first = owned_fault_trace(abi, function, index, OwnedFaultInjection::Bounds, 0, 1)
        .expect("bounds trace");
    let second = owned_fault_trace(abi, function, index, OwnedFaultInjection::Bounds, 0, 1)
        .expect("deterministic bounds trace");
    assert_eq!(first, second);
    assert_eq!(
        first.disposition,
        OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::BoundsV1)
    );
    assert_eq!((first.block, first.instruction), (0, 5));
    assert_eq!(first.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
}

#[test]
fn owned_fault_oracle_is_bounded_and_fails_closed_on_mismatch() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources)
        .expect("source-faithful String literal");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let literal = function.blocks().next().expect("block").instructions().next().expect("literal");
    let valid = OwnedFaultInjection::Runtime {
        operation: LogicalOperation::StringFromUtf8Copy,
        status: RuntimeStatus::Utf8,
    };
    assert!(owned_fault_trace(abi, function, literal, valid, 0, 1).is_ok(), "exact event limit");
    assert_eq!(
        owned_fault_trace(abi, function, literal, valid, 1, 1),
        Err(OwnedFaultOracleError::EventLimit)
    );
    assert_eq!(
        owned_fault_trace(abi, function, literal, valid, usize::MAX, usize::MAX),
        Err(OwnedFaultOracleError::EventLimit)
    );
    for injection in [
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringFromUtf8Copy,
            status: RuntimeStatus::Ok,
        },
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringFromUtf8Copy,
            status: RuntimeStatus::Refcount,
        },
        OwnedFaultInjection::Bounds,
    ] {
        assert!(owned_fault_trace(abi, function, literal, injection, 0, 1).is_err());
    }
}

#[test]
fn private_vec_push_rejects_immutable_target() {
    let source = VEC_PUSH_SOURCE.replacen("let values", "const values", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 36, 2);
    raw.files[0].functions[0].body.statements[0].kind =
        match &raw.files[0].functions[0].body.statements[0].kind {
            RawStatementKind::LocalDeclaration {
                keyword_span,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
                ..
            } => RawStatementKind::LocalDeclaration {
                keyword_span: *keyword_span,
                mutable: false,
                name: name.clone(),
                type_syntax: *type_syntax,
                equals_span: *equals_span,
                initializer: *initializer,
                semicolon_span: *semicolon_span,
            },
            _ => panic!("local"),
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable push");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable push");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3014"));
}

#[test]
fn private_vec_push_rejects_wrong_element_type() {
    let source = VEC_PUSH_SOURCE.replacen("\"b\"", "1", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 95, -2);
    raw.files[0].functions[0].body.expressions[3].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong push element");
    let first = lower(pair_input(&syntax, &sources)).expect_err("wrong push element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic wrong push element");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, nth_untrusted_span(&source, "1", 0))));
}

#[test]
fn private_vec_construct_rejects_unsupported_string_element_at_exact_span_deterministically() {
    let source = VEC_PUSH_SOURCE.replacen("\"a\"", "1", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 75, -2);
    raw.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong construct element");
    let first = lower(pair_input(&syntax, &sources)).expect_err("wrong construct element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic wrong construct element");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, nth_untrusted_span(&source, "1", 0))));
}

#[test]
fn private_vec_in_range_positive_index_uses_same_checked_cleanup() {
    let source = VEC_INDEX_SOURCE.replacen("[-1]", "[1]", 1);
    let sources = sources_for(&source);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let mut raw = shift_snapshot_signed(response_snapshot(&response), 83, -1);
    raw.files[0].functions[0].body.expressions[4].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful positive Vec index");
    let program = lower(pair_input(&syntax, &sources)).expect("in-range Vec index");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(block.terminator().derived_drop_actions().count(), 1);
}

#[test]
fn vec_operand_amplification_budget_is_exact_and_plus_one_fails() {
    assert!(!aggregate_operand_budget_violation(
        zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS - 1,
        1,
    ));
    assert!(aggregate_operand_budget_violation(
        zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS,
        1,
    ));
    assert!(aggregate_operand_budget_violation(usize::MAX, 1));
}

#[test]
fn vec_derived_values_and_resource_additions_have_exact_checked_boundaries() {
    let raw = response_snapshot(VEC_STRING_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 5);
    for maximum in [
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION,
    ] {
        assert!(!resource_budget_violation(maximum - 1, 1, maximum));
        assert!(resource_budget_violation(maximum, 1, maximum));
        assert!(resource_budget_violation(usize::MAX, 1, maximum));
    }
}

#[test]
fn unresolved_or_wrong_case_vec_push_and_index_names_are_source_errors() {
    for replacement in ["absent", "Values"] {
        let source = VEC_PUSH_SOURCE.replacen("push(values", &format!("push({replacement}"), 1);
        let sources = sources_for(&source);
        let mut raw = response_snapshot(VEC_PUSH_RESPONSE);
        let function = &mut raw.files[0].functions[0];
        let vector = function
            .body
            .expressions
            .iter()
            .find_map(|expression| match expression.kind {
                zryna_syntax::v4::RawExpressionKind::VecPush { vector, .. } => Some(vector),
                _ => None,
            })
            .expect("push target");
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut function.body.expressions[usize::try_from(vector).expect("vector index")].kind
        else {
            panic!("push reference")
        };
        name.text = replacement.to_owned();
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful bad push target");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("bad push target");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
    }

    let source = VEC_INDEX_SOURCE.replacen("return values[", "return Values[", 1);
    let sources = sources_for(&source);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let mut raw = response_snapshot(&response);
    let function = &mut raw.files[0].functions[0];
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut function.body.expressions[3].kind
    else {
        panic!("index base")
    };
    name.text = "Values".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-case index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong-case index");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
}

#[test]
fn routed_push_without_any_vec_type_cannot_disappear_silently() {
    const SOURCE: &str = "function bad(): i32 { push(missing, 1); return 0; }";
    let sources = sources_for(SOURCE);
    let syntax = verify_snapshot(unresolved_push_without_vec_snapshot(), &sources)
        .expect("source-faithful unresolved push v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing Vec type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}

#[test]
fn private_vec_local_names_reject_exact_and_ascii_fold_collisions() {
    for replacement in ["values", "VALUES"] {
        let source = VEC_STRING_SOURCE.replace("first", replacement);
        let sources = sources_for(&source);
        let mut raw = shift_snapshot(response_snapshot(VEC_STRING_RESPONSE), 42, 1);
        raw = shift_snapshot(raw, 105, 1);
        let function = &mut raw.files[0].functions[0];
        let RawStatementKind::LocalDeclaration { name, .. } = &mut function.body.statements[0].kind
        else {
            panic!("first local")
        };
        name.text = replacement.to_owned();
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut function.body.expressions[1].kind
        else {
            panic!("first reference")
        };
        name.text = replacement.to_owned();
        let syntax = match verify_snapshot(raw, &sources) {
            Ok(syntax) => syntax,
            Err(diagnostics) if replacement == "values" => {
                assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-Y4002"));
                continue;
            }
            Err(diagnostics) => panic!("source-faithful colliding Vec local: {diagnostics:?}"),
        };
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("colliding Vec local");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
    }
}

#[test]
fn vec_push_target_guard_rejects_moved_and_immutable_states() {
    assert!(!vec_push_target_invalid(true, true));
    assert!(vec_push_target_invalid(false, true));
    assert!(vec_push_target_invalid(true, false));
}

#[test]
fn owned_aggregate_operand_budget_is_exact_plus_one_and_overflow_checked() {
    let maximum = zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS;
    assert!(!aggregate_operand_budget_violation(maximum, 0));
    assert!(!aggregate_operand_budget_violation(maximum - 1, 1));
    assert!(aggregate_operand_budget_violation(maximum, 1));
    assert!(aggregate_operand_budget_violation(usize::MAX, 1));

    let sources = sources_for(OWNED_PAIR_SOURCE);
    let at = sources
        .verify_span(zryna_source::UntrustedSpan { file: 0, start: 121, end: 158 })
        .expect("constructor span");
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        preflight_aggregate_operand_total(maximum - 2, 2, at, &mut exact_errors),
        Some(maximum),
    );
    assert!(exact_errors.is_empty());
    let mut extra_errors = Errors::new(&sources);
    assert_eq!(preflight_aggregate_operand_total(maximum, 1, at, &mut extra_errors), None);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((121, 158)),
    );
}

#[test]
fn private_vec_string_indexing_is_rejected_by_copy_only_rule() {
    let source = VEC_STRING_SOURCE.replacen("return values;", "return values[0];", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_STRING_RESPONSE), 126, 3);
    let function = &mut raw.files[0].functions[0];
    function.body.expressions[4].span.end = 126;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut function.body.expressions[4].kind
    else {
        panic!("values reference")
    };
    name.span.end = 126;
    function.body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 127, end: 128 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    function.body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 120, end: 129 },
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 4,
            open_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 126, end: 127 },
            index: 5,
            close_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 128, end: 129 },
        },
    });
    let RawStatementKind::Return { value, .. } = &mut function.body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("String index excluded");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}

#[test]
fn cumulative_string_byte_budget_accepts_exact_and_rejects_plus_one() {
    assert!(!string_byte_budget_violation(
        zryna_ir::data_ownership_v1::MAX_STRING_LITERAL_BYTES - 1,
        1,
    ));
    assert!(
        string_byte_budget_violation(zryna_ir::data_ownership_v1::MAX_STRING_LITERAL_BYTES, 1,)
    );
}

#[test]
fn public_string_literal_result_remains_outside_scalar_abi() {
    let source = format!("export {STRING_SOURCE}");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(STRING_RESPONSE), 0, 7);
    let function = &mut raw.files[0].functions[0];
    function.span.start = 0;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 0, end: 6 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful public String v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public owned ABI");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert!(diagnostics[0].primary_span().is_some());
}

#[test]
fn unauthenticated_string_spelling_cannot_activate_literal_lowering() {
    let sources = sources_for(STRING_SOURCE);
    let mut raw = response_snapshot(STRING_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling } =
        &mut raw.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("String literal")
    };
    *spelling = "'x'".to_owned();
    assert!(verify_snapshot(raw, &sources).is_err());
}
