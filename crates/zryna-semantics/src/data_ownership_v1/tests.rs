use super::{
    Errors, FunctionCatalog, FunctionSignature, MAX_SEMANTIC_DIAGNOSTICS, OwnedCfgBudgetLimit,
    OwnedCfgState, OwnedStringBranchState, OwnedStringEstimateContext,
    OwnedStringPreparationBudget, OwnerState, PrivateStringLowerer, ProgramCfgBudgetLimit,
    SemanticInput, ValueBudgetLimit, accumulate_generated_cfg_function,
    accumulate_generated_value_function, aggregate_clone_budget_violation,
    aggregate_operand_budget_violation, aggregate_transition_budget_violation,
    authenticated_type_capabilities, checked_string_concat_bytes, cleanup_action_budget_violation,
    cleanup_actions_after_additions, cleanup_actions_after_preparation,
    cleanup_actions_after_transfer, dense_owned_value_id, derived_value_count,
    estimate_owned_string_expression, generated_cfg_budget_violation,
    is_terminal_owned_phi_candidate, lower, owned_call_cleanup_budget_violation,
    owned_cfg_budget_violation, owned_place_budget_violation, owned_value_budget_violation,
    preflight_aggregate_operand_total, preflight_owned_loop_body, preflight_owned_loop_exit,
    preflight_owned_place_capacity, preflight_owned_place_capacity_with_reserved,
    preflight_owned_string_preparation, raw_function_value_count, raw_terminator_edge_count,
    resource_budget_violation, semantic_preflight, span, string_byte_budget_violation,
    terminal_owned_if, value_budget_violation, vec_push_target_invalid,
};
use zryna_ir::data_ownership_v1::{
    PlaceIdentity as FaultPlaceIdentity, ValueIdentity as FaultValueIdentity,
    VerifiedActiveVariant, VerifiedCleanupRole, VerifiedDropActionKind, VerifiedFunction,
    VerifiedInstruction as FaultVerifiedInstruction, VerifiedInstructionKind, VerifiedPlaceKind,
    VerifiedTerminatorKind, VerifiedTrapIdentity, raw,
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

const PAIR_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/syntax-v4-shorthand.zry");
const PAIR_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/syntax-v4-shorthand.json");
const PAIR_SCORE_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/pair-score-v4.zry");
const PAIR_SCORE_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/pair-score-v4.json");
const PAIR_ORACLE: &str = include_str!("../../../../tests/m3-fixtures/pair-oracle-v1.json");
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
const COPY_CALL_SOURCE: &str = "function caller(x: i32): i32 { return add(x, 1); } function add(x: i32, y: i32): i32 { return x + y; }";
const COPY_CALL_RESPONSE: &str = r#"{"id":300,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":19,"end":22},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":19,"end":22}}}},{"span":{"file":0,"start":25,"end":28},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":25,"end":28}}}},{"span":{"file":0,"start":67,"end":70},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":67,"end":70}}}},{"span":{"file":0,"start":75,"end":78},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":75,"end":78}}}},{"span":{"file":0,"start":81,"end":84},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":81,"end":84}}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":50},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"caller","span":{"file":0,"start":9,"end":15}},"parameters":[{"span":{"file":0,"start":16,"end":22},"name":{"text":"x","span":{"file":0,"start":16,"end":17}},"type_syntax":0}],"result_type":1,"body":{"span":{"file":0,"start":29,"end":50},"root_block":0,"blocks":[{"span":{"file":0,"start":29,"end":50},"open_brace_span":{"file":0,"start":29,"end":30},"statements":[0],"close_brace_span":{"file":0,"start":49,"end":50}}],"statements":[{"span":{"file":0,"start":31,"end":48},"kind":{"kind":"return","keyword_span":{"file":0,"start":31,"end":37},"value":2,"semicolon_span":{"file":0,"start":47,"end":48}}}],"expressions":[{"span":{"file":0,"start":42,"end":43},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":42,"end":43}}}},{"span":{"file":0,"start":45,"end":46},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":38,"end":47},"kind":{"kind":"call","callee":{"text":"add","span":{"file":0,"start":38,"end":41}},"open_paren_span":{"file":0,"start":41,"end":42},"arguments":[0,1],"close_paren_span":{"file":0,"start":46,"end":47}}}]}},{"span":{"file":0,"start":51,"end":102},"export_span":null,"function_span":{"file":0,"start":51,"end":59},"name":{"text":"add","span":{"file":0,"start":60,"end":63}},"parameters":[{"span":{"file":0,"start":64,"end":70},"name":{"text":"x","span":{"file":0,"start":64,"end":65}},"type_syntax":2},{"span":{"file":0,"start":72,"end":78},"name":{"text":"y","span":{"file":0,"start":72,"end":73}},"type_syntax":3}],"result_type":4,"body":{"span":{"file":0,"start":85,"end":102},"root_block":0,"blocks":[{"span":{"file":0,"start":85,"end":102},"open_brace_span":{"file":0,"start":85,"end":86},"statements":[0],"close_brace_span":{"file":0,"start":101,"end":102}}],"statements":[{"span":{"file":0,"start":87,"end":100},"kind":{"kind":"return","keyword_span":{"file":0,"start":87,"end":93},"value":2,"semicolon_span":{"file":0,"start":99,"end":100}}}],"expressions":[{"span":{"file":0,"start":94,"end":95},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":94,"end":95}}}},{"span":{"file":0,"start":98,"end":99},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":98,"end":99}}}},{"span":{"file":0,"start":94,"end":99},"kind":{"kind":"addition","operator_span":{"file":0,"start":96,"end":97},"lhs":0,"rhs":1}}]}}]}],"diagnostics":[]}}"#;
const COPY_AGGREGATE_CALL_SOURCE: &str = "interface P extends ZrynaStruct { x: i32; } function id(p: P): P { return p; } function use(p: P): i32 { const q: P = id(p); return p.x + q.x; }";
const COPY_AGGREGATE_CALL_RESPONSE: &str = r#"{"id":302,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":37,"end":40},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":37,"end":40}}}},{"span":{"file":0,"start":59,"end":60},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":59,"end":60}}}},{"span":{"file":0,"start":63,"end":64},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":63,"end":64}}}},{"span":{"file":0,"start":95,"end":96},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":95,"end":96}}}},{"span":{"file":0,"start":99,"end":102},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":99,"end":102}}}},{"span":{"file":0,"start":114,"end":115},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":114,"end":115}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":43},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"P","span":{"file":0,"start":10,"end":11}},"extends_span":{"file":0,"start":12,"end":19},"marker_span":{"file":0,"start":20,"end":31},"open_brace_span":{"file":0,"start":32,"end":33},"close_brace_span":{"file":0,"start":42,"end":43},"fields":[{"span":{"file":0,"start":34,"end":41},"name":{"text":"x","span":{"file":0,"start":34,"end":35}},"colon_span":{"file":0,"start":35,"end":36},"semicolon_span":{"file":0,"start":40,"end":41},"type_syntax":0}]}}],"functions":[{"span":{"file":0,"start":44,"end":78},"export_span":null,"function_span":{"file":0,"start":44,"end":52},"name":{"text":"id","span":{"file":0,"start":53,"end":55}},"parameters":[{"span":{"file":0,"start":56,"end":60},"name":{"text":"p","span":{"file":0,"start":56,"end":57}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":65,"end":78},"root_block":0,"blocks":[{"span":{"file":0,"start":65,"end":78},"open_brace_span":{"file":0,"start":65,"end":66},"statements":[0],"close_brace_span":{"file":0,"start":77,"end":78}}],"statements":[{"span":{"file":0,"start":67,"end":76},"kind":{"kind":"return","keyword_span":{"file":0,"start":67,"end":73},"value":0,"semicolon_span":{"file":0,"start":75,"end":76}}}],"expressions":[{"span":{"file":0,"start":74,"end":75},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":74,"end":75}}}}]}},{"span":{"file":0,"start":79,"end":144},"export_span":null,"function_span":{"file":0,"start":79,"end":87},"name":{"text":"use","span":{"file":0,"start":88,"end":91}},"parameters":[{"span":{"file":0,"start":92,"end":96},"name":{"text":"p","span":{"file":0,"start":92,"end":93}},"type_syntax":3}],"result_type":4,"body":{"span":{"file":0,"start":103,"end":144},"root_block":0,"blocks":[{"span":{"file":0,"start":103,"end":144},"open_brace_span":{"file":0,"start":103,"end":104},"statements":[0,1],"close_brace_span":{"file":0,"start":143,"end":144}}],"statements":[{"span":{"file":0,"start":105,"end":124},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":105,"end":110},"mutable":false,"name":{"text":"q","span":{"file":0,"start":111,"end":112}},"type_syntax":5,"equals_span":{"file":0,"start":116,"end":117},"initializer":1,"semicolon_span":{"file":0,"start":123,"end":124}}},{"span":{"file":0,"start":125,"end":142},"kind":{"kind":"return","keyword_span":{"file":0,"start":125,"end":131},"value":6,"semicolon_span":{"file":0,"start":141,"end":142}}}],"expressions":[{"span":{"file":0,"start":121,"end":122},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":121,"end":122}}}},{"span":{"file":0,"start":118,"end":123},"kind":{"kind":"call","callee":{"text":"id","span":{"file":0,"start":118,"end":120}},"open_paren_span":{"file":0,"start":120,"end":121},"arguments":[0],"close_paren_span":{"file":0,"start":122,"end":123}}},{span":{"file":0,"start":132,"end":133},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":132,"end":133}}}},{"span":{"file":0,"start":132,"end":135},"kind":{"kind":"field-access","base":2,"dot_span":{"file":0,"start":133,"end":134},"field":{"text":"x","span":{"file":0,"start":134,"end":135}}}},{"span":{"file":0,"start":138,"end":139},"kind":{"kind":"reference","name":{"text":"q","span":{"file":0,"start":138,"end":139}}}},{"span":{"file":0,"start":138,"end":141},"kind":{"kind":"field-access","base":4,"dot_span":{"file":0,"start":139,"end":140},"field":{"text":"x","span":{"file":0,"start":140,"end":141}}}},{"span":{"file":0,"start":132,"end":141},"kind":{"kind":"addition","operator_span":{"file":0,"start":136,"end":137},"lhs":3,"rhs":5}}]}}]}],"diagnostics":[]}}"#;
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
const OWNED_TYPES_SOURCE: &str = "interface OwnedBox extends ZrynaStruct { value: String; }\ninterface Node extends ZrynaStruct { children: Vec<Node>; }\nfunction inspect(a: Vec<String>, b: Vec<String>, box: OwnedBox, node: Node): i32 { const xs: Vec<String> = Vec<String>([\"x\"]); return 0; }";
const OWNED_TYPES_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":48,"end":54},"kind":{"kind":"string","keyword_span":{"file":0,"start":48,"end":54}}},{"span":{"file":0,"start":109,"end":113},"kind":{"kind":"named","name":{"text":"Node","span":{"file":0,"start":109,"end":113}}}},{"span":{"file":0,"start":105,"end":114},"kind":{"kind":"vec","keyword_span":{"file":0,"start":105,"end":108},"less_than_span":{"file":0,"start":108,"end":109},"argument":1,"greater_than_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":142,"end":148},"kind":{"kind":"string","keyword_span":{"file":0,"start":142,"end":148}}},{"span":{"file":0,"start":138,"end":149},"kind":{"kind":"vec","keyword_span":{"file":0,"start":138,"end":141},"less_than_span":{"file":0,"start":141,"end":142},"argument":3,"greater_than_span":{"file":0,"start":148,"end":149}}},{"span":{"file":0,"start":158,"end":164},"kind":{"kind":"string","keyword_span":{"file":0,"start":158,"end":164}}},{"span":{"file":0,"start":154,"end":165},"kind":{"kind":"vec","keyword_span":{"file":0,"start":154,"end":157},"less_than_span":{"file":0,"start":157,"end":158},"argument":5,"greater_than_span":{"file":0,"start":164,"end":165}}},{"span":{"file":0,"start":172,"end":180},"kind":{"kind":"named","name":{"text":"OwnedBox","span":{"file":0,"start":172,"end":180}}}},{"span":{"file":0,"start":188,"end":192},"kind":{"kind":"named","name":{"text":"Node","span":{"file":0,"start":188,"end":192}}},{"span":{"file":0,"start":195,"end":198},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":195,"end":198}}},{"span":{"file":0,"start":215,"end":221},"kind":{"kind":"string","keyword_span":{"file":0,"start":215,"end":221}}},{"span":{"file":0,"start":211,"end":222},"kind":{"kind":"vec","keyword_span":{"file":0,"start":211,"end":214},"less_than_span":{"file":0,"start":214,"end":215},"argument":10,"greater_than_span":{"file":0,"start":221,"end":222}}},{"span":{"file":0,"start":229,"end":235},"kind":{"kind":"string","keyword_span":{"file":0,"start":229,"end":235}}},{"span":{"file":0,"start":225,"end":236},"kind":{"kind":"vec","keyword_span":{"file":0,"start":225,"end":228},"less_than_span":{"file":0,"start":228,"end":229},"argument":12,"greater_than_span":{"file":0,"start":235,"end":236}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":57},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"OwnedBox","span":{"file":0,"start":10,"end":18}},"extends_span":{"file":0,"start":19,"end":26},"marker_span":{"file":0,"start":27,"end":38},"open_brace_span":{"file":0,"start":39,"end":40},"close_brace_span":{"file":0,"start":56,"end":57},"fields":[{"span":{"file":0,"start":41,"end":55},"name":{"text":"value","span":{"file":0,"start":41,"end":46}},"colon_span":{"file":0,"start":46,"end":47},"semicolon_span":{"file":0,"start":54,"end":55},"type_syntax":0}]}},{"span":{"file":0,"start":58,"end":117},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":58,"end":67},"name":{"text":"Node","span":{"file":0,"start":68,"end":72}},"extends_span":{"file":0,"start":73,"end":80},"marker_span":{"file":0,"start":81,"end":92},"open_brace_span":{"file":0,"start":93,"end":94},"close_brace_span":{"file":0,"start":116,"end":117},"fields":[{"span":{"file":0,"start":95,"end":115},"name":{"text":"children","span":{"file":0,"start":95,"end":103}},"colon_span":{"file":0,"start":103,"end":104},"semicolon_span":{"file":0,"start":114,"end":115},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":118,"end":256},"export_span":null,"function_span":{"file":0,"start":118,"end":126},"name":{"text":"inspect","span":{"file":0,"start":127,"end":134}},"parameters":[{"span":{"file":0,"start":135,"end":149},"name":{"text":"a","span":{"file":0,"start":135,"end":136}},"type_syntax":4},{"span":{"file":0,"start":151,"end":165},"name":{"text":"b","span":{"file":0,"start":151,"end":152}},"type_syntax":6},{"span":{"file":0,"start":167,"end":180},"name":{"text":"box","span":{"file":0,"start":167,"end":170}},"type_syntax":7},{"span":{"file":0,"start":182,"end":192},"name":{"text":"node","span":{"file":0,"start":182,"end":186}},"type_syntax":8}],"result_type":9,"body":{"span":{"file":0,"start":199,"end":256},"root_block":0,"blocks":[{"span":{"file":0,"start":199,"end":256},"open_brace_span":{"file":0,"start":199,"end":200},"statements":[0,1],"close_brace_span":{"file":0,"start":255,"end":256}}],"statements":[{"span":{"file":0,"start":201,"end":244},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":201,"end":206},"mutable":false,"name":{"text":"xs","span":{"file":0,"start":207,"end":209}},"type_syntax":11,"equals_span":{"file":0,"start":223,"end":224},"initializer":1,"semicolon_span":{"file":0,"start":243,"end":244}}},{"span":{"file":0,"start":245,"end":254},"kind":{"kind":"return","keyword_span":{"file":0,"start":245,"end":251},"value":2,"semicolon_span":{"file":0,"start":253,"end":254}}}],"expressions":[{"span":{"file":0,"start":238,"end":241},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":225,"end":243},"kind":{"kind":"vec-construction","type_syntax":13,"open_paren_span":{"file":0,"start":236,"end":237},"open_bracket_span":{"file":0,"start":237,"end":238},"elements":[0],"close_bracket_span":{"file":0,"start":241,"end":242},"close_paren_span":{"file":0,"start":242,"end":243}}},{"span":{"file":0,"start":252,"end":253},"kind":{"kind":"i32-literal","spelling":"0"}}]}}]}],"diagnostics":[]}}"#;
const ENUM_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction get(x: Maybe): i32 { return match(x, { \"Maybe.none\": () => 0, \"Maybe.some\": (value) => value }); }";
const ENUM_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":82,"end":87},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":82,"end":87}}}},{"span":{"file":0,"start":90,"end":93},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":90,"end":93}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":173},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"get","span":{"file":0,"start":75,"end":78}},"parameters":[{"span":{"file":0,"start":79,"end":87},"name":{"text":"x","span":{"file":0,"start":79,"end":80}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":94,"end":173},"root_block":0,"blocks":[{"span":{"file":0,"start":94,"end":173},"open_brace_span":{"file":0,"start":94,"end":95},"statements":[0],"close_brace_span":{"file":0,"start":172,"end":173}}],"statements":[{"span":{"file":0,"start":96,"end":171},"kind":{"kind":"return","keyword_span":{"file":0,"start":96,"end":102},"value":3,"semicolon_span":{"file":0,"start":170,"end":171}}}],"expressions":[{"span":{"file":0,"start":109,"end":110},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":109,"end":110}}}},{"span":{"file":0,"start":134,"end":135},"kind":{"kind":"i32-literal","spelling":"0"}},{"span":{"file":0,"start":162,"end":167},"kind":{"kind":"reference","name":{"text":"value","span":{"file":0,"start":162,"end":167}}}},{"span":{"file":0,"start":103,"end":170},"kind":{"kind":"match","keyword_span":{"file":0,"start":103,"end":108},"open_paren_span":{"file":0,"start":108,"end":109},"scrutinee":0,"close_paren_span":{"file":0,"start":169,"end":170},"open_brace_span":{"file":0,"start":112,"end":113},"arms":[{"span":{"file":0,"start":114,"end":135},"type_name":{"text":"Maybe","span":{"file":0,"start":115,"end":120}},"dot_span":{"file":0,"start":120,"end":121},"variant":{"text":"none","span":{"file":0,"start":121,"end":125}},"binding":null,"arrow_span":{"file":0,"start":131,"end":133},"value":1},{"span":{"file":0,"start":137,"end":167},"type_name":{"text":"Maybe","span":{"file":0,"start":138,"end":143}},"dot_span":{"file":0,"start":143,"end":144},"variant":{"text":"some","span":{"file":0,"start":144,"end":148}},"binding":{"text":"value","span":{"file":0,"start":152,"end":157}},"arrow_span":{"file":0,"start":159,"end":161},"value":2}],"close_brace_span":{"file":0,"start":168,"end":169}}}]}}]}],"diagnostics":[]}}"#;
const ARRAY_OOB_SOURCE: &str = "function get(xs: FixedArray<i32, 2>): i32 { return xs[2]; }";
const ARRAY_VALID_SOURCE: &str = "function get(xs: FixedArray<i32, 2>): i32 { return xs[1]; }";
const ARRAY_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":38,"end":41},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":38,"end":41}}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":59},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"get","span":{"file":0,"start":9,"end":12}},"parameters":[{"span":{"file":0,"start":13,"end":35},"name":{"text":"xs","span":{"file":0,"start":13,"end":15}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":42,"end":59},"root_block":0,"blocks":[{"span":{"file":0,"start":42,"end":59},"open_brace_span":{"file":0,"start":42,"end":43},"statements":[0],"close_brace_span":{"file":0,"start":58,"end":59}}],"statements":[{"span":{"file":0,"start":44,"end":57},"kind":{"kind":"return","keyword_span":{"file":0,"start":44,"end":50},"value":2,"semicolon_span":{"file":0,"start":56,"end":57}}}],"expressions":[{"span":{"file":0,"start":51,"end":53},"kind":{"kind":"reference","name":{"text":"xs","span":{"file":0,"start":51,"end":53}}}},{"span":{"file":0,"start":54,"end":55},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":51,"end":56},"kind":{"kind":"index","base":0,"open_bracket_span":{"file":0,"start":53,"end":54},"index":1,"close_bracket_span":{"file":0,"start":55,"end":56}}}]}}]}],"diagnostics":[]}}"#;
const ARRAY_CONSTRUCT_SOURCE: &str =
    "function make(): FixedArray<i32, 2> { return FixedArray<i32, 2>([1, 2]); }";
const ARRAY_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":56,"end":59},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":56,"end":59}}}},{"span":{"file":0,"start":45,"end":63},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":45,"end":55},"less_than_span":{"file":0,"start":55,"end":56},"element":2,"comma_span":{"file":0,"start":59,"end":60},"length_span":{"file":0,"start":61,"end":62},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":62,"end":63}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":74},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":36,"end":74},"root_block":0,"blocks":[{"span":{"file":0,"start":36,"end":74},"open_brace_span":{"file":0,"start":36,"end":37},"statements":[0],"close_brace_span":{"file":0,"start":73,"end":74}}],"statements":[{"span":{"file":0,"start":38,"end":72},"kind":{"kind":"return","keyword_span":{"file":0,"start":38,"end":44},"value":2,"semicolon_span":{"file":0,"start":71,"end":72}}}],"expressions":[{"span":{"file":0,"start":65,"end":66},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":68,"end":69},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":45,"end":71},"kind":{"kind":"fixed-array-construction","type_syntax":3,"open_paren_span":{"file":0,"start":63,"end":64},"open_bracket_span":{"file":0,"start":64,"end":65},"elements":[0,1],"close_bracket_span":{"file":0,"start":69,"end":70},"close_paren_span":{"file":0,"start":70,"end":71}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_PAIR_SOURCE: &str = "interface OwnedPair extends ZrynaStruct { first: String; flag: bool; }\nfunction make(): OwnedPair { const p: OwnedPair = OwnedPair({ flag: true, first: \"a\" }); return p; }";
const OWNED_PAIR_RESPONSE: &str = r#"{"id":81,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":49,"end":55},"kind":{"kind":"string","keyword_span":{"file":0,"start":49,"end":55}}},{"span":{"file":0,"start":63,"end":67},"kind":{"kind":"named","name":{"text":"bool","span":{"file":0,"start":63,"end":67}}}},{"span":{"file":0,"start":88,"end":97},"kind":{"kind":"named","name":{"text":"OwnedPair","span":{"file":0,"start":88,"end":97}}}},{"span":{"file":0,"start":109,"end":118},"kind":{"kind":"named","name":{"text":"OwnedPair","span":{"file":0,"start":109,"end":118}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":70},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"OwnedPair","span":{"file":0,"start":10,"end":19}},"extends_span":{"file":0,"start":20,"end":27},"marker_span":{"file":0,"start":28,"end":39},"open_brace_span":{"file":0,"start":40,"end":41},"close_brace_span":{"file":0,"start":69,"end":70},"fields":[{"span":{"file":0,"start":42,"end":56},"name":{"text":"first","span":{"file":0,"start":42,"end":47}},"colon_span":{"file":0,"start":47,"end":48},"semicolon_span":{"file":0,"start":55,"end":56},"type_syntax":0},{"span":{"file":0,"start":57,"end":68},"name":{"text":"flag","span":{"file":0,"start":57,"end":61}},"colon_span":{"file":0,"start":61,"end":62},"semicolon_span":{"file":0,"start":67,"end":68},"type_syntax":1}]}}],"functions":[{"span":{"file":0,"start":71,"end":171},"export_span":null,"function_span":{"file":0,"start":71,"end":79},"name":{"text":"make","span":{"file":0,"start":80,"end":84}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":98,"end":171},"root_block":0,"blocks":[{"span":{"file":0,"start":98,"end":171},"open_brace_span":{"file":0,"start":98,"end":99},"statements":[0,1],"close_brace_span":{"file":0,"start":170,"end":171}}],"statements":[{"span":{"file":0,"start":100,"end":159},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":100,"end":105},"mutable":false,"name":{"text":"p","span":{"file":0,"start":106,"end":107}},"type_syntax":3,"equals_span":{"file":0,"start":119,"end":120},"initializer":2,"semicolon_span":{"file":0,"start":158,"end":159}}},{"span":{"file":0,"start":160,"end":169},"kind":{"kind":"return","keyword_span":{"file":0,"start":160,"end":166},"value":3,"semicolon_span":{"file":0,"start":168,"end":169}}}],"expressions":[{"span":{"file":0,"start":139,"end":143},"kind":{"kind":"bool-literal","value":true}},{"span":{"file":0,"start":152,"end":155},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":121,"end":158},"kind":{"kind":"struct-construction","type_name":{"text":"OwnedPair","span":{"file":0,"start":121,"end":130}},"open_paren_span":{"file":0,"start":130,"end":131},"open_brace_span":{"file":0,"start":131,"end":132},"fields":[{"span":{"file":0,"start":133,"end":143},"kind":{"kind":"explicit","name":{"text":"flag","span":{"file":0,"start":133,"end":137}},"colon_span":{"file":0,"start":137,"end":138},"value":0}},{"span":{"file":0,"start":145,"end":155},"kind":{"kind":"explicit","name":{"text":"first","span":{"file":0,"start":145,"end":150}},"colon_span":{"file":0,"start":150,"end":151},"value":1}}],"close_brace_span":{"file":0,"start":156,"end":157},"close_paren_span":{"file":0,"start":157,"end":158}}},{"span":{"file":0,"start":167,"end":168},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":167,"end":168}}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ARRAY_SOURCE: &str = "function make(): FixedArray<String, 2> { const a: FixedArray<String, 2> = FixedArray<String, 2>([\"x\", \"y\"]); return a; }";
const OWNED_ARRAY_RESPONSE: &str = r#"{"id":82,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":34},"kind":{"kind":"string","keyword_span":{"file":0,"start":28,"end":34}}},{"span":{"file":0,"start":17,"end":38},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":34,"end":35},"length_span":{"file":0,"start":36,"end":37},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":37,"end":38}}},{"span":{"file":0,"start":61,"end":67},"kind":{"kind":"string","keyword_span":{"file":0,"start":61,"end":67}}},{"span":{"file":0,"start":50,"end":71},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":50,"end":60},"less_than_span":{"file":0,"start":60,"end":61},"element":2,"comma_span":{"file":0,"start":67,"end":68},"length_span":{"file":0,"start":69,"end":70},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":70,"end":71}}},{"span":{"file":0,"start":85,"end":91},"kind":{"kind":"string","keyword_span":{"file":0,"start":85,"end":91}}},{"span":{"file":0,"start":74,"end":95},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":74,"end":84},"less_than_span":{"file":0,"start":84,"end":85},"element":4,"comma_span":{"file":0,"start":91,"end":92},"length_span":{"file":0,"start":93,"end":94},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":94,"end":95}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":120},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":39,"end":120},"root_block":0,"blocks":[{"span":{"file":0,"start":39,"end":120},"open_brace_span":{"file":0,"start":39,"end":40},"statements":[0,1],"close_brace_span":{"file":0,"start":119,"end":120}}],"statements":[{"span":{"file":0,"start":41,"end":108},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":41,"end":46},"mutable":false,"name":{"text":"a","span":{"file":0,"start":47,"end":48}},"type_syntax":3,"equals_span":{"file":0,"start":72,"end":73},"initializer":2,"semicolon_span":{"file":0,"start":107,"end":108}}},{"span":{"file":0,"start":109,"end":118},"kind":{"kind":"return","keyword_span":{"file":0,"start":109,"end":115},"value":3,"semicolon_span":{"file":0,"start":117,"end":118}}}],"expressions":[{"span":{"file":0,"start":97,"end":100},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":102,"end":105},"kind":{"kind":"string-literal","spelling":"\"y\""}},{"span":{"file":0,"start":74,"end":107},"kind":{"kind":"fixed-array-construction","type_syntax":5,"open_paren_span":{"file":0,"start":95,"end":96},"open_bracket_span":{"file":0,"start":96,"end":97},"elements":[0,1],"close_bracket_span":{"file":0,"start":105,"end":106},"close_paren_span":{"file":0,"start":106,"end":107}}},{"span":{"file":0,"start":116,"end":117},"kind":{"kind":"reference","name":{"text":"a","span":{"file":0,"start":116,"end":117}}}}]}}]}],"diagnostics":[]}}"#;
const NESTED_OWNED_SOURCE: &str = "interface Inner extends ZrynaStruct { text: String; }\ninterface Outer extends ZrynaStruct { inner: Inner; tail: String; }\nfunction make(): Outer { return Outer({ tail: \"b\", inner: Inner({ text: \"a\" }) }); }";
const NESTED_OWNED_RESPONSE: &str = r#"{"id":83,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"named","name":{"text":"Inner","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":112,"end":118},"kind":{"kind":"string","keyword_span":{"file":0,"start":112,"end":118}}},{"span":{"file":0,"start":139,"end":144},"kind":{"kind":"named","name":{"text":"Outer","span":{"file":0,"start":139,"end":144}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":53},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Inner","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":35},"open_brace_span":{"file":0,"start":36,"end":37},"close_brace_span":{"file":0,"start":52,"end":53},"fields":[{"span":{"file":0,"start":38,"end":51},"name":{"text":"text","span":{"file":0,"start":38,"end":42}},"colon_span":{"file":0,"start":42,"end":43},"semicolon_span":{"file":0,"start":50,"end":51},"type_syntax":0}]}},{"span":{"file":0,"start":54,"end":121},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":54,"end":63},"name":{"text":"Outer","span":{"file":0,"start":64,"end":69}},"extends_span":{"file":0,"start":70,"end":77},"marker_span":{"file":0,"start":78,"end":89},"open_brace_span":{"file":0,"start":90,"end":91},"close_brace_span":{"file":0,"start":120,"end":121},"fields":[{"span":{"file":0,"start":92,"end":105},"name":{"text":"inner","span":{"file":0,"start":92,"end":97}},"colon_span":{"file":0,"start":97,"end":98},"semicolon_span":{"file":0,"start":104,"end":105},"type_syntax":1},{"span":{"file":0,"start":106,"end":119},"name":{"text":"tail","span":{"file":0,"start":106,"end":110}},"colon_span":{"file":0,"start":110,"end":111},"semicolon_span":{"file":0,"start":118,"end":119},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":122,"end":206},"export_span":null,"function_span":{"file":0,"start":122,"end":130},"name":{"text":"make","span":{"file":0,"start":131,"end":135}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":145,"end":206},"root_block":0,"blocks":[{"span":{"file":0,"start":145,"end":206},"open_brace_span":{"file":0,"start":145,"end":146},"statements":[0],"close_brace_span":{"file":0,"start":205,"end":206}}],"statements":[{"span":{"file":0,"start":147,"end":204},"kind":{"kind":"return","keyword_span":{"file":0,"start":147,"end":153},"value":3,"semicolon_span":{"file":0,"start":203,"end":204}}}],"expressions":[{"span":{"file":0,"start":168,"end":171},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":194,"end":197},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":180,"end":200},"kind":{"kind":"struct-construction","type_name":{"text":"Inner","span":{"file":0,"start":180,"end":185}},"open_paren_span":{"file":0,"start":185,"end":186},"open_brace_span":{"file":0,"start":186,"end":187},"fields":[{"span":{"file":0,"start":188,"end":197},"kind":{"kind":"explicit","name":{"text":"text","span":{"file":0,"start":188,"end":192}},"colon_span":{"file":0,"start":192,"end":193},"value":1}}],"close_brace_span":{"file":0,"start":198,"end":199},"close_paren_span":{"file":0,"start":199,"end":200}}},{"span":{"file":0,"start":154,"end":203},"kind":{"kind":"struct-construction","type_name":{"text":"Outer","span":{"file":0,"start":154,"end":159}},"open_paren_span":{"file":0,"start":159,"end":160},"open_brace_span":{"file":0,"start":160,"end":161},"fields":[{"span":{"file":0,"start":162,"end":171},"kind":{"kind":"explicit","name":{"text":"tail","span":{"file":0,"start":162,"end":166}},"colon_span":{"file":0,"start":166,"end":167},"value":0}},{"span":{"file":0,"start":173,"end":200},"kind":{"kind":"explicit","name":{"text":"inner","span":{"file":0,"start":173,"end":178}},"colon_span":{"file":0,"start":178,"end":179},"value":2}}],"close_brace_span":{"file":0,"start":201,"end":202},"close_paren_span":{"file":0,"start":202,"end":203}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_TRIO_SOURCE: &str = "interface Trio extends ZrynaStruct { a: String; b: String; c: String; }\nfunction make(): Trio { return Trio({ c: \"c\", b: \"b\", a: \"a\" }); }";
const OWNED_TRIO_RESPONSE: &str = r#"{"id":84,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":40,"end":46},"kind":{"kind":"string","keyword_span":{"file":0,"start":40,"end":46}}},{"span":{"file":0,"start":51,"end":57},"kind":{"kind":"string","keyword_span":{"file":0,"start":51,"end":57}}},{"span":{"file":0,"start":62,"end":68},"kind":{"kind":"string","keyword_span":{"file":0,"start":62,"end":68}}},{"span":{"file":0,"start":89,"end":93},"kind":{"kind":"named","name":{"text":"Trio","span":{"file":0,"start":89,"end":93}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":71},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Trio","span":{"file":0,"start":10,"end":14}},"extends_span":{"file":0,"start":15,"end":22},"marker_span":{"file":0,"start":23,"end":34},"open_brace_span":{"file":0,"start":35,"end":36},"close_brace_span":{"file":0,"start":70,"end":71},"fields":[{"span":{"file":0,"start":37,"end":47},"name":{"text":"a","span":{"file":0,"start":37,"end":38}},"colon_span":{"file":0,"start":38,"end":39},"semicolon_span":{"file":0,"start":46,"end":47},"type_syntax":0},{"span":{"file":0,"start":48,"end":58},"name":{"text":"b","span":{"file":0,"start":48,"end":49}},"colon_span":{"file":0,"start":49,"end":50},"semicolon_span":{"file":0,"start":57,"end":58},"type_syntax":1},{"span":{"file":0,"start":59,"end":69},"name":{"text":"c","span":{"file":0,"start":59,"end":60}},"colon_span":{"file":0,"start":60,"end":61},"semicolon_span":{"file":0,"start":68,"end":69},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":72,"end":138},"export_span":null,"function_span":{"file":0,"start":72,"end":80},"name":{"text":"make","span":{"file":0,"start":81,"end":85}},"parameters":[],"result_type":3,"body":{"span":{"file":0,"start":94,"end":138},"root_block":0,"blocks":[{"span":{"file":0,"start":94,"end":138},"open_brace_span":{"file":0,"start":94,"end":95},"statements":[0],"close_brace_span":{"file":0,"start":137,"end":138}}],"statements":[{"span":{"file":0,"start":96,"end":136},"kind":{"kind":"return","keyword_span":{"file":0,"start":96,"end":102},"value":3,"semicolon_span":{"file":0,"start":135,"end":136}}}],"expressions":[{"span":{"file":0,"start":113,"end":116},"kind":{"kind":"string-literal","spelling":"\"c\""}},{"span":{"file":0,"start":121,"end":124},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":129,"end":132},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":103,"end":135},"kind":{"kind":"struct-construction","type_name":{"text":"Trio","span":{"file":0,"start":103,"end":107}},"open_paren_span":{"file":0,"start":107,"end":108},"open_brace_span":{"file":0,"start":108,"end":109},"fields":[{"span":{"file":0,"start":110,"end":116},"kind":{"kind":"explicit","name":{"text":"c","span":{"file":0,"start":110,"end":111}},"colon_span":{"file":0,"start":111,"end":112},"value":0}},{"span":{"file":0,"start":118,"end":124},"kind":{"kind":"explicit","name":{"text":"b","span":{"file":0,"start":118,"end":119}},"colon_span":{"file":0,"start":119,"end":120},"value":1}},{"span":{"file":0,"start":126,"end":132},"kind":{"kind":"explicit","name":{"text":"a","span":{"file":0,"start":126,"end":127}},"colon_span":{"file":0,"start":127,"end":128},"value":2}}],"close_brace_span":{"file":0,"start":133,"end":134},"close_paren_span":{"file":0,"start":134,"end":135}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_CROSS_SOURCE: &str = "interface Box extends ZrynaStruct { items: FixedArray<String, 2>; }\nfunction make(): Box { return Box({ items: FixedArray<String, 2>([\"a\", \"b\"]) }); }";
const OWNED_CROSS_RESPONSE: &str = r#"{"id":85,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":54,"end":60},"kind":{"kind":"string","keyword_span":{"file":0,"start":54,"end":60}}},{"span":{"file":0,"start":43,"end":64},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":43,"end":53},"less_than_span":{"file":0,"start":53,"end":54},"element":0,"comma_span":{"file":0,"start":60,"end":61},"length_span":{"file":0,"start":62,"end":63},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":63,"end":64}}},{"span":{"file":0,"start":85,"end":88},"kind":{"kind":"named","name":{"text":"Box","span":{"file":0,"start":85,"end":88}}}},{"span":{"file":0,"start":122,"end":128},"kind":{"kind":"string","keyword_span":{"file":0,"start":122,"end":128}}},{"span":{"file":0,"start":111,"end":132},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":111,"end":121},"less_than_span":{"file":0,"start":121,"end":122},"element":3,"comma_span":{"file":0,"start":128,"end":129},"length_span":{"file":0,"start":130,"end":131},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":131,"end":132}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":67},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Box","span":{"file":0,"start":10,"end":13}},"extends_span":{"file":0,"start":14,"end":21},"marker_span":{"file":0,"start":22,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":66,"end":67},"fields":[{"span":{"file":0,"start":36,"end":65},"name":{"text":"items","span":{"file":0,"start":36,"end":41}},"colon_span":{"file":0,"start":41,"end":42},"semicolon_span":{"file":0,"start":64,"end":65},"type_syntax":1}]}}],"functions":[{"span":{"file":0,"start":68,"end":150},"export_span":null,"function_span":{"file":0,"start":68,"end":76},"name":{"text":"make","span":{"file":0,"start":77,"end":81}},"parameters":[],"result_type":2,"body":{"span":{"file":0,"start":89,"end":150},"root_block":0,"blocks":[{"span":{"file":0,"start":89,"end":150},"open_brace_span":{"file":0,"start":89,"end":90},"statements":[0],"close_brace_span":{"file":0,"start":149,"end":150}}],"statements":[{"span":{"file":0,"start":91,"end":148},"kind":{"kind":"return","keyword_span":{"file":0,"start":91,"end":97},"value":3,"semicolon_span":{"file":0,"start":147,"end":148}}}],"expressions":[{"span":{"file":0,"start":134,"end":137},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":139,"end":142},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":111,"end":144},"kind":{"kind":"fixed-array-construction","type_syntax":4,"open_paren_span":{"file":0,"start":132,"end":133},"open_bracket_span":{"file":0,"start":133,"end":134},"elements":[0,1],"close_bracket_span":{"file":0,"start":142,"end":143},"close_paren_span":{"file":0,"start":143,"end":144}}},{"span":{"file":0,"start":98,"end":147},"kind":{"kind":"struct-construction","type_name":{"text":"Box","span":{"file":0,"start":98,"end":101}},"open_paren_span":{"file":0,"start":101,"end":102},"open_brace_span":{"file":0,"start":102,"end":103},"fields":[{"span":{"file":0,"start":104,"end":144},"kind":{"kind":"explicit","name":{"text":"items","span":{"file":0,"start":104,"end":109}},"colon_span":{"file":0,"start":109,"end":110},"value":2}}],"close_brace_span":{"file":0,"start":145,"end":146},"close_paren_span":{"file":0,"start":146,"end":147}}}]}}]}],"diagnostics":[]}}"#;
const ENUM_CONSTRUCT_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction make(x: i32): Maybe { return Maybe.some(x); }";
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
const ENUM_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":83,"end":86},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":83,"end":86}}}},{"span":{"file":0,"start":89,"end":94},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":89,"end":94}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":120},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"make","span":{"file":0,"start":75,"end":79}},"parameters":[{"span":{"file":0,"start":80,"end":86},"name":{"text":"x","span":{"file":0,"start":80,"end":81}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":95,"end":120},"root_block":0,"blocks":[{"span":{"file":0,"start":95,"end":120},"open_brace_span":{"file":0,"start":95,"end":96},"statements":[0],"close_brace_span":{"file":0,"start":119,"end":120}}],"statements":[{"span":{"file":0,"start":97,"end":118},"kind":{"kind":"return","keyword_span":{"file":0,"start":97,"end":103},"value":1,"semicolon_span":{"file":0,"start":117,"end":118}}}],"expressions":[{"span":{"file":0,"start":115,"end":116},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":115,"end":116}}}},{"span":{"file":0,"start":104,"end":117},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":104,"end":109}},"dot_span":{"file":0,"start":109,"end":110},"variant":{"text":"some","span":{"file":0,"start":110,"end":114}},"open_paren_span":{"file":0,"start":114,"end":115},"payload":0,"close_paren_span":{"file":0,"start":116,"end":117}}}]}}]}],"diagnostics":[]}}"#;

const OWNED_ENUM_DUP_RETURN_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { return Maybe.none(); return Maybe.none(); }";
const OWNED_ENUM_DUP_RETURN_RESPONSE: &str = r#"{"id":20,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":137},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":137},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":137},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1],"close_brace_span":{"file":0,"start":136,"end":137}}],"statements":[{"span":{"file":0,"start":94,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":94,"end":100},"value":0,"semicolon_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":115,"end":135},"kind":{"kind":"return","keyword_span":{"file":0,"start":115,"end":121},"value":1,"semicolon_span":{"file":0,"start":134,"end":135}}}],"expressions":[{"span":{"file":0,"start":101,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":101,"end":106}},"dot_span":{"file":0,"start":106,"end":107},"variant":{"text":"none","span":{"file":0,"start":107,"end":111}},"open_paren_span":{"file":0,"start":111,"end":112},"payload":null,"close_paren_span":{"file":0,"start":112,"end":113}}},{"span":{"file":0,"start":122,"end":134},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":122,"end":127}},"dot_span":{"file":0,"start":127,"end":128},"variant":{"text":"none","span":{"file":0,"start":128,"end":132}},"open_paren_span":{"file":0,"start":132,"end":133},"payload":null,"close_paren_span":{"file":0,"start":133,"end":134}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_LOCAL_AFTER_RETURN_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: String; }\nfunction make(): Maybe { return Maybe.none(); const x: Maybe = Maybe.none(); }";
const OWNED_ENUM_LOCAL_AFTER_RETURN_RESPONSE: &str = r#"{"id":21,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":65},"kind":{"kind":"string","keyword_span":{"file":0,"start":59,"end":65}}},{"span":{"file":0,"start":86,"end":91},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":86,"end":91}}}},{"span":{"file":0,"start":124,"end":129},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":124,"end":129}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":68},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":67,"end":68},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":66},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":65,"end":66},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":69,"end":147},"export_span":null,"function_span":{"file":0,"start":69,"end":77},"name":{"text":"make","span":{"file":0,"start":78,"end":82}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":92,"end":147},"root_block":0,"blocks":[{"span":{"file":0,"start":92,"end":147},"open_brace_span":{"file":0,"start":92,"end":93},"statements":[0,1],"close_brace_span":{"file":0,"start":146,"end":147}}],"statements":[{"span":{"file":0,"start":94,"end":114},"kind":{"kind":"return","keyword_span":{"file":0,"start":94,"end":100},"value":0,"semicolon_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":115,"end":145},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":115,"end":120},"mutable":false,"name":{"text":"x","span":{"file":0,"start":121,"end":122}},"type_syntax":2,"equals_span":{"file":0,"start":130,"end":131},"initializer":1,"semicolon_span":{"file":0,"start":144,"end":145}}}],"expressions":[{"span":{"file":0,"start":101,"end":113},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":101,"end":106}},"dot_span":{"file":0,"start":106,"end":107},"variant":{"text":"none","span":{"file":0,"start":107,"end":111}},"open_paren_span":{"file":0,"start":111,"end":112},"payload":null,"close_paren_span":{"file":0,"start":112,"end":113}}},{"span":{"file":0,"start":132,"end":144},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":132,"end":137}},"dot_span":{"file":0,"start":137,"end":138},"variant":{"text":"none","span":{"file":0,"start":138,"end":142}},"open_paren_span":{"file":0,"start":142,"end":143},"payload":null,"close_paren_span":{"file":0,"start":143,"end":144}}}]}}]}],"diagnostics":[]}}"#;

fn response_snapshot(response: &str) -> RawProjectSyntaxSnapshot {
    let value: serde_json::Value = serde_json::from_str(response).expect("adapter response JSON");
    let result = value.get("result").expect("adapter result");
    decode_snapshot(&serde_json::to_vec(result).expect("snapshot JSON")).expect("v4 snapshot")
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

#[derive(Clone, Copy)]
enum StringAssignmentRhs {
    Move,
    Literal,
    Clone,
    Concat,
    SelfMove,
    CallSelf,
    CloneCallSelf,
}

#[allow(clippy::too_many_lines)]
fn string_assignment_snapshot(
    rhs: StringAssignmentRhs,
) -> (&'static str, RawProjectSyntaxSnapshot) {
    const LITERAL: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = \"fresh\"; return x; }";
    const CLONE: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = clone(y); return x; }";
    const CONCAT: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = concat(x, y); return x; }";
    const SELF_MOVE: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = x; return x; }";
    const CALL_SELF: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = take(x); return x; }";
    const CLONE_CALL_SELF: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = clone(take(x)); return x; }";
    let mut raw = response_snapshot(STRING_ASSIGN_MOVE_RESPONSE);
    let (source, extra) = match rhs {
        StringAssignmentRhs::Move => return (STRING_ASSIGN_MOVE_SOURCE, raw),
        StringAssignmentRhs::Literal => (LITERAL, 6),
        StringAssignmentRhs::Clone => (CLONE, 7),
        StringAssignmentRhs::Concat => (CONCAT, 11),
        StringAssignmentRhs::SelfMove => {
            let zryna_syntax::v4::RawExpressionKind::Reference { name } =
                &mut raw.files[0].functions[0].body.expressions[3].kind
            else {
                panic!("assignment source")
            };
            name.text = "x".to_owned();
            return (SELF_MOVE, raw);
        }
        StringAssignmentRhs::CallSelf => (CALL_SELF, 6),
        StringAssignmentRhs::CloneCallSelf => (CLONE_CALL_SELF, 13),
    };
    raw = shift_snapshot(raw, 83, extra);
    let body = &mut raw.files[0].functions[0].body;
    body.statements[2].span.end += extra;
    let RawStatementKind::Assignment { value, semicolon_span, .. } = &mut body.statements[2].kind
    else {
        panic!("assignment")
    };
    semicolon_span.start += extra;
    semicolon_span.end += extra;
    match rhs {
        StringAssignmentRhs::Literal => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 87 },
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"fresh\"".to_owned(),
                },
            };
        }
        StringAssignmentRhs::Clone => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "y".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                    },
                },
            };
            *value = u32::try_from(body.expressions.len()).expect("expression id");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 88 },
                kind: zryna_syntax::v4::RawExpressionKind::Clone {
                    keyword_span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 85 },
                    open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 85, end: 86 },
                    value: 3,
                    close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                },
            });
        }
        StringAssignmentRhs::Concat => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                    },
                },
            };
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 90, end: 91 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "y".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 90, end: 91 },
                    },
                },
            });
            *value = u32::try_from(body.expressions.len()).expect("expression id");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 92 },
                kind: zryna_syntax::v4::RawExpressionKind::Call {
                    callee: RawIdentifierSyntax {
                        text: "concat".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 86 },
                    },
                    open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                    arguments: vec![3, 5],
                    close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 91, end: 92 },
                },
            });
        }
        StringAssignmentRhs::CallSelf | StringAssignmentRhs::CloneCallSelf => {
            let nested = matches!(rhs, StringAssignmentRhs::CloneCallSelf);
            let reference = if nested { (91, 92) } else { (85, 86) };
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: reference.0, end: reference.1 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: reference.0,
                            end: reference.1,
                        },
                    },
                },
            };
            let call_id = u32::try_from(body.expressions.len()).expect("call id");
            let (call_start, call_end, open, close) =
                if nested { (86, 93, 90, 92) } else { (80, 87, 84, 86) };
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: call_start, end: call_end },
                kind: zryna_syntax::v4::RawExpressionKind::Call {
                    callee: RawIdentifierSyntax {
                        text: "take".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: call_start,
                            end: call_start + 4,
                        },
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: open,
                        end: open + 1,
                    },
                    arguments: vec![3],
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: close,
                        end: close + 1,
                    },
                },
            });
            if nested {
                *value = u32::try_from(body.expressions.len()).expect("clone id");
                body.expressions.push(RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 94 },
                    kind: zryna_syntax::v4::RawExpressionKind::Clone {
                        keyword_span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 85 },
                        open_paren_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: 85,
                            end: 86,
                        },
                        value: call_id,
                        close_paren_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: 93,
                            end: 94,
                        },
                    },
                });
            } else {
                *value = call_id;
            }
        }
        StringAssignmentRhs::Move | StringAssignmentRhs::SelfMove => unreachable!(),
    }
    (source, raw)
}

