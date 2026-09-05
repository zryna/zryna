use super::{authorities, owned_direct_call_program, scalar_call_chain};
use crate::data_ownership_v1::{raw, verify};
use zryna_layout::{StorageTarget, raw as raw_layout};
use zryna_source::{SourceFileInput, SourceMap, Span};

fn authorities_for_two_modules()
-> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
    let text = "export function id(value: i32): i32 { return value; }";
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "00-main.zry".into(), text: text.into() },
        SourceFileInput { path: "01-lib.zry".into(), text: text.into() },
    ])
    .expect("source map");
    let graph = raw_layout::Graph {
        modules: (0..2)
            .map(|id| raw_layout::Module {
                id: raw_layout::ModuleId(id),
                source_file: sources.verify_file_id(id).expect("source file"),
                data_declarations: 0,
            })
            .collect(),
        types: vec![
            raw_layout::TypeNode {
                id: raw_layout::NodeId(0),
                span: None,
                kind: raw_layout::TypeKind::Bool,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(1),
                span: None,
                kind: raw_layout::TypeKind::I32,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(2),
                span: None,
                kind: raw_layout::TypeKind::String,
            },
        ],
        program_roots: vec![],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

fn rebind_spans(function: &mut raw::Function, span: Span) {
    function.span = span;
    for value in &mut function.parameters {
        value.span = span;
    }
    for borrow in &mut function.borrow_parameters {
        borrow.span = span;
    }
    for place in &mut function.places {
        place.span = span;
    }
    for block in &mut function.blocks {
        for value in &mut block.parameters {
            value.span = span;
        }
        for instruction in &mut block.instructions {
            instruction.span = span;
            if let Some(value) = &mut instruction.result {
                value.span = span;
            }
        }
        for terminator in &mut block.terminators {
            terminator.span = span;
        }
    }
    for cleanup in &mut function.cleanup_plans {
        cleanup.span = span;
    }
}

fn cross_module_owned_call()
-> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts, raw::Program) {
    let (sources, linear, linux) = authorities_for_two_modules();
    let (one_source, one_linear, one_linux) = authorities();
    let mut program = owned_direct_call_program(&one_source, &one_linear, &one_linux);
    program.authorities.type_universe = linear.universe_identity().as_bytes();
    program.authorities.linear32_fingerprint = *linear.fingerprint();
    program.authorities.linux_x86_64_fingerprint = *linux.fingerprint();
    let main = sources.verify_file_id(0).expect("main");
    program.modules[0].source_file = main;
    program.modules[0].data_declarations = 0;
    rebind_spans(
        &mut program.modules[0].functions[0],
        sources.span(main, 0, 53).expect("main span"),
    );
    let mut callee = program.modules[0].functions.pop().expect("callee");
    callee.id = raw::FunctionId { module: raw::ModuleId(1), declaration: 0 };
    let library = sources.verify_file_id(1).expect("library");
    rebind_spans(&mut callee, sources.span(library, 0, 53).expect("library span"));
    let raw::InstructionKind::DirectCall { callee: target, .. } =
        &mut program.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("direct call")
    };
    *target = callee.id;
    program.modules.push(raw::Module {
        id: raw::ModuleId(1),
        source_file: library,
        data_declarations: 0,
        functions: vec![callee],
    });
    (sources, linear, linux, program)
}

fn has(diagnostics: &[zryna_diagnostics::Diagnostic], code: &str) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.code() == code)
}

fn cross_module_scalar_chain(
    functions: usize,
) -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts, raw::Program) {
    let (sources, linear, linux) = authorities_for_two_modules();
    let (one_source, one_linear, one_linux) = authorities();
    let mut program = scalar_call_chain(&one_source, &one_linear, &one_linux, functions);
    program.authorities.type_universe = linear.universe_identity().as_bytes();
    program.authorities.linear32_fingerprint = *linear.fingerprint();
    program.authorities.linux_x86_64_fingerprint = *linux.fingerprint();
    let main = sources.verify_file_id(0).expect("main");
    let library = sources.verify_file_id(1).expect("library");
    let mut functions = std::mem::take(&mut program.modules[0].functions);
    let mut caller = functions.remove(0);
    rebind_spans(&mut caller, sources.span(main, 0, 53).expect("main span"));
    if let Some(instruction) = caller.blocks[0].instructions.first_mut() {
        let raw::InstructionKind::DirectCall { callee, .. } = &mut instruction.kind else {
            unreachable!()
        };
        *callee = raw::FunctionId { module: raw::ModuleId(1), declaration: 0 };
    }
    for (index, function) in functions.iter_mut().enumerate() {
        function.id = raw::FunctionId {
            module: raw::ModuleId(1),
            declaration: u32::try_from(index).unwrap(),
        };
        function.entry_export = None;
        rebind_spans(function, sources.span(library, 0, 53).expect("library span"));
        if let Some(instruction) = function.blocks[0].instructions.first_mut() {
            let raw::InstructionKind::DirectCall { callee, .. } = &mut instruction.kind else {
                unreachable!()
            };
            *callee = raw::FunctionId {
                module: raw::ModuleId(1),
                declaration: u32::try_from(index + 1).unwrap(),
            };
        }
    }
    program.modules[0].source_file = main;
    program.modules[0].data_declarations = 0;
    program.modules[0].functions = vec![caller];
    program.modules.push(raw::Module {
        id: raw::ModuleId(1),
        source_file: library,
        data_declarations: 0,
        functions,
    });
    (sources, linear, linux, program)
}

