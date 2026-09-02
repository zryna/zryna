use super::*;

const EMPTY_VEC_SOURCE: &str = "function empty(): Vec<i32> { return Vec<i32>([]); }";
const EMPTY_VEC_RESPONSE: &str = r#"{"id":31,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":22,"end":25},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":22,"end":25}}}},{"span":{"file":0,"start":18,"end":26},"kind":{"kind":"vec","keyword_span":{"file":0,"start":18,"end":21},"less_than_span":{"file":0,"start":21,"end":22},"argument":0,"greater_than_span":{"file":0,"start":25,"end":26}}},{"span":{"file":0,"start":40,"end":43},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":40,"end":43}}}},{"span":{"file":0,"start":36,"end":44},"kind":{"kind":"vec","keyword_span":{"file":0,"start":36,"end":39},"less_than_span":{"file":0,"start":39,"end":40},"argument":2,"greater_than_span":{"file":0,"start":43,"end":44}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":51},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"empty","span":{"file":0,"start":9,"end":14}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":27,"end":51},"root_block":0,"blocks":[{"span":{"file":0,"start":27,"end":51},"open_brace_span":{"file":0,"start":27,"end":28},"statements":[0],"close_brace_span":{"file":0,"start":50,"end":51}}],"statements":[{"span":{"file":0,"start":29,"end":49},"kind":{"kind":"return","keyword_span":{"file":0,"start":29,"end":35},"value":0,"semicolon_span":{"file":0,"start":48,"end":49}}}],"expressions":[{"span":{"file":0,"start":36,"end":48},"kind":{"kind":"vec-construction","type_syntax":3,"open_paren_span":{"file":0,"start":44,"end":45},"open_bracket_span":{"file":0,"start":45,"end":46},"elements":[],"close_bracket_span":{"file":0,"start":46,"end":47},"close_paren_span":{"file":0,"start":47,"end":48}}}]}}]}],"diagnostics":[]}}"#;
const MOVED_VEC_ELEMENT_SOURCE: &str = "function bad(): Vec<String> { const first: String = \"a\"; return Vec<String>([first, first]); }";
const MOVED_VEC_ELEMENT_RESPONSE: &str = r#"{"id":32,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":20,"end":26},"kind":{"kind":"string","keyword_span":{"file":0,"start":20,"end":26}}},{"span":{"file":0,"start":16,"end":27},"kind":{"kind":"vec","keyword_span":{"file":0,"start":16,"end":19},"less_than_span":{"file":0,"start":19,"end":20},"argument":0,"greater_than_span":{"file":0,"start":26,"end":27}}},{"span":{"file":0,"start":43,"end":49},"kind":{"kind":"string","keyword_span":{"file":0,"start":43,"end":49}}},{"span":{"file":0,"start":68,"end":74},"kind":{"kind":"string","keyword_span":{"file":0,"start":68,"end":74}}},{"span":{"file":0,"start":64,"end":75},"kind":{"kind":"vec","keyword_span":{"file":0,"start":64,"end":67},"less_than_span":{"file":0,"start":67,"end":68},"argument":3,"greater_than_span":{"file":0,"start":74,"end":75}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":94},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":28,"end":94},"root_block":0,"blocks":[{"span":{"file":0,"start":28,"end":94},"open_brace_span":{"file":0,"start":28,"end":29},"statements":[0,1],"close_brace_span":{"file":0,"start":93,"end":94}}],"statements":[{"span":{"file":0,"start":30,"end":56},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":30,"end":35},"mutable":false,"name":{"text":"first","span":{"file":0,"start":36,"end":41}},"type_syntax":2,"equals_span":{"file":0,"start":50,"end":51},"initializer":0,"semicolon_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":57,"end":92},"kind":{"kind":"return","keyword_span":{"file":0,"start":57,"end":63},"value":3,"semicolon_span":{"file":0,"start":91,"end":92}}}],"expressions":[{"span":{"file":0,"start":52,"end":55},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":77,"end":82},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":77,"end":82}}}},{"span":{"file":0,"start":84,"end":89},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":84,"end":89}}}},{"span":{"file":0,"start":64,"end":91},"kind":{"kind":"vec-construction","type_syntax":4,"open_paren_span":{"file":0,"start":75,"end":76},"open_bracket_span":{"file":0,"start":76,"end":77},"elements":[1,2],"close_bracket_span":{"file":0,"start":89,"end":90},"close_paren_span":{"file":0,"start":90,"end":91}}}]}}]}],"diagnostics":[]}}"#;
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