fn owned_types_snapshot() -> RawProjectSyntaxSnapshot {
    let response = OWNED_TYPES_RESPONSE
        .replacen(
            "\"end\":192}}},{\"span\":{\"file\":0,\"start\":195",
            "\"end\":192}}}},{\"span\":{\"file\":0,\"start\":195",
            1,
        )
        .replacen(
            "\"end\":198}}},{\"span\":{\"file\":0,\"start\":215",
            "\"end\":198}}}},{\"span\":{\"file\":0,\"start\":215",
            1,
        );
    response_snapshot(&response)
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
fn private_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    fn append(text: &mut String, spelling: &str) -> zryna_source::UntrustedSpan {
        let start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(spelling);
        zryna_source::UntrustedSpan {
            file: 0,
            start,
            end: u32::try_from(text.len()).expect("fixture offset"),
        }
    }
    fn string_type(text: &mut String, types: &mut Vec<RawTypeSyntax>) -> u32 {
        let keyword_span = append(text, "String");
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: keyword_span,
            kind: RawTypeSyntaxKind::String { keyword_span },
        });
        id
    }

    let mut text = String::new();
    let mut types = Vec::new();
    let mut functions = Vec::new();

    let caller_start = u32::try_from(text.len()).expect("offset");
    let caller_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let caller_name_span = append(&mut text, "caller");
    append(&mut text, "()");
    append(&mut text, ": ");
    let caller_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let caller_body_start = u32::try_from(text.len()).expect("offset");
    let caller_open = append(&mut text, "{");
    append(&mut text, " ");

    let survivor_start = u32::try_from(text.len()).expect("offset");
    let survivor_keyword = append(&mut text, "const");
    append(&mut text, " ");
    let survivor_name = append(&mut text, "survivor");
    append(&mut text, ": ");
    let survivor_type = string_type(&mut text, &mut types);
    append(&mut text, " = ");
    let survivor_literal = append(&mut text, "\"keep\"");
    let survivor_semi = append(&mut text, ";");
    append(&mut text, " ");

    let value_start = u32::try_from(text.len()).expect("offset");
    let value_keyword = append(&mut text, "const");
    append(&mut text, " ");
    let value_name = append(&mut text, "value");
    append(&mut text, ": ");
    let value_type = string_type(&mut text, &mut types);
    append(&mut text, " = ");
    let identity_name = append(&mut text, "identity");
    let identity_open = append(&mut text, "(");
    let producer_name = append(&mut text, "producer");
    let producer_open = append(&mut text, "(");
    let producer_close = append(&mut text, ")");
    let identity_close = append(&mut text, ")");
    let value_semi = append(&mut text, ";");
    append(&mut text, " ");

    let return_start = u32::try_from(text.len()).expect("offset");
    let return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let clone_keyword = append(&mut text, "clone");
    let clone_open = append(&mut text, "(");
    let return_name = append(&mut text, "value");
    let clone_close = append(&mut text, ")");
    let return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let caller_close = append(&mut text, "}");
    let caller_end = caller_close.end;
    let caller_span = zryna_source::UntrustedSpan { file: 0, start: caller_start, end: caller_end };
    let caller_body_span =
        zryna_source::UntrustedSpan { file: 0, start: caller_body_start, end: caller_end };
    functions.push(RawFunctionSyntax {
        span: caller_span,
        export_span: None,
        function_span: caller_keyword,
        name: RawIdentifierSyntax { text: "caller".to_owned(), span: caller_name_span },
        parameters: Vec::new(),
        result_type: caller_result,
        body: RawFunctionBodySyntax {
            span: caller_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: caller_body_span,
                open_brace_span: caller_open,
                statements: vec![0, 1, 2],
                close_brace_span: caller_close,
            }],
            statements: vec![
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: survivor_start,
                        end: survivor_semi.end,
                    },
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: survivor_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax {
                            text: "survivor".to_owned(),
                            span: survivor_name,
                        },
                        type_syntax: survivor_type,
                        equals_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: survivor_literal.start - 2,
                            end: survivor_literal.start - 1,
                        },
                        initializer: 0,
                        semicolon_span: survivor_semi,
                    },
                },
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: value_start,
                        end: value_semi.end,
                    },
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: value_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax { text: "value".to_owned(), span: value_name },
                        type_syntax: value_type,
                        equals_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: identity_name.start - 2,
                            end: identity_name.start - 1,
                        },
                        initializer: 2,
                        semicolon_span: value_semi,
                    },
                },
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: return_start,
                        end: return_semi.end,
                    },
                    kind: RawStatementKind::Return {
                        keyword_span: return_keyword,
                        value: 4,
                        semicolon_span: return_semi,
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: survivor_literal,
                    kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                        spelling: "\"keep\"".to_owned(),
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: producer_name.start,
                        end: producer_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "producer".to_owned(),
                            span: producer_name,
                        },
                        open_paren_span: producer_open,
                        arguments: Vec::new(),
                        close_paren_span: producer_close,
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: identity_name.start,
                        end: identity_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "identity".to_owned(),
                            span: identity_name,
                        },
                        open_paren_span: identity_open,
                        arguments: vec![1],
                        close_paren_span: identity_close,
                    },
                },
                RawExpressionSyntax {
                    span: return_name,
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax { text: "value".to_owned(), span: return_name },
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: clone_keyword.start,
                        end: clone_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Clone {
                        keyword_span: clone_keyword,
                        open_paren_span: clone_open,
                        value: 3,
                        close_paren_span: clone_close,
                    },
                },
            ],
        },
    });

    append(&mut text, " ");
    let identity_start = u32::try_from(text.len()).expect("offset");
    let identity_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let identity_decl_name = append(&mut text, "identity");
    let identity_parameter_open = append(&mut text, "(");
    let parameter_start = u32::try_from(text.len()).expect("offset");
    let parameter_name = append(&mut text, "value");
    append(&mut text, ": ");
    let parameter_type = string_type(&mut text, &mut types);
    let identity_parameter_close = append(&mut text, ")");
    append(&mut text, ": ");
    let identity_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let identity_body_start = u32::try_from(text.len()).expect("offset");
    let identity_body_open = append(&mut text, "{");
    append(&mut text, " ");
    let identity_return_start = u32::try_from(text.len()).expect("offset");
    let identity_return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let identity_reference = append(&mut text, "value");
    let identity_return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let identity_body_close = append(&mut text, "}");
    let identity_end = identity_body_close.end;
    let identity_body_span =
        zryna_source::UntrustedSpan { file: 0, start: identity_body_start, end: identity_end };
    functions.push(RawFunctionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: identity_start, end: identity_end },
        export_span: None,
        function_span: identity_keyword,
        name: RawIdentifierSyntax { text: "identity".to_owned(), span: identity_decl_name },
        parameters: vec![RawParameterSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: parameter_start,
                end: types[usize::try_from(parameter_type).expect("type index")].span.end,
            },
            name: RawIdentifierSyntax { text: "value".to_owned(), span: parameter_name },
            type_syntax: parameter_type,
        }],
        result_type: identity_result,
        body: RawFunctionBodySyntax {
            span: identity_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: identity_body_span,
                open_brace_span: identity_body_open,
                statements: vec![0],
                close_brace_span: identity_body_close,
            }],
            statements: vec![RawStatementSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: identity_return_start,
                    end: identity_return_semi.end,
                },
                kind: RawStatementKind::Return {
                    keyword_span: identity_return_keyword,
                    value: 0,
                    semicolon_span: identity_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: identity_reference,
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "value".to_owned(),
                        span: identity_reference,
                    },
                },
            }],
        },
    });
    let _ = (identity_parameter_open, identity_parameter_close);

    append(&mut text, " ");
    let producer_start = u32::try_from(text.len()).expect("offset");
    let producer_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let producer_decl_name = append(&mut text, "producer");
    append(&mut text, "()");
    append(&mut text, ": ");
    let producer_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let producer_body_start = u32::try_from(text.len()).expect("offset");
    let producer_body_open = append(&mut text, "{");
    append(&mut text, " ");
    let producer_return_start = u32::try_from(text.len()).expect("offset");
    let producer_return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let made_literal = append(&mut text, "\"made\"");
    let producer_return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let producer_body_close = append(&mut text, "}");
    let producer_end = producer_body_close.end;
    let producer_body_span =
        zryna_source::UntrustedSpan { file: 0, start: producer_body_start, end: producer_end };
    functions.push(RawFunctionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: producer_start, end: producer_end },
        export_span: None,
        function_span: producer_keyword,
        name: RawIdentifierSyntax { text: "producer".to_owned(), span: producer_decl_name },
        parameters: Vec::new(),
        result_type: producer_result,
        body: RawFunctionBodySyntax {
            span: producer_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: producer_body_span,
                open_brace_span: producer_body_open,
                statements: vec![0],
                close_brace_span: producer_body_close,
            }],
            statements: vec![RawStatementSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: producer_return_start,
                    end: producer_return_semi.end,
                },
                kind: RawStatementKind::Return {
                    keyword_span: producer_return_keyword,
                    value: 0,
                    semicolon_span: producer_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: made_literal,
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"made\"".to_owned(),
                },
            }],
        },
    });

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
                functions,
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn private_nested_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = private_string_call_fixture();
    let old = "producer()";
    let replacement = "concat(survivor, \"x\")";
    let start = u32::try_from(source.find(old).expect("producer call")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let body = &mut raw.files[0].functions[0].body;
    let survivor_literal = body.expressions[0].clone();
    let return_reference = body.expressions[3].clone();
    let mut return_clone = body.expressions[4].clone();
    let survivor = zryna_source::UntrustedSpan { file: 0, start: start + 7, end: start + 15 };
    let survivor_reference = RawExpressionSyntax {
        span: survivor,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: survivor },
        },
    };
    let literal = zryna_source::UntrustedSpan { file: 0, start: start + 17, end: start + 20 };
    let literal_expression = RawExpressionSyntax {
        span: literal,
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
    };
    let concat_expression = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 21 },
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
            arguments: vec![1, 2],
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 20,
                end: start + 21,
            },
        },
    };
    let mut identity_expression = body.expressions[2].clone();
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity_expression.kind
    else {
        panic!("identity call")
    };
    *arguments = vec![3];
    let zryna_syntax::v4::RawExpressionKind::Clone { value, .. } = &mut return_clone.kind else {
        panic!("return clone")
    };
    *value = 5;
    body.expressions = vec![
        survivor_literal,
        survivor_reference,
        literal_expression,
        concat_expression,
        identity_expression,
        return_reference,
        return_clone,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("value declaration")
    };
    *initializer = 4;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
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

