use super::*;

const COPY_CALL_SOURCE: &str = "function caller(x: i32): i32 { return add(x, 1); } function add(x: i32, y: i32): i32 { return x + y; }";
const COPY_CALL_RESPONSE: &str = r#"{"id":300,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":19,"end":22},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":19,"end":22}}}},{"span":{"file":0,"start":25,"end":28},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":25,"end":28}}}},{"span":{"file":0,"start":67,"end":70},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":67,"end":70}}}},{"span":{"file":0,"start":75,"end":78},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":75,"end":78}}}},{"span":{"file":0,"start":81,"end":84},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":81,"end":84}}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":50},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"caller","span":{"file":0,"start":9,"end":15}},"parameters":[{"span":{"file":0,"start":16,"end":22},"name":{"text":"x","span":{"file":0,"start":16,"end":17}},"type_syntax":0}],"result_type":1,"body":{"span":{"file":0,"start":29,"end":50},"root_block":0,"blocks":[{"span":{"file":0,"start":29,"end":50},"open_brace_span":{"file":0,"start":29,"end":30},"statements":[0],"close_brace_span":{"file":0,"start":49,"end":50}}],"statements":[{"span":{"file":0,"start":31,"end":48},"kind":{"kind":"return","keyword_span":{"file":0,"start":31,"end":37},"value":2,"semicolon_span":{"file":0,"start":47,"end":48}}}],"expressions":[{"span":{"file":0,"start":42,"end":43},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":42,"end":43}}}},{"span":{"file":0,"start":45,"end":46},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":38,"end":47},"kind":{"kind":"call","callee":{"text":"add","span":{"file":0,"start":38,"end":41}},"open_paren_span":{"file":0,"start":41,"end":42},"arguments":[0,1],"close_paren_span":{"file":0,"start":46,"end":47}}}]}},{"span":{"file":0,"start":51,"end":102},"export_span":null,"function_span":{"file":0,"start":51,"end":59},"name":{"text":"add","span":{"file":0,"start":60,"end":63}},"parameters":[{"span":{"file":0,"start":64,"end":70},"name":{"text":"x","span":{"file":0,"start":64,"end":65}},"type_syntax":2},{"span":{"file":0,"start":72,"end":78},"name":{"text":"y","span":{"file":0,"start":72,"end":73}},"type_syntax":3}],"result_type":4,"body":{"span":{"file":0,"start":85,"end":102},"root_block":0,"blocks":[{"span":{"file":0,"start":85,"end":102},"open_brace_span":{"file":0,"start":85,"end":86},"statements":[0],"close_brace_span":{"file":0,"start":101,"end":102}}],"statements":[{"span":{"file":0,"start":87,"end":100},"kind":{"kind":"return","keyword_span":{"file":0,"start":87,"end":93},"value":2,"semicolon_span":{"file":0,"start":99,"end":100}}}],"expressions":[{"span":{"file":0,"start":94,"end":95},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":94,"end":95}}}},{"span":{"file":0,"start":98,"end":99},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":98,"end":99}}}},{"span":{"file":0,"start":94,"end":99},"kind":{"kind":"addition","operator_span":{"file":0,"start":96,"end":97},"lhs":0,"rhs":1}}]}}]}],"diagnostics":[]}}"#;
const COPY_AGGREGATE_CALL_SOURCE: &str = "interface P extends ZrynaStruct { x: i32; } function id(p: P): P { return p; } function use(p: P): i32 { const q: P = id(p); return p.x + q.x; }";
const COPY_AGGREGATE_CALL_RESPONSE: &str = r#"{"id":302,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":37,"end":40},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":37,"end":40}}}},{"span":{"file":0,"start":59,"end":60},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":59,"end":60}}}},{"span":{"file":0,"start":63,"end":64},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":63,"end":64}}}},{"span":{"file":0,"start":95,"end":96},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":95,"end":96}}}},{"span":{"file":0,"start":99,"end":102},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":99,"end":102}}}},{"span":{"file":0,"start":114,"end":115},"kind":{"kind":"named","name":{"text":"P","span":{"file":0,"start":114,"end":115}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":43},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"P","span":{"file":0,"start":10,"end":11}},"extends_span":{"file":0,"start":12,"end":19},"marker_span":{"file":0,"start":20,"end":31},"open_brace_span":{"file":0,"start":32,"end":33},"close_brace_span":{"file":0,"start":42,"end":43},"fields":[{"span":{"file":0,"start":34,"end":41},"name":{"text":"x","span":{"file":0,"start":34,"end":35}},"colon_span":{"file":0,"start":35,"end":36},"semicolon_span":{"file":0,"start":40,"end":41},"type_syntax":0}]}}],"functions":[{"span":{"file":0,"start":44,"end":78},"export_span":null,"function_span":{"file":0,"start":44,"end":52},"name":{"text":"id","span":{"file":0,"start":53,"end":55}},"parameters":[{"span":{"file":0,"start":56,"end":60},"name":{"text":"p","span":{"file":0,"start":56,"end":57}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":65,"end":78},"root_block":0,"blocks":[{"span":{"file":0,"start":65,"end":78},"open_brace_span":{"file":0,"start":65,"end":66},"statements":[0],"close_brace_span":{"file":0,"start":77,"end":78}}],"statements":[{"span":{"file":0,"start":67,"end":76},"kind":{"kind":"return","keyword_span":{"file":0,"start":67,"end":73},"value":0,"semicolon_span":{"file":0,"start":75,"end":76}}}],"expressions":[{"span":{"file":0,"start":74,"end":75},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":74,"end":75}}}}]}},{"span":{"file":0,"start":79,"end":144},"export_span":null,"function_span":{"file":0,"start":79,"end":87},"name":{"text":"use","span":{"file":0,"start":88,"end":91}},"parameters":[{"span":{"file":0,"start":92,"end":96},"name":{"text":"p","span":{"file":0,"start":92,"end":93}},"type_syntax":3}],"result_type":4,"body":{"span":{"file":0,"start":103,"end":144},"root_block":0,"blocks":[{"span":{"file":0,"start":103,"end":144},"open_brace_span":{"file":0,"start":103,"end":104},"statements":[0,1],"close_brace_span":{"file":0,"start":143,"end":144}}],"statements":[{"span":{"file":0,"start":105,"end":124},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":105,"end":110},"mutable":false,"name":{"text":"q","span":{"file":0,"start":111,"end":112}},"type_syntax":5,"equals_span":{"file":0,"start":116,"end":117},"initializer":1,"semicolon_span":{"file":0,"start":123,"end":124}}},{"span":{"file":0,"start":125,"end":142},"kind":{"kind":"return","keyword_span":{"file":0,"start":125,"end":131},"value":6,"semicolon_span":{"file":0,"start":141,"end":142}}}],"expressions":[{"span":{"file":0,"start":121,"end":122},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":121,"end":122}}}},{"span":{"file":0,"start":118,"end":123},"kind":{"kind":"call","callee":{"text":"id","span":{"file":0,"start":118,"end":120}},"open_paren_span":{"file":0,"start":120,"end":121},"arguments":[0],"close_paren_span":{"file":0,"start":122,"end":123}}},{span":{"file":0,"start":132,"end":133},"kind":{"kind":"reference","name":{"text":"p","span":{"file":0,"start":132,"end":133}}}},{"span":{"file":0,"start":132,"end":135},"kind":{"kind":"field-access","base":2,"dot_span":{"file":0,"start":133,"end":134},"field":{"text":"x","span":{"file":0,"start":134,"end":135}}}},{"span":{"file":0,"start":138,"end":139},"kind":{"kind":"reference","name":{"text":"q","span":{"file":0,"start":138,"end":139}}}},{"span":{"file":0,"start":138,"end":141},"kind":{"kind":"field-access","base":4,"dot_span":{"file":0,"start":139,"end":140},"field":{"text":"x","span":{"file":0,"start":140,"end":141}}}},{"span":{"file":0,"start":132,"end":141},"kind":{"kind":"addition","operator_span":{"file":0,"start":136,"end":137},"lhs":3,"rhs":5}}]}}]}],"diagnostics":[]}}"#;

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
fn private_copy_call_checks_borrow_signature_arity_before_argument_evaluation() {
    let sources = sources_for(COPY_CALL_SOURCE);
    let syntax = verify_snapshot(response_snapshot(COPY_CALL_RESPONSE), &sources)
        .expect("source-faithful Copy call v4");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified Copy layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let ty = authenticated_type_capabilities(input, 0, 0).expect("i32 type");
    let function = &syntax.files()[0].functions()[0];
    let expression = &function.body.expressions[2];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } = &expression.kind else {
        panic!("Copy call")
    };
    let call_span = span(&sources, expression.span);
    let callee_span = span(&sources, callee.span);
    let catalog = FunctionCatalog {
        modules: vec![vec![
            None,
            Some(FunctionSignature {
                id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                name: "add".to_owned(),
                parameters: Vec::new(),
                borrow_parameters: vec![FunctionBorrowParameter {
                    referent: ty,
                    access: raw::BorrowAccess::Shared,
                    span: callee_span,
                }],
                parameter_order: vec![FunctionParameterOrder::Borrow(0)],
                result: ty,
                private: true,
            }),
        ]],
    };
    let mut errors = Errors::new(&sources);
    let mut lowerer = super::super::FunctionLowerer {
        input,
        file: &syntax.files()[0],
        function,
        module: 0,
        declarations: &declarations,
        graph: &graph,
        node_types: &node_types,
        layouts: &layouts,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        borrow_bindings: std::collections::BTreeMap::new(),
        projections: std::collections::BTreeMap::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        values: 0,
    };
    assert!(lowerer.direct_call(callee, &[u32::MAX, u32::MAX], call_span).is_none());
    assert_eq!(lowerer.values, 0);
    assert!(lowerer.instructions.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3008");
    assert_eq!(
        diagnostics[0].message(),
        "call to 'add' has 2 arguments but its signature requires 1"
    );
    assert_eq!(diagnostics[0].primary_span(), Some(call_span));
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