#[test]
fn named_import_cross_module_function_ids_and_cleanup_are_verified_independently() {
    let (sources, linear, linux, program) = cross_module_owned_call();
    let entry = sources.verify_file_id(0).expect("entry");
    verify(program.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("genuine cross-module owned call");

    let mut mutations = Vec::new();
    let mut wrong_target = program.clone();
    let raw::InstructionKind::DirectCall { callee, .. } =
        &mut wrong_target.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    callee.declaration = 1;
    mutations.push((wrong_target, "ZRYNA-I3009"));

    let mut wrong_signature = program.clone();
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut wrong_signature.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    arguments.pop();
    mutations.push((wrong_signature, "ZRYNA-I3009"));

    let mut wrong_result = program.clone();
    wrong_result.modules[0].functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("result")
        .ty = raw::TypeId(0);
    wrong_result.modules[0].functions[0].result = raw::TypeId(0);
    mutations.push((wrong_result, "ZRYNA-I3009"));

    let mut duplicate_owner = program.clone();
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut duplicate_owner.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    arguments[1] = arguments[0].clone();
    mutations.push((duplicate_owner, "ZRYNA-I3012"));

    let mut caller_cleanup = program.clone();
    caller_cleanup.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(0)));
    mutations.push((caller_cleanup, "ZRYNA-I3012"));

    let mut callee_cleanup = program.clone();
    callee_cleanup.modules[1].functions[0].cleanup_plans[0].actions.pop();
    mutations.push((callee_cleanup, "ZRYNA-I3012"));

    for (mutation, code) in mutations {
        let diagnostics = verify(mutation.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect_err("hostile cross-module call");
        assert!(has(&diagnostics, code), "missing {code}: {diagnostics:?}");
        assert_eq!(
            verify(mutation, &sources, entry, linear.clone(), linux.clone())
                .expect_err("deterministic hostile replay"),
            diagnostics
        );
        verify(program.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect("valid recovery after hostile call");
    }
}

#[test]
fn named_import_two_module_function_ids_do_not_bypass_call_cycle_verification() {
    let (sources, linear, linux) = authorities_for_two_modules();
    let (one_source, one_linear, one_linux) = authorities();
    let mut program = scalar_call_chain(&one_source, &one_linear, &one_linux, 2);
    program.authorities.type_universe = linear.universe_identity().as_bytes();
    program.authorities.linear32_fingerprint = *linear.fingerprint();
    program.authorities.linux_x86_64_fingerprint = *linux.fingerprint();
    let main = sources.verify_file_id(0).expect("main");
    program.modules[0].source_file = main;
    program.modules[0].data_declarations = 0;
    for function in &mut program.modules[0].functions {
        rebind_spans(function, sources.span(main, 0, 53).expect("main span"));
    }
    let mut callee = program.modules[0].functions.pop().expect("callee");
    let library = sources.verify_file_id(1).expect("library");
    callee.id = raw::FunctionId { module: raw::ModuleId(1), declaration: 0 };
    rebind_spans(&mut callee, sources.span(library, 0, 53).expect("span"));
    let raw::InstructionKind::DirectCall { callee: target, .. } =
        &mut program.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    *target = callee.id;
    let span = callee.span;
    callee.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
            arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    callee.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(1) };
    callee.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
    program.modules.push(raw::Module {
        id: raw::ModuleId(1),
        source_file: library,
        data_declarations: 0,
        functions: vec![callee],
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(program, &sources, entry, linear, linux).expect_err("cycle");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3009");
    assert_eq!(diagnostics[0].message, "direct call graph contains a cycle");
    assert_eq!(diagnostics[0].guidance, "use one acyclic exact-signature direct call graph");
}

#[test]
fn named_import_cross_module_static_depth_is_exact_and_first_extra_rejected() {
    let (sources, linear, linux, exact) = cross_module_scalar_chain(super::MAX_STATIC_CALL_DEPTH);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(exact, &sources, entry, linear, linux).expect("exact cross-module call depth");

    let (sources, linear, linux, extra) =
        cross_module_scalar_chain(super::MAX_STATIC_CALL_DEPTH + 1);
    let entry = sources.verify_file_id(0).expect("entry");
    let expected = verify(extra.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect_err("first extra cross-module call depth");
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].code(), "ZRYNA-I3009");
    assert_eq!(expected[0].message, "static call depth exceeds its limit");
    assert_eq!(
        verify(extra, &sources, entry, linear, linux).expect_err("deterministic replay"),
        expected
    );
}