#[allow(clippy::too_many_lines)]
fn private_vec_call_fixture(element: &str) -> (String, RawProjectSyntaxSnapshot) {
    fn take(source: &str, cursor: &mut usize, spelling: &str) -> zryna_source::UntrustedSpan {
        let relative = source[*cursor..].find(spelling).expect("fixture token");
        let start = *cursor + relative;
        let end = start + spelling.len();
        *cursor = end;
        zryna_source::UntrustedSpan {
            file: 0,
            start: u32::try_from(start).expect("span"),
            end: u32::try_from(end).expect("span"),
        }
    }
    fn vec_type(
        source: &str,
        cursor: &mut usize,
        types: &mut Vec<RawTypeSyntax>,
        element: &str,
    ) -> u32 {
        let keyword_span = take(source, cursor, "Vec");
        let less_than_span = take(source, cursor, "<");
        let element_span = take(source, cursor, element);
        let element_id = u32::try_from(types.len()).expect("type id");
        let kind = if element == "String" {
            RawTypeSyntaxKind::String { keyword_span: element_span }
        } else {
            RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: element.to_owned(), span: element_span },
            }
        };
        types.push(RawTypeSyntax { span: element_span, kind });
        let greater_than_span = take(source, cursor, ">");
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: keyword_span.start,
                end: greater_than_span.end,
            },
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element_id,
                greater_than_span,
            },
        });
        id
    }
    fn joined(start: u32, end: u32) -> zryna_source::UntrustedSpan {
        zryna_source::UntrustedSpan { file: 0, start, end }
    }

    let source = format!(
        "function caller(): Vec<{element}> {{ const survivor: Vec<{element}> = Vec<{element}>([]); const result: Vec<{element}> = identity(producer()); return result; }} function identity(value: Vec<{element}>): Vec<{element}> {{ return value; }} function producer(): Vec<{element}> {{ return Vec<{element}>([]); }}"
    );
    let mut cursor = 0;
    let mut types = Vec::new();
    let mut functions = Vec::new();

    let caller_keyword = take(&source, &mut cursor, "function");
    let caller_name = take(&source, &mut cursor, "caller");
    let caller_result = vec_type(&source, &mut cursor, &mut types, element);
    let caller_open = take(&source, &mut cursor, "{");
    let survivor_keyword = take(&source, &mut cursor, "const");
    let survivor_name = take(&source, &mut cursor, "survivor");
    let survivor_type = vec_type(&source, &mut cursor, &mut types, element);
    let survivor_equals = take(&source, &mut cursor, "=");
    let survivor_construct_type = vec_type(&source, &mut cursor, &mut types, element);
    let survivor_construct_open = take(&source, &mut cursor, "(");
    let survivor_bracket_open = take(&source, &mut cursor, "[");
    let survivor_bracket_close = take(&source, &mut cursor, "]");
    let survivor_construct_close = take(&source, &mut cursor, ")");
    let survivor_semi = take(&source, &mut cursor, ";");
    let result_keyword = take(&source, &mut cursor, "const");
    let result_name = take(&source, &mut cursor, "result");
    let result_type = vec_type(&source, &mut cursor, &mut types, element);
    let result_equals = take(&source, &mut cursor, "=");
    let identity_call_name = take(&source, &mut cursor, "identity");
    let identity_call_open = take(&source, &mut cursor, "(");
    let producer_call_name = take(&source, &mut cursor, "producer");
    let producer_call_open = take(&source, &mut cursor, "(");
    let producer_call_close = take(&source, &mut cursor, ")");
    let identity_call_close = take(&source, &mut cursor, ")");
    let result_semi = take(&source, &mut cursor, ";");
    let caller_return_keyword = take(&source, &mut cursor, "return");
    let caller_return_name = take(&source, &mut cursor, "result");
    let caller_return_semi = take(&source, &mut cursor, ";");
    let caller_close = take(&source, &mut cursor, "}");
    let caller_body = joined(caller_open.start, caller_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(caller_keyword.start, caller_close.end),
        export_span: None,
        function_span: caller_keyword,
        name: RawIdentifierSyntax { text: "caller".to_owned(), span: caller_name },
        parameters: Vec::new(),
        result_type: caller_result,
        body: RawFunctionBodySyntax {
            span: caller_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: caller_body,
                open_brace_span: caller_open,
                statements: vec![0, 1, 2],
                close_brace_span: caller_close,
            }],
            statements: vec![
                RawStatementSyntax {
                    span: joined(survivor_keyword.start, survivor_semi.end),
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: survivor_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax {
                            text: "survivor".to_owned(),
                            span: survivor_name,
                        },
                        type_syntax: survivor_type,
                        equals_span: survivor_equals,
                        initializer: 0,
                        semicolon_span: survivor_semi,
                    },
                },
                RawStatementSyntax {
                    span: joined(result_keyword.start, result_semi.end),
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: result_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax { text: "result".to_owned(), span: result_name },
                        type_syntax: result_type,
                        equals_span: result_equals,
                        initializer: 2,
                        semicolon_span: result_semi,
                    },
                },
                RawStatementSyntax {
                    span: joined(caller_return_keyword.start, caller_return_semi.end),
                    kind: RawStatementKind::Return {
                        keyword_span: caller_return_keyword,
                        value: 3,
                        semicolon_span: caller_return_semi,
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: joined(
                        survivor_construct_type_span(&types, survivor_construct_type).start,
                        survivor_construct_close.end,
                    ),
                    kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
                        type_syntax: survivor_construct_type,
                        open_paren_span: survivor_construct_open,
                        open_bracket_span: survivor_bracket_open,
                        elements: Vec::new(),
                        close_bracket_span: survivor_bracket_close,
                        close_paren_span: survivor_construct_close,
                    },
                },
                RawExpressionSyntax {
                    span: joined(producer_call_name.start, producer_call_close.end),
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "producer".to_owned(),
                            span: producer_call_name,
                        },
                        open_paren_span: producer_call_open,
                        arguments: Vec::new(),
                        close_paren_span: producer_call_close,
                    },
                },
                RawExpressionSyntax {
                    span: joined(identity_call_name.start, identity_call_close.end),
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "identity".to_owned(),
                            span: identity_call_name,
                        },
                        open_paren_span: identity_call_open,
                        arguments: vec![1],
                        close_paren_span: identity_call_close,
                    },
                },
                RawExpressionSyntax {
                    span: caller_return_name,
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "result".to_owned(),
                            span: caller_return_name,
                        },
                    },
                },
            ],
        },
    });

    let identity_keyword = take(&source, &mut cursor, "function");
    let identity_name = take(&source, &mut cursor, "identity");
    let parameter_start = take(&source, &mut cursor, "value");
    let parameter_type = vec_type(&source, &mut cursor, &mut types, element);
    let identity_result = vec_type(&source, &mut cursor, &mut types, element);
    let identity_open = take(&source, &mut cursor, "{");
    let identity_return_keyword = take(&source, &mut cursor, "return");
    let identity_return_name = take(&source, &mut cursor, "value");
    let identity_return_semi = take(&source, &mut cursor, ";");
    let identity_close = take(&source, &mut cursor, "}");
    let identity_body = joined(identity_open.start, identity_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(identity_keyword.start, identity_close.end),
        export_span: None,
        function_span: identity_keyword,
        name: RawIdentifierSyntax { text: "identity".to_owned(), span: identity_name },
        parameters: vec![RawParameterSyntax {
            span: joined(
                parameter_start.start,
                types[usize::try_from(parameter_type).expect("type")].span.end,
            ),
            name: RawIdentifierSyntax { text: "value".to_owned(), span: parameter_start },
            type_syntax: parameter_type,
        }],
        result_type: identity_result,
        body: RawFunctionBodySyntax {
            span: identity_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: identity_body,
                open_brace_span: identity_open,
                statements: vec![0],
                close_brace_span: identity_close,
            }],
            statements: vec![RawStatementSyntax {
                span: joined(identity_return_keyword.start, identity_return_semi.end),
                kind: RawStatementKind::Return {
                    keyword_span: identity_return_keyword,
                    value: 0,
                    semicolon_span: identity_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: identity_return_name,
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "value".to_owned(),
                        span: identity_return_name,
                    },
                },
            }],
        },
    });

    let producer_keyword = take(&source, &mut cursor, "function");
    let producer_name = take(&source, &mut cursor, "producer");
    let producer_result = vec_type(&source, &mut cursor, &mut types, element);
    let producer_open = take(&source, &mut cursor, "{");
    let producer_return_keyword = take(&source, &mut cursor, "return");
    let producer_construct_type = vec_type(&source, &mut cursor, &mut types, element);
    let producer_construct_open = take(&source, &mut cursor, "(");
    let producer_bracket_open = take(&source, &mut cursor, "[");
    let producer_bracket_close = take(&source, &mut cursor, "]");
    let producer_construct_close = take(&source, &mut cursor, ")");
    let producer_return_semi = take(&source, &mut cursor, ";");
    let producer_close = take(&source, &mut cursor, "}");
    let producer_body = joined(producer_open.start, producer_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(producer_keyword.start, producer_close.end),
        export_span: None,
        function_span: producer_keyword,
        name: RawIdentifierSyntax { text: "producer".to_owned(), span: producer_name },
        parameters: Vec::new(),
        result_type: producer_result,
        body: RawFunctionBodySyntax {
            span: producer_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: producer_body,
                open_brace_span: producer_open,
                statements: vec![0],
                close_brace_span: producer_close,
            }],
            statements: vec![RawStatementSyntax {
                span: joined(producer_return_keyword.start, producer_return_semi.end),
                kind: RawStatementKind::Return {
                    keyword_span: producer_return_keyword,
                    value: 0,
                    semicolon_span: producer_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: joined(
                    types[usize::try_from(producer_construct_type).expect("type")].span.start,
                    producer_construct_close.end,
                ),
                kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
                    type_syntax: producer_construct_type,
                    open_paren_span: producer_construct_open,
                    open_bracket_span: producer_bracket_open,
                    elements: Vec::new(),
                    close_bracket_span: producer_bracket_close,
                    close_paren_span: producer_construct_close,
                },
            }],
        },
    });

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
                functions,
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn private_vec_nested_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = private_vec_call_fixture("String");
    let old = "producer()";
    let replacement = "Vec<String>([concat(\"a\", \"b\")])";
    let start = u32::try_from(source.find(old).expect("producer call")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let survivor_construct = raw.files[0].functions[0].body.expressions[0].clone();
    let mut identity = raw.files[0].functions[0].body.expressions[2].clone();
    let return_reference = raw.files[0].functions[0].body.expressions[3].clone();
    let string_span = zryna_source::UntrustedSpan { file: 0, start: start + 4, end: start + 10 };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: string_span,
        kind: RawTypeSyntaxKind::String { keyword_span: string_span },
    });
    let vec_type = u32::try_from(raw.files[0].type_syntax.len()).expect("type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 11 },
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 3 },
            less_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 3,
                end: start + 4,
            },
            argument: string_type,
            greater_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 10,
                end: start + 11,
            },
        },
    });
    let literal = |offset, spelling: &str| RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: start + offset,
            end: start + offset + 3,
        },
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: spelling.to_owned() },
    };
    let concat = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: start + 13, end: start + 29 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "concat".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 13, end: start + 19 },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 19,
                end: start + 20,
            },
            arguments: vec![1, 2],
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 28,
                end: start + 29,
            },
        },
    };
    let construct = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 31 },
        kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
            type_syntax: vec_type,
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 11,
                end: start + 12,
            },
            open_bracket_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 12,
                end: start + 13,
            },
            elements: vec![3],
            close_bracket_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 29,
                end: start + 30,
            },
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 30,
                end: start + 31,
            },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity.kind else {
        panic!("identity call")
    };
    *arguments = vec![4];
    let body = &mut raw.files[0].functions[0].body;
    body.expressions = vec![
        survivor_construct,
        literal(20, "\"a\""),
        literal(25, "\"b\""),
        concat,
        construct,
        identity,
        return_reference,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("result declaration")
    };
    *initializer = 5;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
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
    let mut retained_roots = instruction.place_operands().collect::<Vec<_>>();
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
fn pair_oracle_lowers_to_sealed_copy_aggregate_ir() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");

    let program = lower(pair_input(&syntax, &sources)).expect("Pair must lower and verify");

    assert_eq!(program.modules().len(), 1);
    assert_eq!(
        program.runtime_abi().type_universe_identity(),
        program.verified_ir().type_universe_identity()
    );
    assert_eq!(
        program.runtime_abi().linear32_fingerprint(),
        *program.verified_ir().linear32_layouts().fingerprint()
    );
    assert_eq!(
        program.runtime_abi().linux_x86_64_fingerprint(),
        *program.verified_ir().linux_x86_64_layouts().fingerprint()
    );
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    assert!(places.iter().any(|place| matches!(place.kind(), VerifiedPlaceKind::Parameter(0))));
    let kinds = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.iter().any(|kind| matches!(kind, VerifiedInstructionKind::StructConstruct)));
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
fn owned_projection_repeat_and_whole_root_after_partial_move_are_m3014() {
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

    let (root_source, root_raw) = owned_pair_partial_then_root_snapshot();
    let root_sources = sources_for(&root_source);
    let root_syntax = verify_snapshot(root_raw, &root_sources)
        .expect("source-faithful whole root after projected move");
    let root = lower(pair_input(&root_syntax, &root_sources))
        .expect_err("whole root after projected move");
    assert_eq!(root[0].code(), "ZRYNA-M3014");
    assert_eq!(
        root[0].primary_span(),
        Some(span(&root_sources, nth_untrusted_span(&root_source, "p", 2))),
    );
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

#[derive(Clone, Debug)]
enum OracleValue {
    I32(i32),
    Aggregate(Vec<OracleValue>),
}

#[allow(clippy::too_many_lines)]
fn evaluate_pair(function: VerifiedFunction<'_>, arguments: [i32; 2]) -> i32 {
    let places = function.places().collect::<Vec<_>>();
    let mut values = vec![None; 64];
    for (index, argument) in arguments.into_iter().enumerate() {
        values[index] = Some(OracleValue::I32(argument));
    }
    let mut roots = vec![None; places.len()];
    for place in &places {
        if let VerifiedPlaceKind::Parameter(index) = place.kind() {
            roots[usize::try_from(place.id().index()).expect("place index")] =
                values[usize::try_from(index).expect("parameter index")].clone();
        }
    }
    let resolve_place = |place_index: u32,
                         roots: &[Option<OracleValue>],
                         values: &[Option<OracleValue>]| {
        let mut path = Vec::new();
        let mut current = place_index;
        loop {
            let place = places[usize::try_from(current).expect("place index")];
            match place.kind() {
                VerifiedPlaceKind::Parameter(_) | VerifiedPlaceKind::Local(_) => {
                    let mut value = roots[usize::try_from(current).expect("root index")]
                        .clone()
                        .expect("initialized root");
                    for ordinal in path.into_iter().rev() {
                        let OracleValue::Aggregate(fields) = value else {
                            panic!("aggregate projection")
                        };
                        value = fields[usize::try_from(ordinal).expect("field ordinal")].clone();
                    }
                    break value;
                }
                VerifiedPlaceKind::Temporary(value) => {
                    let mut result = values[usize::try_from(value.index()).expect("value index")]
                        .clone()
                        .expect("temporary value");
                    for ordinal in path.into_iter().rev() {
                        let OracleValue::Aggregate(fields) = result else {
                            panic!("aggregate projection")
                        };
                        result = fields[usize::try_from(ordinal).expect("field ordinal")].clone();
                    }
                    break result;
                }
                VerifiedPlaceKind::StructField { base, ordinal }
                | VerifiedPlaceKind::FixedArrayConstant { base, index: ordinal } => {
                    path.push(ordinal);
                    current = base.index();
                }
                VerifiedPlaceKind::EnumPayload { .. } => panic!("Pair oracle has no enum payload"),
            }
        }
    };
    let block = function.blocks().next().expect("Pair block");
    for instruction in block.instructions() {
        let operands = instruction
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>();
        let place_operands = instruction
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>();
        let result = match instruction.kind() {
            VerifiedInstructionKind::I32Literal => {
                Some(OracleValue::I32(instruction.i32_literal().expect("literal")))
            }
            VerifiedInstructionKind::StructConstruct => Some(OracleValue::Aggregate(
                operands
                    .iter()
                    .map(|id| {
                        values[usize::try_from(*id).expect("value index")].clone().expect("operand")
                    })
                    .collect(),
            )),
            VerifiedInstructionKind::CopyFromPlace => {
                Some(resolve_place(place_operands[0], &roots, &values))
            }
            VerifiedInstructionKind::InitializePlace => {
                roots[usize::try_from(place_operands[0]).expect("place index")] =
                    values[usize::try_from(operands[0]).expect("value index")].clone();
                None
            }
            VerifiedInstructionKind::I32Mul | VerifiedInstructionKind::I32Add => {
                let OracleValue::I32(lhs) =
                    values[usize::try_from(operands[0]).expect("lhs")].clone().expect("lhs value")
                else {
                    panic!("i32 lhs")
                };
                let OracleValue::I32(rhs) =
                    values[usize::try_from(operands[1]).expect("rhs")].clone().expect("rhs value")
                else {
                    panic!("i32 rhs")
                };
                Some(OracleValue::I32(if instruction.kind() == VerifiedInstructionKind::I32Mul {
                    lhs.wrapping_mul(rhs)
                } else {
                    lhs.wrapping_add(rhs)
                }))
            }
            other => panic!("unexpected Pair oracle instruction {other:?}"),
        };
        if let (Some(id), Some(value)) = (instruction.result(), result) {
            let index = usize::try_from(id.index()).expect("result index");
            if index >= values.len() {
                values.resize(index + 1, None);
            }
            values[index] = Some(value);
        }
    }
    assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return);
    let returned = block.terminator().value_operands().next().expect("return value");
    let OracleValue::I32(value) = values[usize::try_from(returned.index()).expect("return index")]
        .clone()
        .expect("returned value")
    else {
        panic!("scalar return")
    };
    value
}

#[test]
fn normative_pair_score_matches_all_five_frozen_oracle_cases() {
    let sources = sources_for(PAIR_SCORE_SOURCE);
    let raw = decode_snapshot(PAIR_SCORE_JSON).expect("generated Pair score v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Pair score v4");
    let program = lower(pair_input(&syntax, &sources)).expect("Pair score must lower and verify");
    let function = program.modules().next().expect("module").functions().next().expect("pairScore");
    let oracle: serde_json::Value = serde_json::from_str(PAIR_ORACLE).expect("Pair oracle JSON");
    let cases = oracle["cases"].as_array().expect("oracle cases");
    assert_eq!(cases.len(), 5);
    for case in cases {
        let arguments = case["arguments"].as_array().expect("arguments");
        let left = i32::try_from(arguments[0]["value"].as_i64().expect("left")).expect("left i32");
        let right =
            i32::try_from(arguments[1]["value"].as_i64().expect("right")).expect("right i32");
        let expected = i32::try_from(case["expected"]["value"].as_i64().expect("expected"))
            .expect("expected i32");
        assert_eq!(evaluate_pair(function, [left, right]), expected, "{}", case["id"]);
    }
    let kinds = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(!kinds.iter().any(|kind| matches!(
        kind,
        VerifiedInstructionKind::MoveFromPlace
            | VerifiedInstructionKind::DropPlace
            | VerifiedInstructionKind::ClonePlace
    )));
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
fn authenticated_input_rejects_another_source_authority() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");
    let other = pair_sources();
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = other.file_id(&path).expect("entry");
    assert!(SemanticInput::try_new(&syntax, &other, entry).is_none());
}

#[test]
fn private_multibyte_string_literal_has_distinct_prepare_and_return_cleanup() {
    let sources = sources_for(MULTIBYTE_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(MULTIBYTE_STRING_RESPONSE), &sources)
        .expect("source-faithful multibyte String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    assert_eq!(
        program.runtime_abi().type_universe_identity(),
        program.verified_ir().type_universe_identity()
    );
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.public_export().is_none());
    let block = function.blocks().next().expect("block");
    let instruction = block.instructions().next().expect("StringFromUtf8");
    assert_eq!(instruction.kind(), VerifiedInstructionKind::StringFromUtf8);
    assert_eq!(instruction.string_utf8_bytes(), Some("snowman: ☃".as_bytes()));
    assert_eq!(instruction.cleanup().expect("prepare cleanup").index(), 0);
    assert_eq!(instruction.derived_drop_actions().count(), 0);
    let terminator = block.terminator();
    assert_eq!(terminator.kind(), VerifiedTerminatorKind::Return);
    assert_eq!(terminator.cleanup().expect("return cleanup").index(), 1);
    assert_eq!(terminator.derived_drop_actions().count(), 0);
}

#[test]
fn private_string_local_moves_exact_owner_to_return() {
    let sources = sources_for(LOCAL_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(LOCAL_STRING_RESPONSE), &sources)
        .expect("source-faithful local String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String local");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        instructions,
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    let places =
        function.places().map(zryna_ir::data_ownership_v1::VerifiedPlace::kind).collect::<Vec<_>>();
    assert!(matches!(places[0], VerifiedPlaceKind::Temporary(value) if value.index() == 0));
    assert_eq!(places[1], VerifiedPlaceKind::Local(0));
    assert!(matches!(places[2], VerifiedPlaceKind::Temporary(value) if value.index() == 1));
    let block = function.blocks().next().expect("block");
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_string_return_cleanup_drops_remaining_locals_in_reverse_order() {
    let sources = sources_for(THREE_LOCAL_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(THREE_LOCAL_STRING_RESPONSE), &sources)
        .expect("source-faithful three-local String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("three private String locals");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let prepare_roots = block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .map(|instruction| {
            instruction
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(prepare_roots, [vec![], vec![1], vec![3, 1]]);
    let roots = block
        .terminator()
        .derived_drop_actions()
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(roots, [3, 1]);
}

#[test]
fn private_string_use_after_move_is_rejected_deterministically() {
    let sources = sources_for(USE_AFTER_MOVE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(USE_AFTER_MOVE_RESPONSE), &sources)
        .expect("source-faithful moved String v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("use after move");
    let second = lower(pair_input(&syntax, &sources)).expect_err("same use after move");
    let summary = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code().to_owned(), diagnostic.message().to_owned()))
            .collect::<Vec<_>>()
    };
    let diagnostic = first
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("semantic use-after-move diagnostic");
    let at = diagnostic.primary_span().expect("reference span");
    assert_eq!((at.start(), at.end()), (89, 94));
    assert_eq!(summary(&first), summary(&second));
}

#[test]
fn private_string_clone_retains_source_at_prepare_and_return() {
    let sources = sources_for(STRING_CLONE_SOURCE);
    let raw = response_snapshot(STRING_CLONE_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 2);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String clone v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("StringClone");
    let source = clone.place_operands().next().expect("clone source");
    assert_eq!(source.index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
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
fn private_string_concat_retains_both_sources_in_reverse_cleanup_order() {
    let sources = sources_for(STRING_CONCAT_SOURCE);
    let raw = response_snapshot(STRING_CONCAT_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 3);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String concat call v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let concat = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringConcat)
        .expect("StringConcat");
    assert_eq!(
        concat
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    let expected = [3, 1];
    assert_eq!(
        concat.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn private_string_concat_full_single_block_shape_is_stable() {
    let sources = sources_for(STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful String concat v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id().index(), 0);
    let instructions = blocks[0].instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringConcat,
        ]
    );
    assert_eq!(
        instructions
            .iter()
            .filter_map(|instruction| {
                instruction.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            })
            .collect::<Vec<_>>(),
        [0, 1, 2]
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
            (1, VerifiedCleanupRole::PrepareFailure, vec![1]),
            (2, VerifiedCleanupRole::PrepareFailure, vec![3, 1]),
            (3, VerifiedCleanupRole::Return, vec![3, 1]),
        ]
    );
    let terminator = blocks[0].terminator();
    let return_start =
        u32::try_from(STRING_CONCAT_SOURCE.find("return").expect("return")).expect("return offset");
    assert_eq!(terminator.span().start(), return_start);
    assert_eq!(terminator.span().end(), return_start + 27);
    assert_eq!(terminator.value_operands().next().expect("return value").index(), 2);
    assert_eq!(terminator.cleanup().expect("return cleanup").index(), 3);
}

#[test]
fn private_string_clone_rejects_a_moved_source_at_its_reference() {
    let sources = sources_for(MOVED_STRING_CLONE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(MOVED_STRING_CLONE_RESPONSE), &sources)
        .expect("source-faithful moved String clone v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("clone after move");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved source diagnostic");
    let at = diagnostic.primary_span().expect("source reference");
    assert_eq!((at.start(), at.end()), (96, 102));
}

#[test]
fn private_string_concat_requires_exact_builtin_arity() {
    let sources = sources_for(BAD_STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(BAD_STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful malformed concat call v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("concat arity");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3012")
        .expect("String concat diagnostic");
    let at = diagnostic.primary_span().expect("concat callee");
    assert_eq!((at.start(), at.end()), (60, 66));
}

#[test]
fn private_string_root_assignment_prepares_then_replaces_exact_owner() {
    for rhs in [
        StringAssignmentRhs::Move,
        StringAssignmentRhs::Literal,
        StringAssignmentRhs::Clone,
        StringAssignmentRhs::Concat,
    ] {
        let (source, raw) = string_assignment_snapshot(rhs);
        let sources = sources_for(source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful String assignment v4");
        let program = lower(pair_input(&syntax, &sources)).expect("private String assignment");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let replace = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
            .expect("ReplacePlace");
        let actions = replace.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].root().index(), 1);
        assert_eq!(actions[0].moved_projections().count(), 0);
        assert_eq!(actions[0].initialized_projections().count(), 0);
        assert_eq!(actions[0].active_variant(), None);
        let prepare = block
            .instructions()
            .filter(|instruction| {
                matches!(
                    instruction.kind(),
                    VerifiedInstructionKind::StringFromUtf8
                        | VerifiedInstructionKind::StringClone
                        | VerifiedInstructionKind::StringConcat
                ) && instruction.derived_drop_actions().any(|action| action.root().index() == 1)
            })
            .last();
        if matches!(
            rhs,
            StringAssignmentRhs::Literal | StringAssignmentRhs::Clone | StringAssignmentRhs::Concat
        ) {
            assert!(prepare.is_some(), "fallible RHS retains the old destination");
        }
        let return_roots = block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>();
        if matches!(rhs, StringAssignmentRhs::Move) {
            assert!(return_roots.is_empty());
        } else {
            assert_eq!(return_roots, [3]);
        }
    }
}

#[test]
fn private_string_self_assignment_move_is_narrow_and_deterministic() {
    let (source, raw) = string_assignment_snapshot(StringAssignmentRhs::SelfMove);
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful self assignment v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("self move assignment");
    let second = lower(pair_input(&syntax, &sources)).expect_err("same self move assignment");
    let summarize = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_owned(),
                    diagnostic.primary_span().map(|span| (span.start(), span.end())),
                    diagnostic.message().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(summarize(&first), summarize(&second));
    assert_eq!(first[0].code(), "ZRYNA-M3014");
    let at = first[0].primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (80, 81));
}

#[test]
fn private_string_assignment_rejects_call_based_target_consumption_before_rhs() {
    for rhs in [StringAssignmentRhs::CallSelf, StringAssignmentRhs::CloneCallSelf] {
        let (source, raw) = string_assignment_snapshot(rhs);
        let sources = sources_for(source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming assignment");
        let diagnostics =
            lower(pair_input(&syntax, &sources)).expect_err("target move must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
        let expected = nth_untrusted_span(source, "x", 2);
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, expected)));
    }
}

#[test]
fn private_string_assignment_rejects_an_immutable_target() {
    let source = STRING_ASSIGN_MOVE_SOURCE.replacen("let x", "const x", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(STRING_ASSIGN_MOVE_RESPONSE), 32, 2);
    let RawStatementKind::LocalDeclaration {
        keyword_span,
        name,
        type_syntax,
        equals_span,
        initializer,
        semicolon_span,
        ..
    } = raw.files[0].functions[0].body.statements[0].kind.clone()
    else {
        panic!("first local")
    };
    let mut keyword_span = keyword_span;
    keyword_span.end = 33;
    raw.files[0].functions[0].body.statements[0].kind = RawStatementKind::LocalDeclaration {
        keyword_span,
        mutable: false,
        name,
        type_syntax,
        equals_span,
        initializer,
        semicolon_span,
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable assignment");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable assignment");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3014")
        .expect("immutable target diagnostic");
    let at = diagnostic.primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (78, 79));
}

#[test]
fn private_string_assignment_rejects_a_moved_target() {
    let source = STRING_ASSIGN_MOVE_SOURCE.replacen("\"new\"", "x", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(STRING_ASSIGN_MOVE_RESPONSE), 74, -4);
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved assignment target");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3014")
        .expect("moved target diagnostic");
    let at = diagnostic.primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (72, 73));
}

#[test]
fn private_string_assignment_rejects_a_moved_source() {
    let source = STRING_ASSIGN_MOVE_SOURCE
        .replacen("const y", "let   y", 1)
        .replacen("\"new\"", "x    ", 1)
        .replacen("x = y", "y = x", 1);
    let sources = sources_for(&source);
    let mut raw = response_snapshot(STRING_ASSIGN_MOVE_RESPONSE);
    let RawStatementKind::LocalDeclaration { mutable, keyword_span, .. } =
        &mut raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("second local")
    };
    *mutable = true;
    keyword_span.end = 54;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    for (id, text) in [(2, "y"), (3, "x")] {
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut raw.files[0].functions[0].body.expressions[id].kind
        else {
            panic!("assignment reference")
        };
        name.text = text.to_owned();
    }
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved assignment source");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved source diagnostic");
    let at = diagnostic.primary_span().expect("source span");
    assert_eq!((at.start(), at.end()), (80, 81));
}

#[test]
fn static_string_concat_size_is_checked_at_exact_runtime_limit_and_overflow() {
    let max = zryna_ownership_runtime_abi::MAX_STRING_BYTES;
    assert_eq!(checked_string_concat_bytes(max - 1, 1), Some(max));
    assert_eq!(checked_string_concat_bytes(max, 1), None);
    assert_eq!(checked_string_concat_bytes(u64::MAX, 1), None);
}

#[test]
fn private_string_cleanup_action_budget_is_exact_and_checked_for_overflow() {
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!cleanup_action_budget_violation(maximum - 1, 1, false));
    assert!(cleanup_action_budget_violation(maximum, 1, false));
    assert!(!cleanup_action_budget_violation(maximum, 1, true));
    assert!(cleanup_action_budget_violation(usize::MAX, 1, false));
}

#[test]
fn private_string_cleanup_action_overflow_is_source_located_m3201() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let mut errors = Errors::new(&sources);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState {
            pending: vec![raw::PlaceId(0)],
            value_owners: std::collections::BTreeMap::new(),
        },
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    assert!(lowerer.push_cleanup(at, None).is_none());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    let primary = diagnostics[0].primary_span().expect("cleanup site");
    assert_eq!((primary.start(), primary.end()), (32, 35));
}

#[test]
fn private_string_transition_limit_fails_before_external_lowerer_state_mutates() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let mut errors = Errors::new(&sources);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
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
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    assert!(lowerer.value(0).is_none());
    assert_eq!(lowerer.next_value, 0);
    assert!(lowerer.places.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    assert!(lowerer.owners.pending().is_empty());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
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
) -> PrivateStringLowerer<'a, 'e> {
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
fn private_vec_string_constructor_consumes_elements_after_failure_cleanup() {
    let sources = sources_for(VEC_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_STRING_RESPONSE), &sources)
        .expect("source-faithful Vec<String> v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String>");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let construct = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .expect("VecConstruct");
    assert_eq!(
        construct.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next().is_some_and(|place| place.index() == 1)
    }));
}

#[test]
fn private_vec_i32_clone_preserves_source_and_returns_distinct_owner() {
    let (source, raw) = private_vec_clone_fixture("i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<i32> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<i32> clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone");
    let source_place = clone.place_operands().next().expect("clone source");
    let result = clone.result().expect("clone result");
    assert_eq!(source_place.index(), 1);
    assert_eq!(result.index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        block.terminator().value_operands().next().expect("returned clone").index(),
        result.index()
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1]
    );
    let abi = program.runtime_abi();
    assert_all_runtime_faults(
        abi,
        function,
        clone,
        LogicalOperation::VecAllocate,
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
    let fault = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("authenticated VecClone allocation failure");
    assert!(!fault.result_committed);
    assert_eq!(fault.uncommitted_result.expect("uncommitted clone result").index(), result.index());
    assert_eq!(fault.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
    assert_eq!(fault.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
    let replay_clone = replay
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("replayed VecClone");
    assert_eq!(
        (
            replay_clone.place_operands().next().expect("source").index(),
            replay_clone.result().expect("result").index(),
            replay_clone
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
        ),
        (source_place.index(), result.index(), vec![1])
    );
}

#[test]
fn private_vec_bool_clone_uses_the_same_copy_only_contract() {
    let (source, raw) = private_vec_clone_fixture("bool");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<bool> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<bool> clone");
    let clone = program
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone<bool>");
    assert_eq!(clone.place_operands().next().expect("source").index(), 1);
    assert_eq!(clone.result().expect("result").index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_string_clone_seals_allocation_and_prefix_failures() {
    let (source, raw) = private_vec_clone_fixture("String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone<String>");
    assert_eq!(clone.place_operands().next().expect("source").index(), 4);
    assert_eq!(clone.result().expect("result").index(), 4);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [4]
    );
    let element_actions = clone.vec_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        element_actions
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::kind)
            .collect::<Vec<_>>(),
        [VerifiedDropActionKind::VecInitializedPrefix, VerifiedDropActionKind::Place]
    );
    assert_eq!(
        element_actions.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
        [5, 4]
    );

    let allocation = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("allocation phase");
    assert_eq!(
        allocation.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4]
    );
    assert_eq!(
        allocation.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4]
    );
    assert!(allocation.prefix_owner.is_none());

    for (status, expected) in [
        (
            RuntimeStatus::Allocation,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
        ),
        (
            RuntimeStatus::Capacity,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
        ),
        (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
    ] {
        for completed_prefix in [0, 1, 2] {
            let injection =
                OwnedFaultInjection::VecCloneElement { status, source_length: 3, completed_prefix };
            let event_limit = usize::try_from(completed_prefix).expect("small prefix") + 1;
            let first = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
                .expect("element clone failure");
            let second = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
                .expect("deterministic element failure");
            assert_eq!(first, second);
            assert_eq!(first.disposition, expected);
            assert_eq!(
                first.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
                [4]
            );
            assert_eq!(
                first.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
                [4]
            );
            assert_eq!(first.prefix_owner.expect("prefix owner").index(), 5);
            assert_eq!(first.reverse_prefix, (0..completed_prefix).rev().collect::<Vec<_>>());
        }
    }
    let middle = OwnedFaultInjection::VecCloneElement {
        status: RuntimeStatus::Allocation,
        source_length: 3,
        completed_prefix: 2,
    };
    assert_eq!(
        owned_fault_trace(abi, function, clone, middle, 0, 2),
        Err(OwnedFaultOracleError::EventLimit)
    );
    assert_eq!(
        owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::VecCloneElement {
                status: RuntimeStatus::Allocation,
                source_length: MAX_VEC_ELEMENTS,
                completed_prefix: u64::MAX,
            },
            usize::MAX,
            usize::MAX,
        ),
        Err(OwnedFaultOracleError::InvalidVecClonePrefix)
    );
    for (source_length, completed_prefix) in [(3, 3), (MAX_VEC_ELEMENTS + 1, 0), (0, 0)] {
        assert_eq!(
            owned_fault_trace(
                abi,
                function,
                clone,
                OwnedFaultInjection::VecCloneElement {
                    status: RuntimeStatus::Allocation,
                    source_length,
                    completed_prefix,
                },
                0,
                usize::MAX,
            ),
            Err(OwnedFaultOracleError::InvalidVecClonePrefix)
        );
    }

    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic Vec<String> replay");
    let replay_clone = replay
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("replayed clone");
    assert_eq!(
        replay_clone
            .vec_clone_element_failure_drop_actions()
            .map(|action| (action.kind(), action.root().index()))
            .collect::<Vec<_>>(),
        element_actions
            .iter()
            .map(|action| (action.kind(), action.root().index()))
            .collect::<Vec<_>>()
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_clone_preflights_exact_first_extra_and_overflow_atomically() {
    let (source, raw) = private_vec_clone_fixture("i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec clone");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("Vec<i32>");
    let element = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
        .copied()
        .expect("i32");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.body.expressions[2].span);

    for mode in 0..7 {
        let mut errors = Errors::new(&sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let value_base =
            zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - usize::from(mode != 1);
        cfg.value_types.resize(value_base, vec_ty.ir);
        cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            - usize::from(mode != 3);
        let place_base =
            zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - usize::from(mode != 2);
        let places = (0..place_base)
            .map(|index| raw::Place {
                id: raw::PlaceId(u32::try_from(index).expect("place id")),
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(u32::try_from(index).expect("local id")),
            })
            .collect::<Vec<_>>();
        let source_place = raw::PlaceId(0);
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::Binding { ty: vec_ty, place: source_place, mutable: false },
            )]),
            places,
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: if mode == 4 {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            } else {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1
            },
            reserved_cleanup_actions: if mode == 5 {
                usize::MAX
            } else if mode == 6 {
                zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
            } else {
                zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1
            },
            owners: OwnerState {
                pending: vec![source_place],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: u32::try_from(value_base).expect("value id"),
            next_local: u32::try_from(place_base).expect("local id"),
        };
        let before = (
            lowerer.places.len(),
            lowerer.cfg.value_types.len(),
            lowerer.cfg.transitions,
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.cleanup_plans.clone(),
        );
        let result = lowerer.clone_vec(1, vec_ty, at);
        if mode == 0 {
            assert!(result.is_some(), "exact compound reservation");
            assert!(lowerer.owners.contains(source_place), "clone preserves source");
        } else {
            assert!(result.is_none(), "first extra or overflow must fail");
            assert_eq!(
                (
                    lowerer.places.len(),
                    lowerer.cfg.value_types.len(),
                    lowerer.cfg.transitions,
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.cleanup_plans.clone(),
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if mode == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }

    for case in 0..3 {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let source_place = raw::PlaceId(0);
        let binding_ty = if case == 1 { element } else { vec_ty };
        let owners = if case == 0 {
            OwnerState::default()
        } else {
            OwnerState {
                pending: vec![source_place],
                value_owners: std::collections::BTreeMap::new(),
            }
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::Binding { ty: binding_ty, place: source_place, mutable: false },
            )]),
            places: vec![raw::Place {
                id: source_place,
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            }],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners,
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let operand = u32::from(case != 2);
        assert!(lowerer.clone_vec(operand, vec_ty, at).is_none(), "negative case {case}");
        assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
        assert_eq!(lowerer.places.len(), 1);
        assert!(lowerer.cleanup_plans.is_empty());
        drop(lowerer);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), if case == 0 { "ZRYNA-M3014" } else { "ZRYNA-M3013" });
        let expected = if case == 2 {
            span(&sources, function.body.expressions[0].span)
        } else {
            span(&sources, function.body.expressions[1].span)
        };
        assert_eq!(diagnostics[0].primary_span(), Some(expected));
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_string_clone_prefix_cleanup_is_exact_plus_one_and_overflow_atomic() {
    let (source, raw) = private_vec_clone_fixture("String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> clone");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("Vec<String>");
    let element = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::String)
        .copied()
        .expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.body.expressions[5].span);

    for mode in 0..4 {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let source_place = raw::PlaceId(0);
        let owners = OwnerState {
            pending: vec![source_place],
            value_owners: std::collections::BTreeMap::new(),
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::Binding { ty: vec_ty, place: source_place, mutable: false },
            )]),
            places: vec![raw::Place {
                id: source_place,
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            }],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: if mode == 1 {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1
            } else {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 2
            },
            reserved_cleanup_actions: match mode {
                2 => zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
                3 => usize::MAX,
                _ => zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 3,
            },
            owners,
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let before = (
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
        );
        let result = lowerer.clone_vec(4, vec_ty, at);
        if mode == 0 {
            assert!(result.is_some(), "exact two-phase cleanup budget");
            assert_eq!(lowerer.cleanup_plans.len(), 2);
            assert_eq!(lowerer.cleanup_actions, 3);
        } else {
            assert!(result.is_none(), "first extra or overflow must fail");
            assert_eq!(
                (
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if mode == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}

#[test]
fn private_vec_string_root_assignment_prepares_then_replaces_exact_owner() {
    let sources = sources_for(VEC_ASSIGN_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_ASSIGN_STRING_RESPONSE), &sources)
        .expect("source-faithful Vec<String> assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let constructs = block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .collect::<Vec<_>>();
    assert_eq!(constructs.len(), 2);
    assert_eq!(
        constructs[1]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [3, 2]
    );
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_i32_move_assignment_replaces_old_owner_and_preserves_stack_slot() {
    let sources = sources_for(VEC_ASSIGN_I32_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_ASSIGN_I32_RESPONSE), &sources)
        .expect("source-faithful Vec<i32> move assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<i32> assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_assignment_rejects_call_based_target_consumption_before_rhs() {
    let source = VEC_ASSIGN_I32_SOURCE.replacen("x = y", "x = identity(x)", 1);
    let mut raw = shift_snapshot(response_snapshot(VEC_ASSIGN_I32_RESPONSE), 98, 10);
    let body = &mut raw.files[0].functions[0].body;
    body.statements[2].span.end += 10;
    let RawStatementKind::Assignment { value, semicolon_span, .. } = &mut body.statements[2].kind
    else {
        unreachable!("Vec assignment")
    };
    semicolon_span.start += 10;
    semicolon_span.end += 10;
    body.expressions[3] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 104, end: 105 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 104, end: 105 },
            },
        },
    };
    *value = u32::try_from(body.expressions.len()).expect("call id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 95, end: 106 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "identity".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 95, end: 103 },
            },
            open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 103, end: 104 },
            arguments: vec![3],
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 105, end: 106 },
        },
    });
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming Vec assignment");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("target move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "x", 2)))
    );
}

#[test]
fn private_string_producer_and_identity_calls_transfer_exact_owners() {
    let (source, raw) = private_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private String calls");
    let program = lower(pair_input(&syntax, &sources)).expect("String producer and identity calls");
    let module = program.modules().next().expect("module");
    let functions = module.functions().collect::<Vec<_>>();
    let caller = &functions[0];
    let identity = &functions[1];
    let producer = &functions[2];
    let caller_block = caller.blocks().next().expect("caller block");
    let calls = caller_block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].callee().expect("producer").declaration(), 2);
    assert_eq!(calls[0].call_arguments().count(), 0);
    assert_eq!(calls[1].callee().expect("identity").declaration(), 1);
    assert_eq!(calls[1].call_arguments().count(), 1);
    for call in &calls {
        assert_eq!(
            call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [1],
            "post-transfer CallTrap retains only the pre-existing survivor"
        );
        assert_eq!(
            caller
                .cleanup_plans()
                .find(|plan| plan.id() == call.cleanup().expect("CallTrap cleanup"))
                .expect("cleanup plan")
                .site()
                .role(),
            VerifiedCleanupRole::CallTrap
        );
    }
    assert!(
        caller_block
            .instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::StringClone })
    );
    assert!(identity.places().any(|place| place.kind() == VerifiedPlaceKind::Parameter(0)));
    assert_eq!(
        identity
            .blocks()
            .next()
            .expect("identity block")
            .terminator()
            .derived_drop_actions()
            .count(),
        0
    );
    assert_eq!(
        producer
            .blocks()
            .next()
            .expect("producer block")
            .terminator()
            .derived_drop_actions()
            .count(),
        0
    );
}

#[test]
fn private_string_direct_call_accepts_nested_concat_argument() {
    let (source, raw) = private_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested String call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested String call");
    let caller =
        program.verified_ir().modules().next().expect("module").functions().next().expect("caller");
    let kinds = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&VerifiedInstructionKind::StringConcat));
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::DirectCall).count(),
        1
    );
}

#[test]
fn private_string_call_rejects_exported_owned_callee_at_call_name() {
    let (source, raw) = private_string_call_fixture();
    let identity_start = raw.files[0].functions[1].span.start;
    let mut raw = shift_snapshot(raw, identity_start, 7);
    let identity = &mut raw.files[0].functions[1];
    identity.span.start = identity_start;
    identity.export_span = Some(zryna_source::UntrustedSpan {
        file: 0,
        start: identity_start,
        end: identity_start + 6,
    });
    let call_span = match &raw.files[0].functions[0].body.expressions[2].kind {
        zryna_syntax::v4::RawExpressionKind::Call { callee, .. } => callee.span,
        _ => panic!("identity call"),
    };
    let source = source.replacen("function identity", "export function identity", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported String callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("exported owned call");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3016")
        .expect("private-call diagnostic");
    let at = diagnostic.primary_span().expect("callee span");
    assert_eq!((at.start(), at.end()), (call_span.start, call_span.end));
}

#[test]
fn private_string_call_rejects_wrong_arity_before_argument_transfer() {
    let (source, raw) = private_string_call_fixture();
    let removed = raw.files[0].functions[0].body.expressions[1].span;
    let mut raw = shift_snapshot_signed(raw, removed.end, -10);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions.remove(1);
    let RawStatementKind::LocalDeclaration { initializer, .. } =
        &mut caller.body.statements[1].kind
    else {
        panic!("value local")
    };
    *initializer = 1;
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[2].kind else {
        panic!("return")
    };
    *value = 3;
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[1].kind
    else {
        panic!("identity call")
    };
    arguments.clear();
    let zryna_syntax::v4::RawExpressionKind::Clone { value, .. } =
        &mut caller.body.expressions[3].kind
    else {
        panic!("clone")
    };
    *value = 2;
    let source = source.replacen("identity(producer())", "identity()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong call arity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong String call arity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"));
}

#[test]
fn private_string_call_rejects_wrong_argument_type_and_missing_name() {
    let (source, mut raw) = private_string_call_fixture();
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: argument_span.start,
            end: argument_span.start + 1,
        },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
    };
    let source = source.replacen("producer()", "1         ", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong String argument");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong owned argument type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"));

    let (source, mut raw) = private_string_call_fixture();
    let source = source.replacen("producer()", "missingx()", 1);
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("producer call")
    };
    callee.text = "missingx".to_owned();
    let call_span = callee.span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful missing String callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing owned callee");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    let at = diagnostics[0].primary_span().expect("callee span");
    assert_eq!((at.start(), at.end()), (call_span.start, call_span.end));
}

#[test]
fn owned_call_cleanup_amplification_has_checked_exact_plus_one_and_overflow_boundaries() {
    let plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!owned_call_cleanup_budget_violation(plans - 1, actions - 1, 2, 1));
    assert!(owned_call_cleanup_budget_violation(plans, 0, 1, 1));
    assert!(owned_call_cleanup_budget_violation(0, actions, 2, 1));
    assert!(owned_call_cleanup_budget_violation(0, usize::MAX, 2, 1));
    assert!(owned_call_cleanup_budget_violation(0, 0, 0, 1));
}

#[test]
fn private_string_call_rejects_use_after_argument_transfer_at_the_reference() {
    let (source, raw) = private_string_call_fixture();
    let producer_call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, producer_call_end, -2);
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: argument_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: argument_span },
        },
    };
    let old_reference = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_reference.end, 3);
    let returned_name = zryna_source::UntrustedSpan {
        file: 0,
        start: old_reference.start,
        end: old_reference.start + 8,
    };
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: returned_name,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: returned_name },
        },
    };
    let source =
        source.replacen("producer()", "survivor", 1).replacen("clone(value)", "clone(survivor)", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful transferred String reuse");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("use after call transfer");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved String diagnostic");
    let at = diagnostic.primary_span().expect("reference span");
    assert_eq!(at.end() - at.start(), 8);
}

#[test]
fn private_string_call_rejects_the_same_owned_binding_in_a_second_call() {
    let (source, raw) = private_string_call_fixture();
    let producer_call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, producer_call_end, -2);
    let first_use = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: first_use,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: first_use },
        },
    };
    let old_second_use = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_second_use.end, 3);
    let second_use = zryna_source::UntrustedSpan {
        file: 0,
        start: old_second_use.start,
        end: old_second_use.start + 8,
    };
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: second_use,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: second_use },
        },
    };
    let clone_keyword_end = match &raw.files[0].functions[0].body.expressions[4].kind {
        zryna_syntax::v4::RawExpressionKind::Clone { keyword_span, .. } => keyword_span.end,
        _ => panic!("clone"),
    };
    let mut raw = shift_snapshot(raw, clone_keyword_end, 3);
    let second_use = raw.files[0].functions[0].body.expressions[3].span;
    let replacement = raw.files[0].functions[0].body.expressions[4].clone();
    let zryna_syntax::v4::RawExpressionKind::Clone {
        keyword_span,
        open_paren_span,
        value,
        close_paren_span,
    } = replacement.kind
    else {
        panic!("shifted clone")
    };
    raw.files[0].functions[0].body.expressions[4] = RawExpressionSyntax {
        span: replacement.span,
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "identity".to_owned(), span: keyword_span },
            open_paren_span,
            arguments: vec![value],
            close_paren_span,
        },
    };
    let source = source.replacen("producer()", "survivor", 1).replacen(
        "clone(value)",
        "identity(survivor)",
        1,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful duplicate owned call use");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate owned call use");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("second transfer diagnostic");
    let at = diagnostic.primary_span().expect("second use span");
    assert_eq!((at.start(), at.end()), (second_use.start, second_use.end));
}

#[test]
fn private_string_call_name_is_exact_and_cycles_fail_closed_in_ir() {
    let (source, mut raw) = private_string_call_fixture();
    let source = source.replacen("producer()", "Producer()", 1);
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("producer call")
    };
    callee.text = "Producer".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-case String call");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong-case String call");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");

    let (source, raw) = private_string_call_fixture();
    let literal = raw.files[0].functions[2].body.expressions[0].span;
    let mut raw = shift_snapshot_signed(raw, literal.end, 4);
    let call_span =
        zryna_source::UntrustedSpan { file: 0, start: literal.start, end: literal.end + 4 };
    raw.files[0].functions[2].body.expressions[0] = RawExpressionSyntax {
        span: call_span,
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "producer".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: literal.start,
                    end: literal.start + 8,
                },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: literal.start + 8,
                end: literal.start + 9,
            },
            arguments: Vec::new(),
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: literal.start + 9,
                end: literal.start + 10,
            },
        },
    };
    let source = source.replacen("\"made\"", "producer()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive producer");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("recursive owned call");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}

#[test]
fn private_vec_producer_and_identity_calls_transfer_exact_owners() {
    for element in ["bool", "i32", "String"] {
        let (source, raw) = private_vec_call_fixture(element);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful private Vec calls");
        let program =
            lower(pair_input(&syntax, &sources)).expect("Vec producer and identity calls");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        let caller = &functions[0];
        let identity = &functions[1];
        let caller_block = caller.blocks().next().expect("caller block");
        let calls = caller_block
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2, "{element}");
        assert_eq!(calls[0].callee().expect("producer").declaration(), 2);
        assert_eq!(calls[0].call_arguments().count(), 0);
        assert_eq!(calls[1].callee().expect("identity").declaration(), 1);
        assert_eq!(calls[1].call_arguments().count(), 1);
        assert_ne!(
            calls[0].cleanup().expect("producer cleanup"),
            calls[1].cleanup().expect("identity cleanup")
        );
        for call in &calls {
            let result = call.result().expect("Vec call result");
            assert_eq!(
                caller
                    .places()
                    .filter(|place| {
                        matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == result)
                    })
                    .count(),
                1,
                "{element} call result has one exact Temporary owner"
            );
        }
        for call in calls {
            assert_eq!(
                call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
                [1],
                "{element} CallTrap retains only survivor"
            );
            assert_eq!(
                caller
                    .cleanup_plans()
                    .find(|plan| plan.id() == call.cleanup().expect("CallTrap"))
                    .expect("plan")
                    .site()
                    .role(),
                VerifiedCleanupRole::CallTrap
            );
        }
        assert!(identity.places().any(|place| { place.kind() == VerifiedPlaceKind::Parameter(0) }));
        assert_eq!(identity.parameters().next().expect("dense Vec parameter").id().index(), 0);
        assert_eq!(
            identity
                .blocks()
                .next()
                .expect("identity block")
                .terminator()
                .derived_drop_actions()
                .count(),
            0
        );
        assert_eq!(caller_block.terminator().derived_drop_actions().count(), 1);
    }
}

#[test]
fn private_vec_identity_call_accepts_nested_string_retaining_construction() {
    let (source, raw) = private_vec_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested Vec identity call");
    let caller =
        program.verified_ir().modules().next().expect("module").functions().next().expect("caller");
    let kinds = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&VerifiedInstructionKind::StringConcat));
    assert!(kinds.contains(&VerifiedInstructionKind::VecConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::DirectCall));
}

#[test]
fn private_vec_direct_call_reports_unsupported_nested_string_element_once_at_expression() {
    let (mut source, raw) = private_vec_nested_string_call_fixture();
    let old = "concat(\"a\", \"b\")";
    let start = u32::try_from(source.find(old).expect("nested concat")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        "1",
    );
    let mut raw =
        shift_snapshot_signed(raw, old_end, 1 - i32::try_from(old.len()).expect("length"));
    let body = &mut raw.files[0].functions[0].body;
    let survivor = body.expressions[0].clone();
    let mut construct = body.expressions[4].clone();
    let mut identity = body.expressions[5].clone();
    let returned = body.expressions[6].clone();
    let literal_span = zryna_source::UntrustedSpan { file: 0, start, end: start + 1 };
    let literal = RawExpressionSyntax {
        span: literal_span,
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
    };
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } = &mut construct.kind
    else {
        panic!("nested construction")
    };
    *elements = vec![1];
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity.kind else {
        panic!("identity call")
    };
    *arguments = vec![2];
    body.expressions = vec![survivor, literal, construct, identity, returned];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("result declaration")
    };
    *initializer = 3;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 4;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested call rejection");
    let first = lower(pair_input(&syntax, &sources)).expect_err("unsupported nested call element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic nested call rejection");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, literal_span)));
}

#[test]
fn private_vec_call_rejects_moved_argument_reuse_and_wrong_case() {
    let (source, raw) = private_vec_call_fixture("i32");
    let call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, call_end, -2);
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: argument_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: argument_span },
        },
    };
    let old_return = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_return.end, 2);
    let returned = joined_test_span(old_return.start, old_return.end + 2);
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: returned,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: returned },
        },
    };
    let source = source.replacen("producer()", "survivor", 1).replacen(
        "return result",
        "return survivor",
        1,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved Vec reuse");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved Vec reuse");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3014"));

    for replacement in ["Producer", "missingx"] {
        let (source, mut raw) = private_vec_call_fixture("i32");
        let source = source.replacen("producer()", &format!("{replacement}()"), 1);
        let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
            &mut raw.files[0].functions[0].body.expressions[1].kind
        else {
            panic!("producer call")
        };
        callee.text = replacement.to_owned();
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved Vec call");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved Vec call");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    }
}

#[test]
fn private_vec_call_rejects_arity_export_and_cycles() {
    let (source, raw) = private_vec_call_fixture("i32");
    let removed = raw.files[0].functions[0].body.expressions[1].span;
    let mut raw = shift_snapshot_signed(raw, removed.end, -10);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[1].kind
    else {
        panic!("identity call")
    };
    arguments.clear();
    let RawStatementKind::LocalDeclaration { initializer, .. } =
        &mut caller.body.statements[1].kind
    else {
        panic!("result local")
    };
    *initializer = 1;
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[2].kind else {
        panic!("return")
    };
    *value = 2;
    let source = source.replacen("identity(producer())", "identity()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec call arity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec call arity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (source, raw) = private_vec_call_fixture("i32");
    let cutoff = raw.files[0].functions[1].span.start;
    let mut raw = shift_snapshot(raw, cutoff, 7);
    let identity = &mut raw.files[0].functions[1];
    identity.export_span = Some(joined_test_span(cutoff, cutoff + 6));
    identity.span.start = cutoff;
    let source = source.replacen(" function identity", " export function identity", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported Vec callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("exported Vec callee");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (mut source, mut raw) = private_vec_call_fixture("i32");
    for element_id in [8_usize, 10] {
        let old = raw.files[0].type_syntax[element_id].span;
        source.replace_range(
            usize::try_from(old.start).expect("start")..usize::try_from(old.end).expect("end"),
            "bool",
        );
        raw = shift_snapshot(raw, old.end, 1);
        let at = raw.files[0].type_syntax[element_id].span;
        raw.files[0].type_syntax[element_id].kind = RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: at },
        };
    }
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec family mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec family mismatch");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (source, raw) = private_vec_call_fixture("i32");
    let old = raw.files[0].functions[1].body.expressions[0].span;
    let mut raw = shift_snapshot_signed(raw, old.end, 10);
    let expression = &mut raw.files[0].functions[1].body.expressions[0];
    expression.span = joined_test_span(old.start, old.end + 10);
    expression.kind = zryna_syntax::v4::RawExpressionKind::Call {
        callee: RawIdentifierSyntax {
            text: "identity".to_owned(),
            span: joined_test_span(old.start, old.start + 8),
        },
        open_paren_span: joined_test_span(old.start + 8, old.start + 9),
        arguments: vec![0],
        close_paren_span: joined_test_span(old.end + 9, old.end + 10),
    };
    let reference = RawExpressionSyntax {
        span: joined_test_span(old.start + 9, old.end + 9),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "value".to_owned(),
                span: joined_test_span(old.start + 9, old.end + 9),
            },
        },
    };
    raw.files[0].functions[1].body.expressions.insert(0, reference);
    let RawStatementKind::Return { value, .. } =
        &mut raw.files[0].functions[1].body.statements[0].kind
    else {
        panic!("identity return")
    };
    *value = 1;
    let source = source.replacen("return value", "return identity(value)", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive Vec identity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("recursive Vec identity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}

fn joined_test_span(start: u32, end: u32) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan { file: 0, start, end }
}

#[test]
fn private_copy_scalar_forward_call_has_one_exact_empty_call_trap_site() {
    let sources = sources_for(COPY_CALL_SOURCE);
    let syntax = verify_snapshot(response_snapshot(COPY_CALL_RESPONSE), &sources)
        .expect("source-faithful Copy call v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private forward Copy call");
    let caller = program.modules().next().expect("module").functions().next().expect("caller");
    let call = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("DirectCall");
    assert_eq!(call.callee().expect("callee").declaration(), 1);
    assert_eq!(call.call_arguments().count(), 2);
    let cleanup = call.cleanup().expect("call cleanup");
    let plan =
        caller.cleanup_plans().find(|plan| plan.id() == cleanup).expect("sealed cleanup plan");
    assert_eq!(plan.actions().count(), 0);
    assert_eq!(plan.site().role(), VerifiedCleanupRole::CallTrap);
    assert_ne!(
        cleanup.index(),
        caller
            .blocks()
            .next()
            .expect("block")
            .terminator()
            .cleanup()
            .expect("return cleanup")
            .index()
    );
}

#[test]
fn private_copy_aggregate_call_preserves_copy_source_for_reuse() {
    let sources = sources_for(COPY_AGGREGATE_CALL_SOURCE);
    let response = COPY_AGGREGATE_CALL_RESPONSE.replace("},{span", "},{\"span");
    let syntax = verify_snapshot(response_snapshot(&response), &sources)
        .expect("source-faithful Copy aggregate call v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Copy aggregate call");
    let use_function =
        program.modules().next().expect("module").functions().nth(1).expect("use function");
    let instructions = use_function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().filter(|kind| **kind == VerifiedInstructionKind::DirectCall).count(),
        1
    );
    assert!(
        instructions.iter().filter(|kind| **kind == VerifiedInstructionKind::CopyFromPlace).count()
            >= 3
    );
}

#[test]
fn private_copy_call_names_are_exact_and_module_local() {
    for replacement in ["Add", "bad"] {
        let source = COPY_CALL_SOURCE.replacen("return add", &format!("return {replacement}"), 1);
        let sources = sources_for(&source);
        let mut raw = response_snapshot(COPY_CALL_RESPONSE);
        let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
            &mut raw.files[0].functions[0].body.expressions[2].kind
        else {
            panic!("call")
        };
        callee.text = replacement.to_owned();
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved call v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved call");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
        let at = diagnostics[0].primary_span().expect("callee span");
        assert_eq!((at.start(), at.end()), (38, 41));
    }
}

#[test]
fn function_catalog_rejects_case_and_concat_builtin_collisions() {
    for replacement in ["Caller", "concat", "Concat"] {
        let source =
            COPY_CALL_SOURCE.replacen("function add", &format!("function {replacement}"), 1);
        let sources = sources_for(&source);
        let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 63, 3);
        raw.files[0].functions[1].name.text = replacement.to_owned();
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful function collision v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("function collision");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
    }
}

#[test]
fn private_copy_call_rejects_a_public_callee() {
    let source = COPY_CALL_SOURCE.replacen("function add", "export function add", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 51, 7);
    let callee = &mut raw.files[0].functions[1];
    callee.span.start = 51;
    callee.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 51, end: 57 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful public callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public callee");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3008")
        .expect("private-call diagnostic");
    let at = diagnostic.primary_span().expect("callee span");
    assert_eq!((at.start(), at.end()), (38, 41));
}

#[test]
fn private_copy_call_rejects_too_few_and_too_many_arguments() {
    let source = COPY_CALL_SOURCE.replacen("add(x, 1)", "add(x   )", 1);
    let sources = sources_for(&source);
    let mut raw = response_snapshot(COPY_CALL_RESPONSE);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[1].kind
    else {
        panic!("call")
    };
    arguments.pop();
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[0].kind else {
        panic!("return")
    };
    *value = 1;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful short call");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("short call");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3008");
    assert_eq!(diagnostics[0].primary_span().expect("call span").start(), 38);

    let source = COPY_CALL_SOURCE.replacen("add(x, 1)", "add(x, 1, 2)", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 46, 3);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions[1].span.end = 46;
    let mut call = caller.body.expressions.pop().expect("call");
    let extra = u32::try_from(caller.body.expressions.len()).expect("extra id");
    caller.body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 48, end: 49 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "2".to_owned() },
    });
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut call.kind else {
        panic!("call")
    };
    arguments.push(extra);
    caller.body.expressions.push(call);
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful long call");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("long call");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3008");
    assert_eq!(diagnostics[0].primary_span().expect("call span").start(), 38);
}

#[test]
fn private_copy_call_rejects_argument_and_result_type_mismatches_at_source() {
    let source = COPY_CALL_SOURCE.replacen("add(x, 1)", "add(x, true)", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 46, 3);
    raw.files[0].functions[0].body.expressions[1].kind =
        zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong argument type");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong argument type");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3007")
        .expect("argument type diagnostic");
    let at = diagnostic.primary_span().expect("argument span");
    assert_eq!((at.start(), at.end()), (45, 49));

    let source = COPY_CALL_SOURCE.replacen("caller(x: i32): i32", "caller(x: i32): bool", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 28, 1);
    raw.files[0].type_syntax[1] = RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 25, end: 29 },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "bool".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 25, end: 29 },
            },
        },
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong call result");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong result type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3007"));
}

#[test]
fn source_faithful_copy_call_cycles_fail_closed_in_ir_authority() {
    for (replacement, callee, arguments) in
        [("add(x, y)", "add", vec![0, 1]), ("caller(x)", "caller", vec![0])]
    {
        let source = COPY_CALL_SOURCE.replacen("x + y", replacement, 1);
        let sources = sources_for(&source);
        let mut raw = shift_snapshot(response_snapshot(COPY_CALL_RESPONSE), 99, 4);
        let function = &mut raw.files[0].functions[1];
        let expressions = if arguments.len() == 2 {
            vec![
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: 98, end: 99 },
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "x".to_owned(),
                            span: zryna_source::UntrustedSpan { file: 0, start: 98, end: 99 },
                        },
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: 101, end: 102 },
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "y".to_owned(),
                            span: zryna_source::UntrustedSpan { file: 0, start: 101, end: 102 },
                        },
                    },
                },
            ]
        } else {
            vec![RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 101, end: 102 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 101, end: 102 },
                    },
                },
            }]
        };
        function.body.expressions = expressions;
        let call = u32::try_from(function.body.expressions.len()).expect("call id");
        function.body.expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: 94, end: 103 },
            kind: zryna_syntax::v4::RawExpressionKind::Call {
                callee: RawIdentifierSyntax {
                    text: callee.to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: 94,
                        end: 94 + u32::try_from(callee.len()).expect("callee length"),
                    },
                },
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: 94 + u32::try_from(callee.len()).expect("callee length"),
                    end: 95 + u32::try_from(callee.len()).expect("callee length"),
                },
                arguments,
                close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 102, end: 103 },
            },
        });
        let RawStatementKind::Return { value, .. } = &mut function.body.statements[0].kind else {
            panic!("return")
        };
        *value = call;
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful call cycle");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("call cycle");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
    }
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
fn owner_state_consumption_removes_every_stale_value_claim() {
    let mut owners = OwnerState::default();
    let owner = raw::PlaceId(7);
    let first = raw::ValueId(11);
    let stale_alias = raw::ValueId(12);
    let _ = owners.register(first, owner);
    owners.value_owners.insert(stale_alias, owner);

    assert!(owners.consume_owner(owner).is_some());
    assert!(!owners.contains(owner));
    assert_eq!(owners.owner(first), None);
    assert_eq!(owners.owner(stale_alias), None);
}

#[test]
fn owner_state_rejects_duplicate_alias_and_self_rehome_without_mutation() {
    let mut owners = OwnerState::default();
    let first_owner = raw::PlaceId(3);
    let second_owner = raw::PlaceId(4);
    let first_value = raw::ValueId(8);
    let second_value = raw::ValueId(9);
    assert!(owners.register(first_value, first_owner).is_some());
    assert!(owners.register(first_value, second_owner).is_none());
    assert!(owners.register(second_value, first_owner).is_none());
    assert!(owners.register_parameter(first_owner).is_none());
    assert_eq!(owners.pending(), &[first_owner]);
    assert_eq!(owners.owner(first_value), Some(first_owner));

    assert!(owners.register(second_value, second_owner).is_some());
    assert!(owners.rename(second_value, first_owner).is_none());
    assert!(owners.rehome_move_result(first_value, first_owner).is_none());
    assert_eq!(owners.pending(), &[first_owner, second_owner]);
    assert_eq!(owners.owner(first_value), Some(first_owner));
    assert_eq!(owners.owner(second_value), Some(second_owner));
}

#[test]
fn owned_cfg_budgets_are_checked_at_exact_plus_one_and_overflow() {
    let blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION;
    let edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(owned_cfg_budget_violation(blocks, edges, transitions), None);
    assert_eq!(
        owned_cfg_budget_violation(blocks + 1, edges, transitions),
        Some(OwnedCfgBudgetLimit::Blocks)
    );
    assert_eq!(
        owned_cfg_budget_violation(blocks, edges + 1, transitions),
        Some(OwnedCfgBudgetLimit::Edges)
    );
    assert_eq!(
        owned_cfg_budget_violation(blocks, edges, transitions + 1),
        Some(OwnedCfgBudgetLimit::Transitions)
    );
    assert_eq!(
        owned_cfg_budget_violation(usize::MAX, usize::MAX, usize::MAX),
        Some(OwnedCfgBudgetLimit::Blocks)
    );
    assert_eq!(dense_owned_value_id(u32::MAX as usize), Some(raw::ValueId(u32::MAX)));
    assert_eq!(dense_owned_value_id(u32::MAX as usize + 1), None);
    assert_eq!(dense_owned_value_id(usize::MAX), None);
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    assert!(!owned_value_budget_violation(values, 0));
    assert!(!owned_value_budget_violation(values - 1, 1));
    assert!(owned_value_budget_violation(values, 1));
    assert!(owned_value_budget_violation(usize::MAX, 1));
    assert!(!owned_place_budget_violation(places, 0));
    assert!(!owned_place_budget_violation(places - 1, 1));
    assert!(owned_place_budget_violation(places, 1));
    assert!(owned_place_budget_violation(usize::MAX, 1));
}

#[test]
fn owned_cfg_value_ledger_is_atomic_for_parameters_blocks_and_results() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let definition = |index| raw::ValueDefinition {
        id: raw::ValueId(u32::try_from(index).expect("bounded value")),
        ty: raw::TypeId(0),
        span: at,
    };

    let mut parameter_errors = Errors::new(&sources);
    let mut parameters = OwnedCfgState::single_block(at, &mut parameter_errors).expect("entry");
    parameters.value_types.resize(maximum - 1, raw::TypeId(0));
    parameters
        .seed_function_parameter(&definition(maximum - 1), &mut parameter_errors)
        .expect("exact value budget");
    assert_eq!(parameters.value_types.len(), maximum);
    assert!(
        parameters.seed_function_parameter(&definition(maximum), &mut parameter_errors).is_none()
    );
    assert_eq!(parameters.value_types.len(), maximum);

    let mut block_errors = Errors::new(&sources);
    let mut blocks = OwnedCfgState::single_block(at, &mut block_errors).expect("entry");
    let successor = blocks.reserve_block(at, &mut block_errors).expect("successor");
    assert!(blocks.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut block_errors,
    ));
    blocks.value_types.resize(maximum - 1, raw::TypeId(0));
    assert!(
        blocks
            .begin_block(
                successor,
                vec![definition(maximum - 1), definition(maximum)],
                at,
                &mut block_errors,
            )
            .is_none()
    );
    assert_eq!(blocks.value_types.len(), maximum - 1);
    assert!(!blocks.arena.blocks[1].populated);

    let mut result_errors = Errors::new(&sources);
    let mut results = OwnedCfgState::single_block(at, &mut result_errors).expect("entry");
    results.value_types.resize(maximum, raw::TypeId(0));
    assert!(!results.emit(
        raw::Instruction {
            result: Some(definition(maximum)),
            span: at,
            kind: raw::InstructionKind::BoolLiteral(true),
        },
        &mut result_errors,
    ));
    assert_eq!(results.value_types.len(), maximum);
    assert_eq!(results.transitions, 0);
    assert!(results.current_block().expect("entry").instructions.is_empty());

    for errors in [parameter_errors, block_errors, result_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
        assert!(diagnostics[0].message().contains("owned CFG values"));
    }
}

#[test]
fn owned_cfg_value_reservation_keeps_child_and_parent_ids_dense() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let definition =
        |id| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(0), span: at };
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.reserve_values(1, at, &mut errors).expect("parent reservation");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(definition(0)),
            span: at,
            kind: raw::InstructionKind::I32Literal(1),
        },
        &mut errors,
    ));
    cfg.release_values(1);
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(definition(1)),
            span: at,
            kind: raw::InstructionKind::I32Literal(2),
        },
        &mut errors,
    ));
    assert_eq!(cfg.value_types.len(), 2);
    assert!(errors.finish().is_empty());

    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let mut limit_errors = Errors::new(&sources);
    let mut limited = OwnedCfgState::single_block(at, &mut limit_errors).expect("entry");
    limited.value_types.resize(maximum - 1, raw::TypeId(0));
    limited.reserve_values(1, at, &mut limit_errors).expect("call-result reservation");
    assert!(!limited.emit(
        raw::Instruction {
            result: Some(definition(u32::try_from(maximum - 1).expect("value id"))),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut limit_errors,
    ));
    assert_eq!(limited.value_types.len(), maximum - 1);
    assert_eq!(limited.transitions, 0);
    assert!(limited.current_block().expect("entry").instructions.is_empty());
    let diagnostics = limit_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn owned_cfg_reserved_local_commit_transition_blocks_initializer_without_mutation() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.transitions = maximum - 1;
    cfg.reserve_transitions(1, at, &mut errors).expect("InitializePlace reservation");
    assert!(!cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(0),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::StringFromUtf8 {
                bytes: b"x".to_vec(),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    assert_eq!(cfg.transitions, maximum - 1);
    assert!(cfg.value_types.is_empty());
    assert!(cfg.current_block().expect("entry").instructions.is_empty());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn owned_place_preflight_is_source_located_and_string_temporary_failure_is_atomic() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let maximum = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;

    let mut errors = Errors::new(&sources);
    assert!(preflight_owned_place_capacity(maximum - 1, 1, at, &mut errors));
    assert!(!preflight_owned_place_capacity_with_reserved(maximum - 1, 1, 1, at, &mut errors,));
    assert_eq!(errors.finish()[0].primary_span(), Some(at));
    let mut errors = Errors::new(&sources);
    let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        places: vec![
            raw::Place {
                id: raw::PlaceId(0),
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            };
            maximum - 1
        ],
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState::default(),
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    assert!(lowerer.reserve_local_place(at));
    assert!(
        lowerer
            .push_temporary(
                at,
                raw::InstructionKind::StringFromUtf8 {
                    bytes: b"x".to_vec(),
                    cleanup: raw::CleanupPlanId(0),
                },
            )
            .is_none()
    );
    assert_eq!(lowerer.places.len(), maximum - 1);
    assert_eq!(lowerer.reserved_places, 1);
    assert_eq!(lowerer.next_value, 0);
    assert_eq!(lowerer.cfg.transitions, 0);
    assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
    assert!(lowerer.owners.pending().is_empty());
    lowerer.release_local_place();
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut overflow_errors = Errors::new(&sources);
    assert!(!preflight_owned_place_capacity(usize::MAX, 1, at, &mut overflow_errors));
    let diagnostics = overflow_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_string_nested_identity_result_reservation_fails_before_argument_mutation() {
    let (source, raw) = private_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String calls");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let signature = |declaration, name: &str, parameters| FunctionSignature {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration },
        name: name.to_owned(),
        parameters,
        result: ty,
        private: true,
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[2];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    cfg.value_types.resize(maximum - 1, ty.ir);
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
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
        known_bytes: std::collections::BTreeMap::new(),
        next_value: u32::try_from(maximum - 1).expect("value limit"),
        next_local: 0,
    };
    assert!(lowerer.direct_call(callee, arguments, at).is_none());
    assert_eq!(lowerer.cfg.value_types.len(), maximum - 1);
    assert_eq!(lowerer.cfg.transitions, 0);
    assert_eq!(lowerer.cfg.reserved_values, 0);
    assert_eq!(lowerer.cfg.reserved_transitions, 0);
    assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
    assert!(lowerer.places.is_empty());
    assert_eq!(lowerer.reserved_places, 0);
    assert!(lowerer.owners.pending().is_empty());
    assert!(lowerer.known_bytes.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &function.body.expressions[3].kind
    else {
        panic!("call fixture reference")
    };
    let mut cleanup_errors = Errors::new(&sources);
    let mut cleanup = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut cleanup_errors,
        at,
        0,
    );
    cleanup
        .bindings
        .insert(name.text.clone(), super::Binding { ty, place: raw::PlaceId(0), mutable: false });
    cleanup.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: ty.ir,
        span: at,
        kind: raw::PlaceKind::Local(0),
    }];
    cleanup.owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    cleanup.known_bytes.insert(raw::PlaceId(0), Some(1));
    cleanup.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let before = (
        cleanup.bindings.clone(),
        cleanup.owners.clone(),
        cleanup.known_bytes.clone(),
        cleanup.cfg.current_block().expect("entry").instructions.clone(),
    );
    assert!(cleanup.direct_call(callee, &[3], at).is_none());
    assert_eq!(
        (
            cleanup.bindings.clone(),
            cleanup.owners.clone(),
            cleanup.known_bytes.clone(),
            cleanup.cfg.current_block().expect("entry").instructions.clone(),
        ),
        before
    );
    assert_eq!(cleanup.cfg.reserved_values, 0);
    assert_eq!(cleanup.cfg.reserved_transitions, 0);
    assert_eq!(cleanup.reserved_places, 0);
    drop(cleanup);
    let diagnostics = cleanup_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
#[allow(clippy::too_many_lines)]
fn recursive_owned_string_preflight_is_exact_atomic_and_overflow_checked_for_all_consumers() {
    fn assert_boundaries(
        estimate: super::OwnedStringPreparationEstimate,
        ty: super::Ty,
        sources: &SourceMap,
        at: zryna_source::Span,
    ) {
        let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - estimate.values;
        let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            - estimate.transitions;
        let budget = OwnedStringPreparationBudget {
            cleanup_plans: zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
                - estimate.cleanup_plans,
            reserved_cleanup_plans: 0,
            cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions,
            reserved_cleanup_actions: 0,
            places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - estimate.places,
            reserved_places: 0,
        };
        let mut exact_errors = Errors::new(sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry");
        cfg.value_types.resize(values, ty.ir);
        cfg.transitions = transitions;
        let before =
            (cfg.value_types.len(), cfg.transitions, cfg.reserved_values, cfg.reserved_transitions);
        assert!(preflight_owned_string_preparation(
            estimate,
            budget,
            &mut cfg,
            at,
            &mut exact_errors,
        ));
        assert_eq!(
            (cfg.value_types.len(), cfg.transitions, cfg.reserved_values, cfg.reserved_transitions,),
            before
        );
        assert!(exact_errors.finish().is_empty());

        let mut extra_errors = Errors::new(sources);
        let mut extra_cfg = OwnedCfgState::single_block(at, &mut extra_errors).expect("entry");
        let extra_budget =
            OwnedStringPreparationBudget { cleanup_actions: budget.cleanup_actions + 1, ..budget };
        let before = (
            extra_cfg.value_types.len(),
            extra_cfg.transitions,
            extra_cfg.reserved_values,
            extra_cfg.reserved_transitions,
        );
        assert!(!preflight_owned_string_preparation(
            estimate,
            extra_budget,
            &mut extra_cfg,
            at,
            &mut extra_errors,
        ));
        assert_eq!(
            (
                extra_cfg.value_types.len(),
                extra_cfg.transitions,
                extra_cfg.reserved_values,
                extra_cfg.reserved_transitions,
            ),
            before
        );
        let diagnostics = extra_errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));

        let mut overflow_errors = Errors::new(sources);
        let mut overflow_cfg =
            OwnedCfgState::single_block(at, &mut overflow_errors).expect("entry");
        let overflow_budget =
            OwnedStringPreparationBudget { cleanup_actions: usize::MAX, ..budget };
        assert!(!preflight_owned_string_preparation(
            estimate,
            overflow_budget,
            &mut overflow_cfg,
            at,
            &mut overflow_errors,
        ));
        let diagnostics = overflow_errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }

    let (call_source, call_raw) = private_nested_string_call_fixture();
    let call_sources = sources_for(&call_source);
    let call_syntax = verify_snapshot(call_raw, &call_sources).expect("nested call fixture");
    let call_input = pair_input(&call_syntax, &call_sources);
    let call_ty = authenticated_type_capabilities(call_input, 0, 0).expect("String type");
    let call_function = &call_syntax.files()[0].functions()[0];
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "survivor".to_owned(),
        super::Binding { ty: call_ty, place: raw::PlaceId(0), mutable: false },
    );
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let call_estimate = estimate_owned_string_expression(
        call_function,
        &bindings,
        &owners,
        call_ty,
        4,
        1,
        OwnedStringEstimateContext::Value,
    )
    .expect("nested call estimate");
    assert_boundaries(
        call_estimate,
        call_ty,
        &call_sources,
        span(&call_sources, call_function.body.expressions[4].span),
    );

    let (vec_source, vec_raw) = private_vec_nested_string_fixture();
    let vec_sources = sources_for(&vec_source);
    let vec_syntax = verify_snapshot(vec_raw, &vec_sources).expect("nested Vec fixture");
    let vec_input = pair_input(&vec_syntax, &vec_sources);
    let vec_string = authenticated_type_capabilities(vec_input, 0, 0).expect("String type");
    let vec_function = &vec_syntax.files()[0].functions()[0];
    for (expression, pending) in [(2, 0), (7, 1)] {
        let owners = OwnerState {
            pending: (0..pending).map(raw::PlaceId).collect(),
            value_owners: std::collections::BTreeMap::new(),
        };
        let estimate = estimate_owned_string_expression(
            vec_function,
            &std::collections::BTreeMap::new(),
            &owners,
            vec_string,
            expression,
            usize::try_from(pending).expect("pending"),
            OwnedStringEstimateContext::Value,
        )
        .expect("nested Vec element estimate");
        assert_boundaries(
            estimate,
            vec_string,
            &vec_sources,
            span(
                &vec_sources,
                vec_function.body.expressions[usize::try_from(expression).expect("expression")]
                    .span,
            ),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_direct_call_uses_preflight_credit_without_conservative_double_counting() {
    let (source, raw_snapshot) = private_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw_snapshot, &sources).expect("nested call fixture");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let signature = |declaration, name: &str, parameters| FunctionSignature {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration },
        name: name.to_owned(),
        parameters,
        result: ty,
        private: true,
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[4];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "survivor".to_owned(),
        super::Binding { ty, place: raw::PlaceId(0), mutable: false },
    );
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let estimate = estimate_owned_string_expression(
        function,
        &bindings,
        &owners,
        ty,
        4,
        1,
        OwnedStringEstimateContext::Value,
    )
    .expect("nested call estimate");
    let value_base = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - estimate.values;
    let place_base = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - estimate.places;
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.value_types.resize(value_base, ty.ir);
    cfg.transitions =
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - estimate.transitions;
    let places = (0..place_base)
        .map(|index| raw::Place {
            id: raw::PlaceId(u32::try_from(index).expect("place id")),
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(u32::try_from(index).expect("local id")),
        })
        .collect();
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings,
        places,
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans,
        reserved_cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
            - estimate.cleanup_actions,
        owners,
        known_bytes: std::collections::BTreeMap::from([(raw::PlaceId(0), Some(4))]),
        next_value: u32::try_from(value_base).expect("value id"),
        next_local: u32::try_from(place_base).expect("local id"),
    };
    let result = lowerer.direct_call(callee, arguments, at);
    assert!(result.is_some(), "exact nested preparation must not be double counted");
    assert_eq!(lowerer.cfg.value_types.len(), zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION);
    assert_eq!(lowerer.places.len(), zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION);
    assert_eq!(
        lowerer.cfg.transitions,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
    );
    drop(lowerer);
    assert!(errors.finish().is_empty());
}

#[test]
fn generated_program_cfg_budgets_are_checked_at_exact_plus_one_and_overflow() {
    let blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_PROGRAM;
    let edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_PROGRAM;
    assert_eq!(generated_cfg_budget_violation(blocks, edges, 0, 0), None);
    assert_eq!(generated_cfg_budget_violation(blocks - 1, edges, 1, 0), None);
    assert_eq!(
        generated_cfg_budget_violation(blocks, edges, 1, 0),
        Some(ProgramCfgBudgetLimit::Blocks)
    );
    assert_eq!(generated_cfg_budget_violation(blocks, edges - 1, 0, 1), None);
    assert_eq!(
        generated_cfg_budget_violation(blocks, edges, 0, 1),
        Some(ProgramCfgBudgetLimit::Edges)
    );
    assert_eq!(
        generated_cfg_budget_violation(usize::MAX, 0, 1, 0),
        Some(ProgramCfgBudgetLimit::Blocks)
    );
    assert_eq!(
        generated_cfg_budget_violation(0, usize::MAX, 0, 1),
        Some(ProgramCfgBudgetLimit::Edges)
    );
}

#[test]
fn generated_value_composition_counts_only_emitted_definitions_at_exact_boundaries() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("function span");
    let definition =
        |id| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(0), span: at };
    let function = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        entry_export: None,
        span: at,
        parameters: vec![definition(0)],
        borrow_parameters: Vec::new(),
        result: raw::TypeId(0),
        places: Vec::new(),
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: vec![raw::Instruction {
                result: Some(definition(1)),
                span: at,
                kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(0) },
            }],
            terminators: Vec::new(),
        }],
        cleanup_plans: Vec::new(),
    };
    // A fixed-array constant index emits only CopyFromPlace; its literal index is not a value.
    assert_eq!(raw_function_value_count(&function), Some(2));

    let mut vec_index = function.clone();
    vec_index.parameters.clear();
    vec_index.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(definition(0)),
            span: at,
            kind: raw::InstructionKind::I32Literal(0),
        },
        raw::Instruction {
            result: Some(definition(1)),
            span: at,
            kind: raw::InstructionKind::VecIndexCopy {
                place: raw::PlaceId(0),
                index: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
    ];
    // Vec indexing emits both the runtime index value and the checked result.
    assert_eq!(raw_function_value_count(&vec_index), Some(2));

    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_PROGRAM;
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(maximum - 2, &function, &mut exact_errors),
        Some(maximum)
    );
    assert!(exact_errors.finish().is_empty());

    let mut extra_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(maximum - 1, &function, &mut extra_errors),
        None
    );
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut overflow_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(usize::MAX, &function, &mut overflow_errors),
        None
    );
    assert_eq!(overflow_errors.finish()[0].primary_span(), Some(at));
}

#[test]
fn generated_cfg_edge_table_and_cross_function_first_extra_span_are_exact() {
    let edge = || raw::Edge { target: raw::BlockId(1), arguments: Vec::new() };
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Return {
            value: raw::ValueId(0),
            cleanup: raw::CleanupPlanId(0),
        }),
        0
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Trap {
            identity: raw::TrapIdentity::BoundsV1,
            cleanup: raw::CleanupPlanId(0),
        }),
        0
    );
    assert_eq!(raw_terminator_edge_count(&raw::Terminator::Jump(edge())), 1);
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Branch {
            condition: raw::ValueId(0),
            when_true: edge(),
            when_false: edge(),
        }),
        2
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::EnumMatch {
            place: raw::PlaceId(0),
            arms: (0..3).map(|variant| raw::EnumArm { variant, edge: edge() }).collect(),
        }),
        3
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::WeakUpgradeBranch {
            weak: raw::PlaceId(0),
            success: edge(),
            expired: edge(),
            cleanup: raw::CleanupPlanId(0),
        }),
        2
    );

    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("function span");
    let function = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        entry_export: None,
        span: at,
        parameters: Vec::new(),
        borrow_parameters: Vec::new(),
        result: raw::TypeId(0),
        places: Vec::new(),
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(edge()),
            }],
        }],
        cleanup_plans: Vec::new(),
    };
    let maximum = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_PROGRAM;
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_cfg_function(0, maximum - 1, &function, &mut exact_errors),
        Some((1, maximum))
    );
    assert!(exact_errors.finish().is_empty());
    let mut extra_errors = Errors::new(&sources);
    assert_eq!(accumulate_generated_cfg_function(0, maximum, &function, &mut extra_errors), None);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn generated_cfg_composition_accepts_multiple_lowered_functions() {
    let (text, raw) = private_string_call_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful multi-function v4");
    let program = lower(pair_input(&syntax, &sources)).expect("multi-function generated CFG");
    let functions = program
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    assert!(functions.len() >= 2);
    assert!(functions.iter().all(|function| function.blocks().len() == 1));
}

#[test]
fn owned_cfg_rejects_duplicate_terminator_and_emission_after_termination() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let terminator = raw::SpannedTerminator {
        span: at,
        kind: raw::Terminator::Return { value: raw::ValueId(0), cleanup: raw::CleanupPlanId(0) },
    };
    assert!(cfg.terminate(terminator.clone(), &mut errors));
    assert!(!cfg.terminate(terminator, &mut errors));
    assert!(!cfg.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("one terminated block");
    assert!(blocks[0].instructions.is_empty());
    assert_eq!(blocks[0].terminators.len(), 1);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.code() == "ZRYNA-M3015" && diagnostic.primary_span() == Some(at)
    }));
}

#[test]
fn terminal_owned_if_skeleton_preflight_is_exact_and_atomic() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("terminal if span");

    let mut exact_errors = Errors::new(&sources);
    let exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    let exact_before =
        (exact.arena.blocks.len(), exact.incoming.clone(), exact.edges, exact.transitions);
    assert!(exact.preflight_skeleton(
        zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION,
        at,
        &mut exact_errors,
    ));
    assert_eq!(
        (exact.arena.blocks.len(), exact.incoming.clone(), exact.edges, exact.transitions,),
        exact_before
    );
    assert!(exact_errors.finish().is_empty());

    for (blocks, edges) in [
        (zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION, 0),
        (0, zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION + 1),
        (usize::MAX, usize::MAX),
    ] {
        let mut errors = Errors::new(&sources);
        let state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        let before =
            (state.arena.blocks.len(), state.incoming.clone(), state.edges, state.transitions);
        assert!(!state.preflight_skeleton(blocks, edges, at, &mut errors));
        assert_eq!(
            (state.arena.blocks.len(), state.incoming.clone(), state.edges, state.transitions,),
            before
        );
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_loop_three_block_four_edge_preflight_is_exact_plus_one_and_atomic() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("loop span");
    let maximum_blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION;
    let maximum_edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    exact.arena.blocks.resize_with(maximum_blocks - 3, || super::OwnedPendingBlock {
        populated: false,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: None,
    });
    exact.incoming.resize(maximum_blocks - 3, 0);
    exact.edges = maximum_edges - 4;
    let before = (exact.arena.blocks.len(), exact.incoming.len(), exact.edges);
    assert!(exact.preflight_skeleton(3, 4, at, &mut exact_errors));
    assert_eq!((exact.arena.blocks.len(), exact.incoming.len(), exact.edges), before);
    assert!(exact_errors.finish().is_empty());

    for (blocks, edges) in [(4, 4), (3, 5), (usize::MAX, usize::MAX)] {
        let mut errors = Errors::new(&sources);
        let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        state.arena.blocks.resize_with(maximum_blocks - 3, || super::OwnedPendingBlock {
            populated: false,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
        });
        state.incoming.resize(maximum_blocks - 3, 0);
        state.edges = maximum_edges - 4;
        let before = (state.arena.blocks.len(), state.incoming.len(), state.edges);
        assert!(!state.preflight_skeleton(blocks, edges, at, &mut errors));
        assert_eq!((state.arena.blocks.len(), state.incoming.len(), state.edges), before);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }

    let mut errors = Errors::new(&sources);
    let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    state.arena.blocks.resize_with(maximum_blocks - 2, || super::OwnedPendingBlock {
        populated: false,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: None,
    });
    state.incoming.resize(maximum_blocks - 2, 0);
    state.edges = maximum_edges - 4;
    let mut known = std::collections::BTreeMap::from([(raw::PlaceId(7), Some(6))]);
    let before = known.clone();
    assert!(!super::preflight_owned_string_loop_skeleton(
        &state,
        &mut known,
        true,
        at,
        &mut errors,
    ));
    assert_eq!(known, before);
}

#[test]
fn owned_loop_commit_transition_reservation_is_exact_plus_one_and_releasable() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("mutation span");
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    exact.transitions = maximum - 1;
    assert!(super::reserve_owned_commit_transition(&mut exact, at, &mut exact_errors));
    assert_eq!(exact.transitions, maximum - 1);
    assert_eq!(exact.reserved_transitions, 1);
    super::release_owned_commit_transition(&mut exact);
    assert_eq!(exact.reserved_transitions, 0);
    assert!(exact_errors.finish().is_empty());

    let mut read_cleanup_errors = Errors::new(&sources);
    let mut read_cleanup =
        OwnedCfgState::single_block(at, &mut read_cleanup_errors).expect("entry block");
    read_cleanup.transitions = maximum - 2;
    assert!(super::reserve_owned_commit_transitions(
        &mut read_cleanup,
        2,
        at,
        &mut read_cleanup_errors,
    ));
    assert_eq!(read_cleanup.reserved_transitions, 2);
    super::release_owned_commit_transitions(&mut read_cleanup, 2);
    assert_eq!(read_cleanup.reserved_transitions, 0);
    assert!(read_cleanup_errors.finish().is_empty());

    let mut first_extra_errors = Errors::new(&sources);
    let mut first_extra =
        OwnedCfgState::single_block(at, &mut first_extra_errors).expect("entry block");
    first_extra.transitions = maximum - 1;
    let before = (first_extra.transitions, first_extra.reserved_transitions);
    assert!(!super::reserve_owned_commit_transitions(
        &mut first_extra,
        2,
        at,
        &mut first_extra_errors,
    ));
    assert_eq!((first_extra.transitions, first_extra.reserved_transitions), before);
    assert_eq!(first_extra_errors.finish()[0].code(), "ZRYNA-M3201");

    for transitions in [maximum, usize::MAX] {
        let mut errors = Errors::new(&sources);
        let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        state.transitions = transitions;
        let before = (state.transitions, state.reserved_transitions);
        assert!(!super::reserve_owned_commit_transition(&mut state, at, &mut errors));
        assert_eq!((state.transitions, state.reserved_transitions), before);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_loop_drop_action_reservation_is_exact_plus_one_overflow_and_releasable() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.span);
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut exact_errors,
        at,
        maximum - 1,
    );
    assert!(exact.reserve_loop_drop_actions(1, at));
    assert_eq!((exact.cleanup_actions, exact.reserved_cleanup_actions), (maximum - 1, 1));
    exact.release_loop_drop_actions(1);
    assert_eq!((exact.cleanup_actions, exact.reserved_cleanup_actions), (maximum - 1, 0));
    drop(exact);
    assert!(exact_errors.finish().is_empty());

    for current in [maximum, usize::MAX] {
        let mut errors = Errors::new(&sources);
        let mut lowerer = private_string_branch_budget_lowerer(
            input,
            function,
            ty,
            &catalog,
            &mut errors,
            at,
            current,
        );
        let before = (lowerer.cleanup_actions, lowerer.reserved_cleanup_actions);
        assert!(!lowerer.reserve_loop_drop_actions(1, at));
        assert_eq!((lowerer.cleanup_actions, lowerer.reserved_cleanup_actions), before);
        drop(lowerer);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_cfg_enforces_each_storage_limit_at_the_emission_site() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut block_errors = Errors::new(&sources);
    let mut blocks = OwnedCfgState::single_block(at, &mut block_errors).expect("entry block");
    for _ in 1..zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION {
        blocks.reserve_block(at, &mut block_errors).expect("exact block budget");
    }
    assert!(blocks.reserve_block(at, &mut block_errors).is_none());

    let mut edge_errors = Errors::new(&sources);
    let mut edges = OwnedCfgState::single_block(at, &mut edge_errors).expect("entry block");
    let successor = edges.reserve_block(at, &mut edge_errors).expect("reserved successor");
    edges.edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;
    assert!(!edges.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut edge_errors,
    ));

    let mut transition_errors = Errors::new(&sources);
    let mut transitions =
        OwnedCfgState::single_block(at, &mut transition_errors).expect("entry block");
    transitions.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!transitions.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut transition_errors,
    ));

    for errors in [block_errors, edge_errors, transition_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_cfg_reserves_then_populates_a_canonical_multiblock_skeleton() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let left = cfg.reserve_block(at, &mut errors).expect("left reservation");
    let join = cfg.reserve_block(at, &mut errors).expect("join reservation");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Branch {
                condition: raw::ValueId(0),
                when_true: raw::Edge { target: left, arguments: Vec::new() },
                when_false: raw::Edge { target: join, arguments: Vec::new() },
            },
        },
        &mut errors,
    ));
    cfg.begin_block(left, Vec::new(), at, &mut errors).expect("left block");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: join, arguments: Vec::new() }),
        },
        &mut errors,
    ));
    cfg.begin_block(join, Vec::new(), at, &mut errors).expect("join block");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("complete skeleton");
    assert_eq!(blocks.iter().map(|block| block.id.0).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(errors.finish().is_empty());
}

#[test]
fn owned_cfg_finish_rejects_unterminated_and_unpopulated_blocks_with_m3015() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut unterminated_errors = Errors::new(&sources);
    let unterminated =
        OwnedCfgState::single_block(at, &mut unterminated_errors).expect("entry block");
    assert!(unterminated.finish(at, &mut unterminated_errors).is_none());

    let mut unpopulated_errors = Errors::new(&sources);
    let mut unpopulated =
        OwnedCfgState::single_block(at, &mut unpopulated_errors).expect("entry block");
    unpopulated.reserve_block(at, &mut unpopulated_errors).expect("successor reservation");
    assert!(unpopulated.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut unpopulated_errors,
    ));
    assert!(unpopulated.finish(at, &mut unpopulated_errors).is_none());

    for errors in [unterminated_errors, unpopulated_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_cfg_rejects_invalid_targets_and_switch_before_termination() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    for target in [raw::BlockId(0), raw::BlockId(u32::MAX)] {
        let mut errors = Errors::new(&sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        assert!(!cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target, arguments: Vec::new() }),
            },
            &mut errors,
        ));
        assert_eq!(errors.finish()[0].code(), "ZRYNA-M3015");
    }

    let mut current_errors = Errors::new(&sources);
    let mut invalid_current =
        OwnedCfgState::single_block(at, &mut current_errors).expect("entry block");
    invalid_current.current = None;
    assert!(!invalid_current.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut current_errors,
    ));
    assert_eq!(current_errors.finish()[0].code(), "ZRYNA-M3015");

    let mut switch_errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut switch_errors).expect("entry block");
    let successor = cfg.reserve_block(at, &mut switch_errors).expect("successor");
    assert!(cfg.begin_block(successor, Vec::new(), at, &mut switch_errors).is_none());
    assert_eq!(switch_errors.finish()[0].code(), "ZRYNA-M3015");
}

#[test]
fn owned_cfg_reservation_preserves_dense_global_value_definition_order() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("parameter-seeded entry");
    cfg.seed_function_parameter(
        &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
        &mut errors,
    )
    .expect("function parameter");
    let successor = cfg.reserve_block(at, &mut errors).expect("identity-only reservation");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(1),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::StringFromUtf8 {
                bytes: vec![b'x'],
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge {
                target: successor,
                arguments: vec![raw::ValueId(1)],
            }),
        },
        &mut errors,
    ));
    cfg.begin_block(
        successor,
        vec![raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span: at }],
        at,
        &mut errors,
    )
    .expect("successor parameter");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(3),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut errors,
    ));
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(3),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("complete value-ordered CFG");
    assert_eq!(blocks[0].instructions[0].result.as_ref().expect("entry value").id.0, 1);
    assert_eq!(blocks[1].parameters[0].id.0, 2);
    assert_eq!(blocks[1].instructions[0].result.as_ref().expect("successor value").id.0, 3);
    assert!(errors.finish().is_empty());
}

#[test]
fn owned_cfg_failed_value_mutations_preserve_state_and_close_parameter_seeding() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let successor = cfg.reserve_block(at, &mut errors).expect("successor");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut errors,
    ));
    let before = (cfg.current, cfg.value_types.clone(), cfg.arena.blocks[1].populated);
    assert!(
        cfg.begin_block(
            successor,
            vec![
                raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
                raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span: at },
            ],
            at,
            &mut errors,
        )
        .is_none()
    );
    assert_eq!((cfg.current, cfg.value_types.clone(), cfg.arena.blocks[1].populated), before);

    let mut transition_errors = Errors::new(&sources);
    let mut transitions =
        OwnedCfgState::single_block(at, &mut transition_errors).expect("entry block");
    transitions.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let before = (
        transitions.value_types.clone(),
        transitions.arena.blocks[0].instructions.len(),
        transitions.function_parameters_open,
    );
    assert!(!transitions.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(0),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut transition_errors,
    ));
    assert_eq!(
        (
            transitions.value_types.clone(),
            transitions.arena.blocks[0].instructions.len(),
            transitions.function_parameters_open,
        ),
        before
    );

    let mut late_errors = Errors::new(&sources);
    let mut late = OwnedCfgState::single_block(at, &mut late_errors).expect("entry block");
    assert!(late.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut late_errors,
    ));
    assert!(
        late.seed_function_parameter(
            &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
            &mut late_errors,
        )
        .is_none()
    );
    assert_eq!(late_errors.finish()[0].code(), "ZRYNA-M3015");
}

#[test]
fn owned_cfg_finish_rejects_disconnected_cycles_and_edge_signature_mismatch() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut cycle_errors = Errors::new(&sources);
    let mut cycle = OwnedCfgState::single_block(at, &mut cycle_errors).expect("entry block");
    let left = cycle.reserve_block(at, &mut cycle_errors).expect("left");
    let right = cycle.reserve_block(at, &mut cycle_errors).expect("right");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut cycle_errors,
    ));
    cycle.begin_block(left, Vec::new(), at, &mut cycle_errors).expect("left block");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: right, arguments: Vec::new() }),
        },
        &mut cycle_errors,
    ));
    cycle.begin_block(right, Vec::new(), at, &mut cycle_errors).expect("right block");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: left, arguments: Vec::new() }),
        },
        &mut cycle_errors,
    ));
    assert!(cycle.finish(at, &mut cycle_errors).is_none());
    assert_eq!(cycle_errors.finish()[0].code(), "ZRYNA-M3015");

    let mut signature_errors = Errors::new(&sources);
    let mut signature =
        OwnedCfgState::single_block(at, &mut signature_errors).expect("entry block");
    signature
        .seed_function_parameter(
            &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
            &mut signature_errors,
        )
        .expect("function parameter");
    let target = signature.reserve_block(at, &mut signature_errors).expect("target");
    assert!(signature.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target, arguments: vec![raw::ValueId(0)] }),
        },
        &mut signature_errors,
    ));
    signature
        .begin_block(
            target,
            vec![raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span: at }],
            at,
            &mut signature_errors,
        )
        .expect("target signature");
    assert!(signature.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut signature_errors,
    ));
    assert!(signature.finish(at, &mut signature_errors).is_none());
    assert_eq!(signature_errors.finish()[0].code(), "ZRYNA-M3015");
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

#[test]
fn exhaustive_enum_match_lowers_to_refined_cfg() {
    let sources = sources_for(ENUM_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ENUM_RESPONSE), &sources)
        .expect("source-faithful enum v4");
    let program = lower(pair_input(&syntax, &sources)).expect("exhaustive match must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.blocks().len(), 3);
    assert!(
        function
            .places()
            .any(|place| matches!(place.kind(), VerifiedPlaceKind::EnumPayload { variant: 1, .. }))
    );
}

#[test]
fn nonexhaustive_enum_match_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..137, "                       ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[2].kind
    else {
        panic!("match")
    };
    arms.remove(0);
    arms[0].value = 1;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 2;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nonexhaustive match");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nonexhaustive match");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn v4_match_child_order_regression_authenticates_source_faithful_adapter_output() {
    let sources = sources_for(ENUM_SOURCE);
    verify_snapshot(response_snapshot(ENUM_RESPONSE), &sources)
        .expect("call-style match close parenthesis follows the complete arm object");
}

#[test]
fn fixed_array_constant_projection_accepts_last_index() {
    let sources = sources_for(ARRAY_VALID_SOURCE);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index literal")
    };
    *spelling = "1".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful fixed array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("last fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 1, .. }
    )));
}

#[test]
fn fixed_array_constant_projection_accepts_zero_index() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "0".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful zero index");
    let program = lower(pair_input(&syntax, &sources)).expect("zero fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        function.parameters().len()
            + block.parameters().len()
            + block.instructions().filter(|instruction| instruction.result().is_some()).count(),
        2,
        "fixed-array constant index spelling is not emitted as a runtime value",
    );
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 0, .. }
    )));
}

#[test]
fn fixed_array_constructor_requires_exact_count_and_element_type() {
    let sources = sources_for(ARRAY_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), &sources)
        .expect("array constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact fixed-array constructor");

    let mut missing_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    missing_source.replace_range(66..69, "   ");
    let missing_sources = sources_for(&missing_source);
    let mut missing = response_snapshot(ARRAY_CONSTRUCT_RESPONSE);
    let body = &mut missing.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[1].kind
    else {
        panic!("array constructor")
    };
    elements.pop();
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 1;
    let syntax = verify_snapshot(missing, &missing_sources).expect("source-faithful short array");
    assert_eq!(
        lower(pair_input(&syntax, &missing_sources)).expect_err("short array")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(65..66, "true");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), 66, 3);
    typed.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true };
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped array");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped array");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped element child");
    assert_eq!((primary.start(), primary.end()), (65, 69));
}

#[test]
fn fixed_array_index_equal_to_length_is_rejected() {
    let sources = sources_for(ARRAY_OOB_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_RESPONSE), &sources)
        .expect("source-faithful fixed array v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("index N is out of bounds");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn struct_unknown_and_missing_field_is_rejected_at_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..141, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("left reference")
    };
    name.text = "nope".to_owned();
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("shorthand")
    };
    name.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown/missing field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    assert_eq!(diagnostics[0].primary_span().expect("initializer span").start(), 137);
}

#[test]
fn duplicate_struct_field_is_rejected_at_later_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(143..148, "left ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("right reference")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    expressions[1].span.end = 147;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    fields[1].span.end = 147;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("shorthand")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    let diagnostics = verify_snapshot(raw, &sources)
        .expect_err("v4 rejects duplicate initializer names before semantics");
    assert_eq!(diagnostics[0].code(), "ZRYNA-Y4002");
}

#[test]
fn enum_payload_binding_mismatch_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(152..157, "     ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("match")
    };
    arms[1].binding = None;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful payload mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("payload binding mismatch");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn reversed_enum_match_arms_lower_in_variant_ordinal_order() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..167, "\"Maybe.some\": (value) => value, \"Maybe.none\": () => 0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "value".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
            },
        },
    };
    body.expressions[2] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 166, end: 167 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[3].kind
    else {
        panic!("match")
    };
    arms.swap(0, 1);
    let some = &mut arms[0];
    some.span = zryna_source::UntrustedSpan { file: 0, start: 114, end: 144 };
    some.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 115, end: 120 };
    some.dot_span = zryna_source::UntrustedSpan { file: 0, start: 120, end: 121 };
    some.variant.span = zryna_source::UntrustedSpan { file: 0, start: 121, end: 125 };
    let binding = some.binding.as_mut().expect("some binding");
    binding.span = zryna_source::UntrustedSpan { file: 0, start: 129, end: 134 };
    some.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 136, end: 138 };
    some.value = 1;
    let none = &mut arms[1];
    none.span = zryna_source::UntrustedSpan { file: 0, start: 146, end: 167 };
    none.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 147, end: 152 };
    none.dot_span = zryna_source::UntrustedSpan { file: 0, start: 152, end: 153 };
    none.variant.span = zryna_source::UntrustedSpan { file: 0, start: 153, end: 157 };
    none.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 163, end: 165 };
    none.value = 2;

    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed match");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed match must lower");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let entry = function.blocks().next().expect("entry");
    assert_eq!(
        entry.terminator().enum_arms().map(|arm| arm.variant()).collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn wrong_case_value_references_are_not_aliases() {
    let mut parameter_source = ARRAY_OOB_SOURCE.to_owned();
    parameter_source.replace_range(51..53, "XS");
    let parameter_sources = sources_for(&parameter_source);
    let mut parameter = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut parameter.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("parameter reference")
    };
    name.text = "XS".to_owned();
    let syntax = verify_snapshot(parameter, &parameter_sources).expect("wrong-case parameter v4");
    let diagnostics =
        lower(pair_input(&syntax, &parameter_sources)).expect_err("wrong-case parameter");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("parameter use").start(), 51);

    let mut local_source = PAIR_SCORE_SOURCE.to_owned();
    local_source.replace_range(160..164, "PAIR");
    let local_sources = sources_for(&local_source);
    let mut local = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut local.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("local reference")
    };
    name.text = "PAIR".to_owned();
    let syntax = verify_snapshot(local, &local_sources).expect("wrong-case local v4");
    let diagnostics = lower(pair_input(&syntax, &local_sources)).expect_err("wrong-case local");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("local use").start(), 160);

    let mut scrutinee_source = ENUM_SOURCE.to_owned();
    scrutinee_source.replace_range(109..110, "X");
    let scrutinee_sources = sources_for(&scrutinee_source);
    let mut scrutinee = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut scrutinee.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("scrutinee")
    };
    name.text = "X".to_owned();
    let syntax = verify_snapshot(scrutinee, &scrutinee_sources).expect("wrong-case scrutinee v4");
    let diagnostics =
        lower(pair_input(&syntax, &scrutinee_sources)).expect_err("wrong-case scrutinee");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("scrutinee use").start(), 109);

    let mut payload_source = ENUM_SOURCE.to_owned();
    payload_source.replace_range(162..167, "VALUE");
    let payload_sources = sources_for(&payload_source);
    let mut payload = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut payload.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("payload")
    };
    name.text = "VALUE".to_owned();
    let syntax = verify_snapshot(payload, &payload_sources).expect("wrong-case payload v4");
    let diagnostics = lower(pair_input(&syntax, &payload_sources)).expect_err("wrong-case payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("payload use").start(), 162);
}

#[test]
fn enum_constructor_payload_shape_type_and_name_are_exact() {
    let sources = sources_for(ENUM_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), &sources)
        .expect("enum constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact enum constructor");

    let mut absent_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    absent_source.replace_range(115..116, " ");
    let absent_sources = sources_for(&absent_source);
    let mut absent = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let body = &mut absent.files[0].functions[0].body;
    body.expressions.remove(0);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { payload, .. } =
        &mut body.expressions[0].kind
    else {
        panic!("enum constructor")
    };
    *payload = None;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 0;
    let syntax = verify_snapshot(absent, &absent_sources).expect("source-faithful absent payload");
    assert_eq!(
        lower(pair_input(&syntax, &absent_sources)).expect_err("absent payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut extra_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    extra_source.replace_range(110..114, "none");
    let extra_sources = sources_for(&extra_source);
    let mut extra = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut extra.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "none".to_owned();
    let syntax = verify_snapshot(extra, &extra_sources).expect("source-faithful extra payload");
    assert_eq!(
        lower(pair_input(&syntax, &extra_sources)).expect_err("extra payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(83..86, "bool");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), 86, 1);
    let RawTypeSyntaxKind::Named { name } = &mut typed.files[0].type_syntax[1].kind else {
        panic!("parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped payload");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped payload child");
    assert_eq!((primary.start(), primary.end()), (116, 117));

    let mut variant_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    variant_source.replace_range(110..114, "nope");
    let variant_sources = sources_for(&variant_source);
    let mut variant_snapshot = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut variant_snapshot.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "nope".to_owned();
    let syntax = verify_snapshot(variant_snapshot, &variant_sources)
        .expect("source-faithful unknown variant");
    let diagnostics = lower(pair_input(&syntax, &variant_sources)).expect_err("unknown variant");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    let primary = diagnostics[0].primary_span().expect("variant token");
    assert_eq!((primary.start(), primary.end()), (110, 114));

    let mut unknown_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    unknown_source.replace_range(104..109, "Other");
    let unknown_sources = sources_for(&unknown_source);
    let mut unknown = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { type_name, .. } =
        &mut unknown.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    type_name.text = "Other".to_owned();
    let syntax = verify_snapshot(unknown, &unknown_sources).expect("source-faithful unknown enum");
    assert_eq!(
        lower(pair_input(&syntax, &unknown_sources)).expect_err("unknown enum")[0].code(),
        "ZRYNA-M3005"
    );
}

#[test]
fn dynamic_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "x");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1].kind =
        zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "x".to_owned(), span },
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful dynamic index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("dynamic index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("dynamic index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn negative_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "-1");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 55, 1);
    let body = &mut raw.files[0].functions[0].body;
    let mut index = body.expressions.pop().expect("index expression");
    let literal = &mut body.expressions[1];
    literal.span.start = 55;
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } = &mut literal.kind else {
        panic!("literal")
    };
    *spelling = "1".to_owned();
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 56 },
        kind: zryna_syntax::v4::RawExpressionKind::Negation {
            operator_span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 55 },
            operand: 1,
        },
    });
    let zryna_syntax::v4::RawExpressionKind::Index { index: index_id, .. } = &mut index.kind else {
        panic!("index")
    };
    *index_id = 2;
    body.expressions.push(index);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful negative index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("negative index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("negative index child");
    assert_eq!((primary.start(), primary.end()), (54, 56));
}

#[test]
fn unresolved_aggregate_element_type_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(28..31, "Foo");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[0].kind else {
        panic!("named element")
    };
    name.text = "Foo".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved type");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved type");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
}

#[test]
fn unresolved_value_name_is_rejected_at_its_use() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(160..164, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("pair reference")
    };
    name.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved value");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved value");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("name use").start(), 160);
}

#[test]
fn unknown_field_access_is_rejected_at_the_use_not_declaration() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(165..169, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { field, .. } =
        &mut raw.files[0].functions[0].body.expressions[4].kind
    else {
        panic!("field access")
    };
    field.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field access");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    assert_eq!(diagnostics[0].primary_span().expect("field use").start(), 165);
}

#[test]
fn by_value_recursive_struct_is_rejected_by_layout_authority() {
    let mut source = PAIR_SOURCE.to_owned();
    source.replace_range(43..46, "Pair");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 46, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[0].kind else {
        panic!("field type")
    };
    name.text = "Pair".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("by-value recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn fixed_array_mediated_recursive_struct_is_rejected_by_layout_authority() {
    let text = "interface Loop extends ZrynaStruct { items: FixedArray<Loop, 1>; }\n";
    let sources = sources_for(text);
    let span = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        diagnostics: Vec::new(),
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                RawTypeSyntax {
                    span: span(55, 59),
                    kind: RawTypeSyntaxKind::Named {
                        name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(55, 59) },
                    },
                },
                RawTypeSyntax {
                    span: span(44, 63),
                    kind: RawTypeSyntaxKind::FixedArray {
                        keyword_span: span(44, 54),
                        less_than_span: span(54, 55),
                        element: 0,
                        comma_span: span(59, 60),
                        length_span: span(61, 62),
                        length_spelling: "1".to_owned(),
                        length: 1,
                        greater_than_span: span(62, 63),
                    },
                },
            ],
            data_declarations: vec![RawDataDeclaration {
                span: span(0, 66),
                export_span: None,
                kind: RawDataDeclarationKind::Struct {
                    interface_span: span(0, 9),
                    name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(10, 14) },
                    extends_span: span(15, 22),
                    marker_span: span(23, 34),
                    open_brace_span: span(35, 36),
                    fields: vec![RawDataField {
                        span: span(37, 64),
                        name: RawIdentifierSyntax { text: "items".to_owned(), span: span(37, 42) },
                        colon_span: span(42, 43),
                        type_syntax: 1,
                        semicolon_span: span(63, 64),
                    }],
                    close_brace_span: span(65, 66),
                },
            }],
            functions: Vec::new(),
        }],
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array recursion");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("array-mediated recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn public_aggregate_result_is_rejected() {
    let mut source = PAIR_SOURCE.to_owned();
    source.insert_str(50, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 50, 7);
    let function = &mut raw.files[0].functions[0];
    function.span.start = 50;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 50, end: 56 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate result");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn public_aggregate_parameter_is_rejected() {
    let mut source = ARRAY_VALID_SOURCE.to_owned();
    source.insert_str(0, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 0, 7);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "1".to_owned();
    let function = &mut raw.files[0].functions[0];
    function.span.start = 0;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 0, end: 6 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported array parameter");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate parameter");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn mistyped_struct_initializer_is_rejected_at_the_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(99..102, "bool");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 102, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[3].kind else {
        panic!("right parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful mistyped Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mistyped initializer");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    assert!(diagnostics[0].primary_span().is_some());
}

#[test]
fn portable_field_name_collision_is_rejected() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(48..53, "LEFT ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Pair struct")
    };
    fields[1].name.text = "LEFT".to_owned();
    fields[1].name.span.end = 52;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful case collision");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("portable collision");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("later field").start(), 48);
}

#[test]
fn semantic_diagnostics_replay_deterministically() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("first rejection");
    let second = lower(pair_input(&syntax, &sources)).expect_err("second rejection");
    let summarize = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|d| {
                (
                    d.code().to_owned(),
                    d.primary_span().map(|s| (s.start(), s.end())),
                    d.message().to_owned(),
                    d.guidance().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(summarize(&first), summarize(&second));
}

#[test]
fn authenticated_owned_types_have_canonical_sealed_capabilities() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let input = pair_input(&syntax, &sources);
    let string = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let first_vec = authenticated_type_capabilities(input, 0, 4).expect("first Vec<String>");
    let second_vec = authenticated_type_capabilities(input, 0, 6).expect("second Vec<String>");
    let owned_box = authenticated_type_capabilities(input, 0, 7).expect("owned struct");
    let recursive_node = authenticated_type_capabilities(input, 0, 8).expect("recursive Vec node");
    let constructed_vec =
        authenticated_type_capabilities(input, 0, 13).expect("Vec construction type");

    assert_eq!(first_vec.layout, second_vec.layout);
    assert_eq!(first_vec.layout, constructed_vec.layout);
    for ty in [string, first_vec, owned_box, recursive_node] {
        assert!(!ty.is_copy());
        assert!(ty.is_clone());
        assert_ne!(ty.drop_kind, 0);
    }
    assert_ne!(string.runtime_kind, 0);
    assert_ne!(first_vec.runtime_kind, 0);
}

#[test]
fn unsupported_owned_vec_shape_uses_vec_diagnostic_family() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unsupported Vec shape");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
}

#[test]
fn duplicate_return_is_rejected_at_the_second_return() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "return 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 10);
    let body = &mut raw.files[0].functions[0].body;
    let value = u32::try_from(body.expressions.len()).expect("expression id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 196, end: 197 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 195 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 197, end: 198 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful duplicate return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("second return").start(), 189);
}

#[test]
fn mutation_after_return_is_rejected_before_lowering_the_mutation() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "pair.left = 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 15);
    let body = &mut raw.files[0].functions[0].body;
    let reference = u32::try_from(body.expressions.len()).expect("reference id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "pair".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
            },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("target id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: reference,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: 193, end: 194 },
            field: RawIdentifierSyntax {
                text: "left".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 194, end: 198 },
            },
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("value id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 201, end: 202 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 203 },
        kind: RawStatementKind::Assignment {
            target,
            equals_span: zryna_source::UntrustedSpan { file: 0, start: 199, end: 200 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 202, end: 203 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful post-return mutation");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mutation after return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("mutation span").start(), 189);
}

fn empty_declaration_file(file: u32, first: usize, count: usize) -> (String, RawSourceUnit) {
    let mut text = String::new();
    let mut declarations = Vec::with_capacity(count);
    let mut types = Vec::with_capacity(count);
    for number in first..first + count {
        let name = format!("T{number}");
        let start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str("interface ");
        let name_start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(&name);
        let name_end = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(" extends ZrynaStruct { x: i32; }\n");
        let end = u32::try_from(text.len() - 1).expect("fixture offset");
        let extends_start = name_end + 1;
        let marker_start = extends_start + 8;
        let open = marker_start + 12;
        let field_start = open + 2;
        let colon = field_start + 1;
        let type_start = colon + 2;
        let semicolon = type_start + 3;
        let type_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file, start: type_start, end: type_start + 3 },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: type_start,
                        end: type_start + 3,
                    },
                },
            },
        });
        declarations.push(RawDataDeclaration {
            span: zryna_source::UntrustedSpan { file, start, end },
            export_span: None,
            kind: RawDataDeclarationKind::Struct {
                interface_span: zryna_source::UntrustedSpan { file, start, end: start + 9 },
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file, start: name_start, end: name_end },
                },
                extends_span: zryna_source::UntrustedSpan {
                    file,
                    start: extends_start,
                    end: extends_start + 7,
                },
                marker_span: zryna_source::UntrustedSpan {
                    file,
                    start: marker_start,
                    end: marker_start + 11,
                },
                open_brace_span: zryna_source::UntrustedSpan { file, start: open, end: open + 1 },
                fields: vec![RawDataField {
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: field_start,
                        end: semicolon + 1,
                    },
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file,
                            start: field_start,
                            end: field_start + 1,
                        },
                    },
                    colon_span: zryna_source::UntrustedSpan { file, start: colon, end: colon + 1 },
                    type_syntax: type_id,
                    semicolon_span: zryna_source::UntrustedSpan {
                        file,
                        start: semicolon,
                        end: semicolon + 1,
                    },
                }],
                close_brace_span: zryna_source::UntrustedSpan {
                    file,
                    start: semicolon + 2,
                    end: semicolon + 3,
                },
            },
        });
    }
    let path = if file == 0 { "src/a.zry" } else { "src/b.zry" };
    (
        text,
        RawSourceUnit {
            id: file,
            path: path.to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: declarations,
            functions: Vec::new(),
        },
    )
}

#[test]
fn nominal_declaration_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let exact_sources =
        SourceMap::build(vec![SourceFileInput { path: "src/a.zry".to_owned(), text: exact_text }])
            .expect("exact source map");
    let exact_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![exact_file],
            diagnostics: Vec::new(),
        },
        &exact_sources,
    )
    .expect("exact v4 budget");
    let entry_path = NormalizedSourcePath::new("src/a.zry").expect("entry path");
    let entry = exact_sources.file_id(&entry_path).expect("entry");
    lower(SemanticInput::try_new(&exact_syntax, &exact_sources, entry).expect("exact input"))
        .expect("the exact nominal declaration budget must verify");

    let (first_text, first_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let (last_text, last_file) =
        empty_declaration_file(1, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS, 1);
    let plus_sources = SourceMap::build(vec![
        SourceFileInput { path: "src/a.zry".to_owned(), text: first_text },
        SourceFileInput { path: "src/b.zry".to_owned(), text: last_text },
    ])
    .expect("plus-one source map");
    let plus_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![first_file, last_file],
            diagnostics: Vec::new(),
        },
        &plus_sources,
    )
    .expect("plus-one v4 budget");
    let entry = plus_sources.file_id(&entry_path).expect("entry");
    let plus =
        lower(SemanticInput::try_new(&plus_syntax, &plus_sources, entry).expect("plus-one input"))
            .expect_err("M3 nominal limit must fail");
    assert_eq!(plus[0].code(), "ZRYNA-M3201");
}

fn derived_value_fixture(result_count: usize) -> RawFunctionSyntax {
    assert!(result_count > 0);
    let span = zryna_source::UntrustedSpan { file: 0, start: 0, end: 1 };
    let child_count = result_count - 1;
    let mut expressions = (0..child_count)
        .map(|_| RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
        })
        .collect::<Vec<_>>();
    expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: 0,
            open_paren_span: span,
            open_bracket_span: span,
            elements: (0..u32::try_from(child_count).expect("fixture expression ids")).collect(),
            close_bracket_span: span,
            close_paren_span: span,
        },
    });
    RawFunctionSyntax {
        span,
        export_span: None,
        function_span: span,
        name: RawIdentifierSyntax { text: "budget".to_owned(), span },
        parameters: Vec::new(),
        result_type: 0,
        body: RawFunctionBodySyntax {
            span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span,
                open_brace_span: span,
                statements: vec![0],
                close_brace_span: span,
            }],
            statements: vec![RawStatementSyntax {
                span,
                kind: RawStatementKind::Return {
                    keyword_span: span,
                    value: u32::try_from(child_count).expect("fixture root id"),
                    semicolon_span: span,
                },
            }],
            expressions,
        },
    }
}

fn nested_control_value_fixture(result_count: usize) -> RawFunctionSyntax {
    assert!(result_count >= 3);
    let mut function = derived_value_fixture(result_count - 2);
    let span = function.span;
    let result = u32::try_from(function.body.expressions.len() - 1).expect("result expression id");
    let if_condition = u32::try_from(function.body.expressions.len()).expect("if condition id");
    function.body.expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true },
    });
    let while_condition =
        u32::try_from(function.body.expressions.len()).expect("while condition id");
    function.body.expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
    });
    function.body.blocks = vec![
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: vec![0, 3],
            close_brace_span: span,
        },
        RawBlockSyntax { span, open_brace_span: span, statements: vec![1], close_brace_span: span },
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: Vec::new(),
            close_brace_span: span,
        },
        RawBlockSyntax { span, open_brace_span: span, statements: vec![2], close_brace_span: span },
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: Vec::new(),
            close_brace_span: span,
        },
    ];
    function.body.statements = vec![
        RawStatementSyntax { span, kind: RawStatementKind::Block { block: 1 } },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::If {
                keyword_span: span,
                open_paren_span: span,
                condition: if_condition,
                close_paren_span: span,
                then_block: 2,
                else_clause: Some(zryna_syntax::v4::RawElseSyntax { keyword_span: span, block: 3 }),
            },
        },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::While {
                keyword_span: span,
                open_paren_span: span,
                condition: while_condition,
                close_paren_span: span,
                body_block: 4,
            },
        },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::Return {
                keyword_span: span,
                value: result,
                semicolon_span: span,
            },
        },
    ];
    function
}

#[allow(clippy::too_many_lines)]
fn authenticated_value_budget_fixture(with_parameter: bool) -> (String, RawProjectSyntaxSnapshot) {
    fn offset(text: &str) -> u32 {
        u32::try_from(text.len()).expect("fixture offset")
    }
    fn fixed_array_type(text: &mut String, types: &mut Vec<RawTypeSyntax>, length: usize) -> u32 {
        let start = offset(text);
        text.push_str("FixedArray<");
        let element_start = offset(text);
        text.push_str("i32");
        let element_end = offset(text);
        let element = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: element_start, end: element_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: element_start,
                        end: element_end,
                    },
                },
            },
        });
        let comma = offset(text);
        text.push_str(", ");
        let length_start = offset(text);
        let spelling = length.to_string();
        text.push_str(&spelling);
        let length_end = offset(text);
        let greater = offset(text);
        text.push('>');
        let end = offset(text);
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start, end },
            kind: RawTypeSyntaxKind::FixedArray {
                keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 10 },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 10,
                    end: start + 11,
                },
                element,
                comma_span: zryna_source::UntrustedSpan { file: 0, start: comma, end: comma + 1 },
                length_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: length_start,
                    end: length_end,
                },
                length: u32::try_from(length).expect("array length"),
                length_spelling: spelling,
                greater_than_span: zryna_source::UntrustedSpan { file: 0, start: greater, end },
            },
        });
        id
    }

    let mut text = "function budget(".to_owned();
    let mut types = Vec::new();
    let mut parameters = Vec::new();
    if with_parameter {
        let parameter_start = offset(&text);
        text.push_str("x: ");
        let type_start = offset(&text);
        text.push_str("i32");
        let type_end = offset(&text);
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
                },
            },
        });
        parameters.push(RawParameterSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: parameter_start, end: type_end },
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: parameter_start,
                    end: parameter_start + 1,
                },
            },
            type_syntax: 0,
        });
    }
    text.push_str("): ");
    let result_start = offset(&text);
    text.push_str("i32");
    let result_end = offset(&text);
    let result_type = u32::try_from(types.len()).expect("result type id");
    types.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "i32".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
            },
        },
    });
    text.push_str(" { ");
    let body_start = result_end + 1;
    let mut expressions = Vec::with_capacity(zryna_syntax::v4::MAX_EXPRESSIONS_PER_FUNCTION);
    let mut statements = Vec::with_capacity(5);
    for (local, element_count) in [4_095_usize, 4_095, 4_095, 4_094].into_iter().enumerate() {
        let statement_start = offset(&text);
        text.push_str("const ");
        let name_start = offset(&text);
        let name = format!("a{local}");
        text.push_str(&name);
        let name_end = offset(&text);
        text.push_str(": ");
        let declared_type = fixed_array_type(&mut text, &mut types, element_count);
        text.push(' ');
        let equals = offset(&text);
        text.push_str("= ");
        let constructor_start = offset(&text);
        let constructor_type = fixed_array_type(&mut text, &mut types, element_count);
        let open_paren = offset(&text);
        text.push_str("([");
        let open_bracket = open_paren + 1;
        let first_element = u32::try_from(expressions.len()).expect("first element id");
        for index in 0..element_count {
            if index > 0 {
                text.push_str(", ");
            }
            let start = offset(&text);
            text.push('0');
            expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
            });
        }
        let close_bracket = offset(&text);
        text.push_str("])");
        let close_paren = close_bracket + 1;
        let constructor_end = offset(&text);
        let initializer = u32::try_from(expressions.len()).expect("constructor id");
        expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: constructor_start,
                end: constructor_end,
            },
            kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
                type_syntax: constructor_type,
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_paren,
                    end: open_paren + 1,
                },
                open_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_bracket,
                    end: open_bracket + 1,
                },
                elements: (first_element..initializer).collect(),
                close_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_bracket,
                    end: close_bracket + 1,
                },
                close_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_paren,
                    end: close_paren + 1,
                },
            },
        });
        let semicolon = offset(&text);
        text.push_str("; ");
        statements.push(RawStatementSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: statement_start,
                end: semicolon + 1,
            },
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: statement_start,
                    end: statement_start + 5,
                },
                mutable: false,
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file: 0, start: name_start, end: name_end },
                },
                type_syntax: declared_type,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: equals,
                    end: equals + 1,
                },
                initializer,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: semicolon,
                    end: semicolon + 1,
                },
            },
        });
    }
    let return_start = offset(&text);
    text.push_str("return ");
    let value_start = offset(&text);
    text.push('0');
    let returned = u32::try_from(expressions.len()).expect("return value id");
    expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: value_start, end: value_start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let semicolon = offset(&text);
    text.push_str("; }");
    statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_start, end: semicolon + 1 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: return_start,
                end: return_start + 6,
            },
            value: returned,
            semicolon_span: zryna_source::UntrustedSpan {
                file: 0,
                start: semicolon,
                end: semicolon + 1,
            },
        },
    });
    let end = offset(&text);
    let body_end = end;
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 0, end },
                export_span: None,
                function_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 8 },
                name: RawIdentifierSyntax {
                    text: "budget".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: 9, end: 15 },
                },
                parameters,
                result_type,
                body: RawFunctionBodySyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: body_start, end: body_end },
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_end,
                        },
                        open_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_start + 1,
                        },
                        statements: (0..u32::try_from(statements.len()).expect("statement ids"))
                            .collect(),
                        close_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_end - 1,
                            end: body_end,
                        },
                    }],
                    statements,
                    expressions,
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    (text, raw)
}

#[test]
fn derived_ir_value_budgets_are_exact_and_plus_one_is_rejected() {
    let per_function = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    assert_eq!(derived_value_count(&derived_value_fixture(per_function)), per_function);
    assert_eq!(derived_value_count(&derived_value_fixture(per_function + 1)), per_function + 1);

    assert_eq!(value_budget_violation(0, per_function), None);
    assert_eq!(value_budget_violation(0, per_function + 1), Some(ValueBudgetLimit::Function));
    let per_program = zryna_ir::data_ownership_v1::MAX_VALUES_PER_PROGRAM;
    assert_eq!(value_budget_violation(per_program - per_function, per_function), None);
    assert_eq!(
        value_budget_violation(per_program - per_function + 1, per_function),
        Some(ValueBudgetLimit::Program)
    );
    assert_eq!(value_budget_violation(usize::MAX, per_function), Some(ValueBudgetLimit::Program));
}

#[test]
fn nested_block_branch_and_loop_values_have_exact_checked_boundaries() {
    let exact = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let nested = nested_control_value_fixture(exact);
    assert_eq!(derived_value_count(&nested), exact);
    assert_eq!(value_budget_violation(0, derived_value_count(&nested)), None);

    let plus_one = nested_control_value_fixture(exact + 1);
    assert_eq!(derived_value_count(&plus_one), exact + 1);
    assert_eq!(
        value_budget_violation(0, derived_value_count(&plus_one)),
        Some(ValueBudgetLimit::Function)
    );
}

#[test]
fn terminal_semantic_budget_diagnostic_retains_the_triggering_source_span() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    for _ in 0..MAX_SEMANTIC_DIAGNOSTICS {
        errors.at("ZRYNA-M3201", at, "budget exceeded", "reduce the input");
    }
    let diagnostics = errors.finish();
    let terminal = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3202")
        .expect("terminal diagnostic");
    assert_eq!(terminal.primary_span(), Some(at));
}

#[test]
#[ignore = "authenticated exact/first-extra boundary runs in the full M3 preflight gate"]
fn authenticated_v4_derived_value_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_raw) = authenticated_value_budget_fixture(false);
    let exact_sources = sources_for(&exact_text);
    let exact_syntax = verify_snapshot(exact_raw, &exact_sources).expect("exact value-budget v4");
    let exact_input = pair_input(&exact_syntax, &exact_sources);
    let mut exact_errors = Errors::new(&exact_sources);
    semantic_preflight(exact_input, &mut exact_errors);
    assert!(exact_errors.finish().is_empty(), "exact value budget must pass preflight");

    let (plus_text, plus_raw) = authenticated_value_budget_fixture(true);
    let plus_sources = sources_for(&plus_text);
    let plus_syntax = verify_snapshot(plus_raw, &plus_sources).expect("plus-one value-budget v4");
    let mut plus_errors = Errors::new(&plus_sources);
    semantic_preflight(pair_input(&plus_syntax, &plus_sources), &mut plus_errors);
    let diagnostics = plus_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    let primary = diagnostics[0].primary_span().expect("function source span");
    assert_eq!(
        (primary.start(), primary.end()),
        (0, u32::try_from(plus_text.len()).expect("fixture length"))
    );
}
