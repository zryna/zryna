use super::*;
use raw::{
    Block, BlockId, Function, FunctionId, Instruction, InstructionKind, Module, ModuleId, Program,
    SpannedTerminator, Terminator, ValueDefinition, ValueId,
};
use zryna_source::{NormalizedSourcePath, SourceFileInput};

fn sources(paths: &[&str]) -> SourceMap {
    SourceMap::build(
        paths
            .iter()
            .map(|path| SourceFileInput { path: (*path).to_owned(), text: "x".to_owned() })
            .collect(),
    )
    .expect("fixture source map")
}

fn file(sources: &SourceMap, path: &str) -> FileId {
    sources.file_id(&NormalizedSourcePath::new(path).expect("fixture path")).expect("fixture file")
}

fn span(sources: &SourceMap, path: &str) -> Span {
    sources.span(file(sources, path), 0, 1).expect("fixture span")
}

fn value(id: u32, ty: Type, span: Span) -> ValueDefinition {
    ValueDefinition { id: ValueId(id), ty, span }
}

fn instruction(id: u32, ty: Type, span: Span, kind: InstructionKind) -> Instruction {
    Instruction { result: value(id, ty, span), kind }
}

fn ret(span: Span, value: u32) -> Vec<SpannedTerminator> {
    vec![SpannedTerminator { span, kind: Terminator::Return(ValueId(value)) }]
}

fn literal_function(module: u32, declaration: u32, span: Span, export: Option<&str>) -> Function {
    Function {
        id: FunctionId { module: ModuleId(module), declaration },
        entry_export: export.map(str::to_owned),
        span,
        parameters: Vec::new(),
        result: Type::I32,
        blocks: vec![Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: vec![instruction(0, Type::I32, span, InstructionKind::I32Literal(42))],
            terminators: ret(span, 0),
        }],
    }
}

fn literal_program(sources: &SourceMap) -> Program {
    let main = file(sources, "src/main.zry");
    let span = span(sources, "src/main.zry");
    Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file: main,
            functions: vec![literal_function(0, 0, span, Some("answer"))],
        }],
    }
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(Diagnostic::code).collect()
}

fn preflight_has_limit(program: &Program) -> bool {
    let mut errors = Errors::default();
    preflight(program, &mut errors);
    errors.diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I2201")
}

#[test]
fn verifies_one_block_m1_equivalent_without_changing_m1_surface() {
    let sources = sources(&["src/main.zry"]);
    let verified = verify(literal_program(&sources), &sources, file(&sources, "src/main.zry"))
        .expect("single-block M2 representation must verify");
    assert_eq!(verified.entry_module().index(), 0);
    assert_eq!(verified.scalar_abi().exports().len(), 1);
    let module = verified.modules().next().expect("module");
    assert_eq!(module.id().index(), 0);
    let function = module.functions().next().expect("function");
    assert_eq!(function.id().declaration(), 0);
    assert_eq!(function.public_export().expect("public export").logical_name().as_str(), "answer");
    let block = function.blocks().next().expect("block");
    let operation = block.instructions().next().expect("instruction").kind();
    assert_eq!(operation, VerifiedInstructionKind::I32Literal(42));
    assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return(ValueIdentity(0)));

    let m1 = crate::Program::default();
    assert!(crate::verify(m1, &sources).is_ok(), "existing M1 verifier remains available");
}

#[test]
fn verifies_bool_diamond_and_parallel_branch_edges() {
    let sources = sources(&["src/main.zry"]);
    let span = span(&sources, "src/main.zry");
    let function = Function {
        id: FunctionId { module: ModuleId(0), declaration: 0 },
        entry_export: Some("select".to_owned()),
        span,
        parameters: vec![value(0, Type::Bool, span)],
        result: Type::I32,
        blocks: vec![
            Block {
                id: BlockId(0),
                parameters: Vec::new(),
                instructions: vec![
                    instruction(1, Type::I32, span, InstructionKind::I32Literal(11)),
                    instruction(2, Type::I32, span, InstructionKind::I32Literal(22)),
                ],
                terminators: vec![SpannedTerminator {
                    span,
                    kind: Terminator::Branch {
                        condition: ValueId(0),
                        true_target: BlockId(1),
                        true_arguments: vec![ValueId(1)],
                        false_target: BlockId(1),
                        false_arguments: vec![ValueId(2)],
                    },
                }],
            },
            Block {
                id: BlockId(1),
                parameters: vec![value(3, Type::I32, span)],
                instructions: Vec::new(),
                terminators: ret(span, 3),
            },
        ],
    };
    let program = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file: file(&sources, "src/main.zry"),
            functions: vec![function],
        }],
    };
    let verified = verify(program, &sources, file(&sources, "src/main.zry"))
        .expect("diamond fixture must verify");
    let export = verified.scalar_abi().export("select").expect("bool ABI export");
    assert_eq!(export.parameters(), &[zryna_abi::ScalarType::Bool]);
}

#[test]
fn verifies_every_approved_scalar_operation() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    let span = span(&sources, "src/main.zry");
    let operations = vec![
        instruction(0, Type::I32, span, InstructionKind::I32Literal(7)),
        instruction(1, Type::I32, span, InstructionKind::I32Literal(3)),
        instruction(2, Type::Bool, span, InstructionKind::BoolLiteral(true)),
        instruction(
            3,
            Type::I32,
            span,
            InstructionKind::I32Add { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(
            4,
            Type::I32,
            span,
            InstructionKind::I32Sub { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(
            5,
            Type::I32,
            span,
            InstructionKind::I32Mul { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(6, Type::I32, span, InstructionKind::I32Neg { operand: ValueId(0) }),
        instruction(7, Type::Bool, span, InstructionKind::Eq { lhs: ValueId(0), rhs: ValueId(1) }),
        instruction(8, Type::Bool, span, InstructionKind::Ne { lhs: ValueId(2), rhs: ValueId(2) }),
        instruction(
            9,
            Type::Bool,
            span,
            InstructionKind::I32LtS { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(
            10,
            Type::Bool,
            span,
            InstructionKind::I32LeS { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(
            11,
            Type::Bool,
            span,
            InstructionKind::I32GtS { lhs: ValueId(0), rhs: ValueId(1) },
        ),
        instruction(
            12,
            Type::Bool,
            span,
            InstructionKind::I32GeS { lhs: ValueId(0), rhs: ValueId(1) },
        ),
    ];
    let program = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file: entry,
            functions: vec![Function {
                id: FunctionId { module: ModuleId(0), declaration: 0 },
                entry_export: Some("operators".to_owned()),
                span,
                parameters: Vec::new(),
                result: Type::I32,
                blocks: vec![Block {
                    id: BlockId(0),
                    parameters: Vec::new(),
                    instructions: operations,
                    terminators: ret(span, 3),
                }],
            }],
        }],
    };
    let verified = verify(program, &sources, entry).expect("complete operator fixture");
    assert_eq!(
        verified
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
            .len(),
        13
    );
}

#[test]
fn verifies_natural_loop_and_multi_module_direct_call() {
    let sources = sources(&["src/dep.zry", "src/main.zry"]);
    let dep_span = span(&sources, "src/dep.zry");
    let main_span = span(&sources, "src/main.zry");
    let dep = literal_function(0, 0, dep_span, None);
    let main = Function {
        id: FunctionId { module: ModuleId(1), declaration: 0 },
        entry_export: Some("run".to_owned()),
        span: main_span,
        parameters: Vec::new(),
        result: Type::I32,
        blocks: vec![
            Block {
                id: BlockId(0),
                parameters: Vec::new(),
                instructions: vec![instruction(
                    0,
                    Type::I32,
                    main_span,
                    InstructionKind::I32Literal(0),
                )],
                terminators: vec![SpannedTerminator {
                    span: main_span,
                    kind: Terminator::Jump { target: BlockId(1), arguments: vec![ValueId(0)] },
                }],
            },
            Block {
                id: BlockId(1),
                parameters: vec![value(1, Type::I32, main_span)],
                instructions: vec![instruction(
                    2,
                    Type::Bool,
                    main_span,
                    InstructionKind::BoolLiteral(false),
                )],
                terminators: vec![SpannedTerminator {
                    span: main_span,
                    kind: Terminator::Branch {
                        condition: ValueId(2),
                        true_target: BlockId(1),
                        true_arguments: vec![ValueId(1)],
                        false_target: BlockId(2),
                        false_arguments: Vec::new(),
                    },
                }],
            },
            Block {
                id: BlockId(2),
                parameters: Vec::new(),
                instructions: vec![instruction(
                    3,
                    Type::I32,
                    main_span,
                    InstructionKind::DirectCall {
                        callee: FunctionId { module: ModuleId(0), declaration: 0 },
                        arguments: Vec::new(),
                    },
                )],
                terminators: ret(main_span, 3),
            },
        ],
    };
    let program = Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: file(&sources, "src/dep.zry"),
                functions: vec![dep],
            },
            Module {
                id: ModuleId(1),
                source_file: file(&sources, "src/main.zry"),
                functions: vec![main],
            },
        ],
    };
    verify(program, &sources, file(&sources, "src/main.zry"))
        .expect("natural loop and acyclic cross-module call must verify");
}

#[test]
fn rejects_source_entry_and_dense_identity_claims() {
    let sources = sources(&["src/dep.zry", "src/main.zry"]);
    let dep_span = span(&sources, "src/dep.zry");
    let main_span = span(&sources, "src/main.zry");
    let base = Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: file(&sources, "src/dep.zry"),
                functions: vec![literal_function(0, 0, dep_span, None)],
            },
            Module {
                id: ModuleId(1),
                source_file: file(&sources, "src/main.zry"),
                functions: vec![literal_function(1, 0, main_span, Some("main"))],
            },
        ],
    };
    let mut wrong_module = base.clone();
    wrong_module.modules[0].id = ModuleId(1);
    assert!(
        codes(
            &verify(wrong_module, &sources, file(&sources, "src/main.zry"))
                .expect_err("module identity")
        )
        .contains(&"ZRYNA-I2001")
    );

    let mut wrong_function = base.clone();
    wrong_function.modules[1].functions[0].id.declaration = 1;
    assert!(
        codes(
            &verify(wrong_function, &sources, file(&sources, "src/main.zry"))
                .expect_err("function identity")
        )
        .contains(&"ZRYNA-I2003")
    );

    let mut wrong_value = base.clone();
    wrong_value.modules[1].functions[0].blocks[0].instructions[0].result.id = ValueId(1);
    assert!(
        codes(
            &verify(wrong_value, &sources, file(&sources, "src/main.zry"))
                .expect_err("value identity")
        )
        .contains(&"ZRYNA-I2008")
    );

    assert!(
        codes(
            &verify(base.clone(), &sources, file(&sources, "src/dep.zry"))
                .expect_err("entry authority")
        )
        .contains(&"ZRYNA-I2002")
    );

    let other = SourceMap::build(vec![
        SourceFileInput { path: "src/dep.zry".to_owned(), text: "x".to_owned() },
        SourceFileInput { path: "src/main.zry".to_owned(), text: "x".to_owned() },
    ])
    .expect("independent source map");
    assert!(
        codes(
            &verify(base, &other, file(&other, "src/main.zry"))
                .expect_err("foreign source authority")
        )
        .iter()
        .any(|code| code.starts_with("ZRYNA-S"))
    );
}

#[test]
fn rejects_cross_module_spans_and_argument_limit_plus_one() {
    let sources = sources(&["src/dep.zry", "src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    let dep_span = span(&sources, "src/dep.zry");
    let main_span = span(&sources, "src/main.zry");
    let mut cross_span = Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: file(&sources, "src/dep.zry"),
                functions: Vec::new(),
            },
            Module {
                id: ModuleId(1),
                source_file: entry,
                functions: vec![literal_function(1, 0, main_span, Some("main"))],
            },
        ],
    };
    cross_span.modules[1].functions[0].blocks[0].instructions[0].result.span = dep_span;
    assert!(
        codes(&verify(cross_span, &sources, entry).expect_err("cross-module span"))
            .contains(&"ZRYNA-I2023")
    );

    let mut call_arguments = Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: file(&sources, "src/dep.zry"),
                functions: vec![literal_function(0, 0, dep_span, None)],
            },
            Module {
                id: ModuleId(1),
                source_file: entry,
                functions: vec![literal_function(1, 0, main_span, Some("main"))],
            },
        ],
    };
    call_arguments.modules[1].functions[0].blocks[0].instructions[0].kind =
        InstructionKind::DirectCall {
            callee: FunctionId { module: ModuleId(0), declaration: 0 },
            arguments: vec![ValueId(0); MAX_PARAMETERS_PER_FUNCTION + 1],
        };
    assert!(
        codes(&verify(call_arguments, &sources, entry).expect_err("call argument +1"))
            .contains(&"ZRYNA-I2201")
    );

    let mut jump_arguments = Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: file(&sources, "src/dep.zry"),
                functions: Vec::new(),
            },
            Module {
                id: ModuleId(1),
                source_file: entry,
                functions: vec![literal_function(1, 0, main_span, Some("main"))],
            },
        ],
    };
    jump_arguments.modules[1].functions[0].blocks[0].terminators[0].kind = Terminator::Jump {
        target: BlockId(0),
        arguments: vec![ValueId(0); MAX_BLOCK_PARAMETERS + 1],
    };
    assert!(
        codes(&verify(jump_arguments, &sources, entry).expect_err("jump argument +1"))
            .contains(&"ZRYNA-I2201")
    );

    let mut branch_arguments = literal_program(&sources);
    branch_arguments.modules[0].functions[0].blocks[0].terminators[0].kind = Terminator::Branch {
        condition: ValueId(0),
        true_target: BlockId(0),
        true_arguments: vec![ValueId(0); MAX_BLOCK_PARAMETERS + 1],
        false_target: BlockId(0),
        false_arguments: Vec::new(),
    };
    assert!(
        codes(&verify(branch_arguments, &sources, entry).expect_err("branch argument +1"))
            .contains(&"ZRYNA-I2201")
    );
}

#[test]
fn rejects_terminator_type_edge_and_call_failures() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    let mut missing = literal_program(&sources);
    missing.modules[0].functions[0].blocks[0].terminators.clear();
    assert!(
        codes(&verify(missing, &sources, entry).expect_err("missing terminator"))
            .contains(&"ZRYNA-I2007")
    );

    let mut duplicate = literal_program(&sources);
    let term = duplicate.modules[0].functions[0].blocks[0].terminators[0].clone();
    duplicate.modules[0].functions[0].blocks[0].terminators.push(term);
    assert!(
        codes(&verify(duplicate, &sources, entry).expect_err("duplicate terminator"))
            .contains(&"ZRYNA-I2007")
    );

    let mut wrong_type = literal_program(&sources);
    wrong_type.modules[0].functions[0].blocks[0].instructions[0].result.ty = Type::Bool;
    assert!(
        codes(&verify(wrong_type, &sources, entry).expect_err("wrong literal type"))
            .contains(&"ZRYNA-I2010")
    );

    let mut edge_to_entry = literal_program(&sources);
    edge_to_entry.modules[0].functions[0].blocks[0].terminators = vec![SpannedTerminator {
        span: span(&sources, "src/main.zry"),
        kind: Terminator::Jump { target: BlockId(0), arguments: Vec::new() },
    }];
    assert!(
        codes(&verify(edge_to_entry, &sources, entry).expect_err("edge to entry"))
            .contains(&"ZRYNA-I2016")
    );

    let mut call_cycle = literal_program(&sources);
    call_cycle.modules[0].functions[0].blocks[0].instructions[0].kind =
        InstructionKind::DirectCall {
            callee: FunctionId { module: ModuleId(0), declaration: 0 },
            arguments: Vec::new(),
        };
    assert!(
        codes(&verify(call_cycle, &sources, entry).expect_err("recursive call"))
            .contains(&"ZRYNA-I2021")
    );
}

#[test]
fn rejects_non_dominating_use_and_irreducible_cfg() {
    let sources = sources(&["src/main.zry"]);
    let span = span(&sources, "src/main.zry");
    let entry = file(&sources, "src/main.zry");
    let function = Function {
        id: FunctionId { module: ModuleId(0), declaration: 0 },
        entry_export: Some("bad".to_owned()),
        span,
        parameters: vec![value(0, Type::Bool, span)],
        result: Type::I32,
        blocks: vec![
            Block {
                id: BlockId(0),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminators: vec![SpannedTerminator {
                    span,
                    kind: Terminator::Branch {
                        condition: ValueId(0),
                        true_target: BlockId(1),
                        true_arguments: Vec::new(),
                        false_target: BlockId(2),
                        false_arguments: Vec::new(),
                    },
                }],
            },
            Block {
                id: BlockId(1),
                parameters: Vec::new(),
                instructions: vec![instruction(1, Type::I32, span, InstructionKind::I32Literal(1))],
                terminators: vec![SpannedTerminator {
                    span,
                    kind: Terminator::Jump { target: BlockId(2), arguments: Vec::new() },
                }],
            },
            Block {
                id: BlockId(2),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminators: ret(span, 1),
            },
        ],
    };
    let program = Program {
        entry_module: ModuleId(0),
        modules: vec![Module { id: ModuleId(0), source_file: entry, functions: vec![function] }],
    };
    assert!(
        codes(&verify(program, &sources, entry).expect_err("non-dominating use"))
            .contains(&"ZRYNA-I2013")
    );

    let mut irreducible = literal_program(&sources);
    let function = &mut irreducible.modules[0].functions[0];
    function.parameters = vec![value(0, Type::Bool, span)];
    function.blocks = vec![
        Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Branch {
                    condition: ValueId(0),
                    true_target: BlockId(1),
                    true_arguments: Vec::new(),
                    false_target: BlockId(2),
                    false_arguments: Vec::new(),
                },
            }],
        },
        Block {
            id: BlockId(1),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Jump { target: BlockId(2), arguments: Vec::new() },
            }],
        },
        Block {
            id: BlockId(2),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Jump { target: BlockId(1), arguments: Vec::new() },
            }],
        },
    ];
    assert!(
        codes(&verify(irreducible, &sources, entry).expect_err("irreducible CFG"))
            .contains(&"ZRYNA-I2020")
    );
}

#[test]
fn exact_block_limit_verifies_iteratively_and_first_extra_fails_preflight() {
    let sources = sources(&["src/main.zry"]);
    let span = span(&sources, "src/main.zry");
    let entry = file(&sources, "src/main.zry");
    let mut blocks = Vec::with_capacity(MAX_BLOCKS_PER_FUNCTION);
    for index in 0..MAX_BLOCKS_PER_FUNCTION {
        let id = u32::try_from(index).expect("bounded fixture");
        let terminators = if index + 1 == MAX_BLOCKS_PER_FUNCTION {
            ret(span, 0)
        } else {
            vec![SpannedTerminator {
                span,
                kind: Terminator::Jump { target: BlockId(id + 1), arguments: Vec::new() },
            }]
        };
        blocks.push(Block {
            id: BlockId(id),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators,
        });
    }
    let function = Function {
        id: FunctionId { module: ModuleId(0), declaration: 0 },
        entry_export: Some("deep".to_owned()),
        span,
        parameters: vec![value(0, Type::I32, span)],
        result: Type::I32,
        blocks,
    };
    let program = Program {
        entry_module: ModuleId(0),
        modules: vec![Module { id: ModuleId(0), source_file: entry, functions: vec![function] }],
    };
    verify(program.clone(), &sources, entry)
        .expect("exact block limit must verify without recursion");
    let mut extra = program;
    extra.modules[0].functions[0].blocks.push(Block {
        id: BlockId(u32::try_from(MAX_BLOCKS_PER_FUNCTION).expect("bounded")),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: ret(span, 0),
    });
    assert!(
        codes(&verify(extra, &sources, entry).expect_err("block limit + 1"))
            .contains(&"ZRYNA-I2201")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn preflight_freezes_exact_and_plus_one_aggregate_ir_budgets() {
    let sources = sources(&["src/main.zry"]);
    let source_file = file(&sources, "src/main.zry");
    let span = span(&sources, "src/main.zry");

    let empty_block = |id: u32| Block {
        id: BlockId(id),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: ret(span, 0),
    };
    let empty_function = |declaration: u32| Function {
        id: FunctionId { module: ModuleId(0), declaration },
        entry_export: None,
        span,
        parameters: vec![value(0, Type::I32, span)],
        result: Type::I32,
        blocks: vec![empty_block(0)],
    };

    let exact_modules = Program {
        entry_module: ModuleId(0),
        modules: (0..MAX_MODULES)
            .map(|index| Module {
                id: ModuleId(u32::try_from(index).expect("bounded")),
                source_file,
                functions: Vec::new(),
            })
            .collect(),
    };
    assert!(!preflight_has_limit(&exact_modules));
    let mut extra_modules = exact_modules;
    extra_modules.modules.push(Module {
        id: ModuleId(u32::try_from(MAX_MODULES).expect("bounded")),
        source_file,
        functions: Vec::new(),
    });
    assert!(preflight_has_limit(&extra_modules));

    let mut functions = (0..MAX_FUNCTIONS_PER_MODULE)
        .map(|index| empty_function(u32::try_from(index).expect("bounded")))
        .collect::<Vec<_>>();
    let exact_functions = Program {
        entry_module: ModuleId(0),
        modules: vec![Module { id: ModuleId(0), source_file, functions: functions.clone() }],
    };
    assert!(!preflight_has_limit(&exact_functions));
    functions.push(empty_function(u32::try_from(MAX_FUNCTIONS_PER_MODULE).expect("bounded")));
    let extra_functions = Program {
        entry_module: ModuleId(0),
        modules: vec![Module { id: ModuleId(0), source_file, functions }],
    };
    assert!(preflight_has_limit(&extra_functions));

    let functions_per_module = MAX_FUNCTIONS_PER_PROGRAM / 4;
    assert!(functions_per_module <= MAX_FUNCTIONS_PER_MODULE);
    let exact_program_functions = Program {
        entry_module: ModuleId(0),
        modules: (0..4)
            .map(|module| Module {
                id: ModuleId(module),
                source_file,
                functions: (0..functions_per_module)
                    .map(|index| empty_function(u32::try_from(index).expect("bounded")))
                    .collect(),
            })
            .collect(),
    };
    assert_eq!(
        exact_program_functions.modules.iter().map(|module| module.functions.len()).sum::<usize>(),
        MAX_FUNCTIONS_PER_PROGRAM
    );
    assert!(!preflight_has_limit(&exact_program_functions));
    let mut extra_program_function = exact_program_functions;
    extra_program_function.modules.push(Module {
        id: ModuleId(4),
        source_file,
        functions: vec![empty_function(0)],
    });
    assert!(preflight_has_limit(&extra_program_function));

    let blocks = (0..MAX_BLOCKS_PER_PROGRAM)
        .map(|index| empty_block(u32::try_from(index % MAX_BLOCKS_PER_FUNCTION).expect("bounded")))
        .collect::<Vec<_>>();
    let exact_blocks = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: blocks
                .chunks(MAX_BLOCKS_PER_FUNCTION)
                .enumerate()
                .map(|(index, chunk)| Function {
                    id: FunctionId {
                        module: ModuleId(0),
                        declaration: u32::try_from(index).expect("bounded"),
                    },
                    entry_export: None,
                    span,
                    parameters: vec![value(0, Type::I32, span)],
                    result: Type::I32,
                    blocks: chunk.to_vec(),
                })
                .collect(),
        }],
    };
    assert!(!preflight_has_limit(&exact_blocks));
    let mut extra_blocks = exact_blocks;
    extra_blocks.modules[0].functions.push(empty_function(16));
    assert!(preflight_has_limit(&extra_blocks));

    let make_value_function = |declaration: u32, value_count: usize| Function {
        id: FunctionId { module: ModuleId(0), declaration },
        entry_export: None,
        span,
        parameters: Vec::new(),
        result: Type::I32,
        blocks: vec![Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: (0..value_count)
                .map(|index| {
                    instruction(
                        u32::try_from(index).expect("bounded"),
                        Type::I32,
                        span,
                        InstructionKind::I32Literal(0),
                    )
                })
                .collect(),
            terminators: ret(span, 0),
        }],
    };
    let exact_values = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: (0..(MAX_VALUES_PER_PROGRAM / MAX_VALUES_PER_FUNCTION))
                .map(|index| {
                    make_value_function(
                        u32::try_from(index).expect("bounded"),
                        MAX_VALUES_PER_FUNCTION,
                    )
                })
                .collect(),
        }],
    };
    assert!(!preflight_has_limit(&exact_values));
    let mut extra_values = exact_values;
    extra_values.modules[0].functions.push(make_value_function(16, 1));
    assert!(preflight_has_limit(&extra_values));
}

#[test]
fn preflight_freezes_cfg_and_call_edge_budgets() {
    let sources = sources(&["src/main.zry"]);
    let source_file = file(&sources, "src/main.zry");
    let span = span(&sources, "src/main.zry");
    let edge_function = |declaration: u32| Function {
        id: FunctionId { module: ModuleId(0), declaration },
        entry_export: None,
        span,
        parameters: vec![value(0, Type::Bool, span)],
        result: Type::I32,
        blocks: (0..MAX_BLOCKS_PER_FUNCTION)
            .map(|index| Block {
                id: BlockId(u32::try_from(index).expect("bounded")),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminators: vec![SpannedTerminator {
                    span,
                    kind: Terminator::Branch {
                        condition: ValueId(0),
                        true_target: BlockId(1),
                        true_arguments: Vec::new(),
                        false_target: BlockId(1),
                        false_arguments: Vec::new(),
                    },
                }],
            })
            .collect(),
    };
    let exact_edges = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: (0..16).map(edge_function).collect(),
        }],
    };
    assert!(!preflight_has_limit(&exact_edges));
    let mut extra_edges = exact_edges;
    extra_edges.modules[0].functions.push(Function {
        id: FunctionId { module: ModuleId(0), declaration: 16 },
        entry_export: None,
        span,
        parameters: vec![value(0, Type::I32, span)],
        result: Type::I32,
        blocks: vec![Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Jump { target: BlockId(0), arguments: Vec::new() },
            }],
        }],
    });
    assert!(preflight_has_limit(&extra_edges));

    let call_function = |declaration: u32, count: usize| Function {
        id: FunctionId { module: ModuleId(0), declaration },
        entry_export: None,
        span,
        parameters: Vec::new(),
        result: Type::I32,
        blocks: vec![Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: (0..count)
                .map(|index| {
                    instruction(
                        u32::try_from(index).expect("bounded"),
                        Type::I32,
                        span,
                        InstructionKind::DirectCall {
                            callee: FunctionId { module: ModuleId(0), declaration: 0 },
                            arguments: Vec::new(),
                        },
                    )
                })
                .collect(),
            terminators: ret(span, 0),
        }],
    };
    let exact_calls = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: (0..4).map(|index| call_function(index, MAX_VALUES_PER_FUNCTION)).collect(),
        }],
    };
    assert!(!preflight_has_limit(&exact_calls));
    let mut extra_calls = exact_calls;
    extra_calls.modules[0].functions.push(call_function(4, 1));
    assert!(preflight_has_limit(&extra_calls));
}

#[test]
#[allow(clippy::too_many_lines)]
fn preflight_freezes_parameter_block_parameter_and_function_value_budgets() {
    let sources = sources(&["src/main.zry"]);
    let source_file = file(&sources, "src/main.zry");
    let span = span(&sources, "src/main.zry");
    let make_parameters = |count: usize| {
        (0..count)
            .map(|index| value(u32::try_from(index).expect("bounded"), Type::I32, span))
            .collect::<Vec<_>>()
    };

    let function_with_parameters = |declaration: u32, count: usize| Function {
        id: FunctionId { module: ModuleId(0), declaration },
        entry_export: None,
        span,
        parameters: make_parameters(count),
        result: Type::I32,
        blocks: vec![Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: ret(span, 0),
        }],
    };
    let exact_parameters = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: (0..(MAX_PARAMETERS_PER_PROGRAM / MAX_PARAMETERS_PER_FUNCTION))
                .map(|index| {
                    function_with_parameters(
                        u32::try_from(index).expect("bounded"),
                        MAX_PARAMETERS_PER_FUNCTION,
                    )
                })
                .collect(),
        }],
    };
    assert!(!preflight_has_limit(&exact_parameters));
    let mut extra_parameters = exact_parameters;
    extra_parameters.modules[0].functions.push(function_with_parameters(1024, 1));
    assert!(preflight_has_limit(&extra_parameters));
    let per_function_extra = Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: vec![function_with_parameters(0, MAX_PARAMETERS_PER_FUNCTION + 1)],
        }],
    };
    assert!(preflight_has_limit(&per_function_extra));

    let block_parameter_program = |count: usize| Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: vec![Function {
                id: FunctionId { module: ModuleId(0), declaration: 0 },
                entry_export: None,
                span,
                parameters: vec![value(0, Type::I32, span)],
                result: Type::I32,
                blocks: vec![
                    Block {
                        id: BlockId(0),
                        parameters: Vec::new(),
                        instructions: Vec::new(),
                        terminators: vec![SpannedTerminator {
                            span,
                            kind: Terminator::Jump {
                                target: BlockId(1),
                                arguments: vec![ValueId(0); count],
                            },
                        }],
                    },
                    Block {
                        id: BlockId(1),
                        parameters: (0..count)
                            .map(|index| {
                                value(u32::try_from(index + 1).expect("bounded"), Type::I32, span)
                            })
                            .collect(),
                        instructions: Vec::new(),
                        terminators: ret(span, 1),
                    },
                ],
            }],
        }],
    };
    assert!(!preflight_has_limit(&block_parameter_program(MAX_BLOCK_PARAMETERS)));
    assert!(preflight_has_limit(&block_parameter_program(MAX_BLOCK_PARAMETERS + 1)));

    let value_program = |count: usize| Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file,
            functions: vec![Function {
                id: FunctionId { module: ModuleId(0), declaration: 0 },
                entry_export: None,
                span,
                parameters: Vec::new(),
                result: Type::I32,
                blocks: vec![Block {
                    id: BlockId(0),
                    parameters: Vec::new(),
                    instructions: (0..count)
                        .map(|index| {
                            instruction(
                                u32::try_from(index).expect("bounded"),
                                Type::I32,
                                span,
                                InstructionKind::I32Literal(0),
                            )
                        })
                        .collect(),
                    terminators: ret(span, 0),
                }],
            }],
        }],
    };
    assert!(!preflight_has_limit(&value_program(MAX_VALUES_PER_FUNCTION)));
    assert!(preflight_has_limit(&value_program(MAX_VALUES_PER_FUNCTION + 1)));
}

fn call_chain_program(sources: &SourceMap, count: usize) -> Program {
    let span = span(sources, "src/main.zry");
    let functions = (0..count)
        .map(|index| {
            let kind = if index + 1 == count {
                InstructionKind::I32Literal(1)
            } else {
                InstructionKind::DirectCall {
                    callee: FunctionId {
                        module: ModuleId(0),
                        declaration: u32::try_from(index + 1).expect("bounded"),
                    },
                    arguments: Vec::new(),
                }
            };
            Function {
                id: FunctionId {
                    module: ModuleId(0),
                    declaration: u32::try_from(index).expect("bounded"),
                },
                entry_export: (index == 0).then(|| "chain".to_owned()),
                span,
                parameters: Vec::new(),
                result: Type::I32,
                blocks: vec![Block {
                    id: BlockId(0),
                    parameters: Vec::new(),
                    instructions: vec![instruction(0, Type::I32, span, kind)],
                    terminators: ret(span, 0),
                }],
            }
        })
        .collect();
    Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file: file(sources, "src/main.zry"),
            functions,
        }],
    }
}

#[test]
fn static_call_depth_accepts_128_and_rejects_129() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    verify(call_chain_program(&sources, MAX_STATIC_CALL_DEPTH), &sources, entry)
        .expect("depth 128");
    assert!(
        codes(
            &verify(call_chain_program(&sources, MAX_STATIC_CALL_DEPTH + 1), &sources, entry)
                .expect_err("depth 129")
        )
        .contains(&"ZRYNA-I2201")
    );
}

fn nested_loop_program(sources: &SourceMap, nesting: usize) -> Program {
    let span = span(sources, "src/main.zry");
    let mut blocks = Vec::with_capacity(nesting * 3 + 2);
    // Entry selects the first header.
    blocks.push(Block {
        id: BlockId(0),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: vec![SpannedTerminator {
            span,
            kind: Terminator::Jump { target: BlockId(1), arguments: Vec::new() },
        }],
    });
    // Header i enters the next nested header/body or exits to its matching latch.
    for level in 0..nesting {
        let header = 1 + level;
        let inner = if level + 1 == nesting { 1 + nesting } else { header + 1 };
        let after = if level == 0 { 0 } else { 1 + nesting + 1 + (level - 1) };
        blocks.push(Block {
            id: BlockId(u32::try_from(header).expect("bounded")),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Branch {
                    condition: ValueId(0),
                    true_target: BlockId(u32::try_from(inner).expect("bounded")),
                    true_arguments: Vec::new(),
                    false_target: BlockId(u32::try_from(after).expect("bounded")),
                    false_arguments: Vec::new(),
                },
            }],
        });
    }
    // Innermost body goes to the innermost latch.
    blocks.push(Block {
        id: BlockId(u32::try_from(1 + nesting).expect("bounded")),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: vec![SpannedTerminator {
            span,
            kind: Terminator::Jump {
                target: BlockId(u32::try_from(1 + nesting + nesting).expect("bounded")),
                arguments: Vec::new(),
            },
        }],
    });
    // Each latch backedges to its header; the header false edge reaches the next outer latch.
    for latch_offset in 0..nesting {
        let block = 1 + nesting + 1 + latch_offset;
        let header = 1 + latch_offset;
        blocks.push(Block {
            id: BlockId(u32::try_from(block).expect("bounded")),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![SpannedTerminator {
                span,
                kind: Terminator::Jump {
                    target: BlockId(u32::try_from(header).expect("bounded")),
                    arguments: Vec::new(),
                },
            }],
        });
    }
    let exit = blocks.len();
    blocks.push(Block {
        id: BlockId(u32::try_from(exit).expect("bounded")),
        parameters: Vec::new(),
        instructions: vec![instruction(1, Type::I32, span, InstructionKind::I32Literal(1))],
        terminators: ret(span, 1),
    });
    // The outer header's false edge reaches the final exit.
    if let Terminator::Branch { false_target, .. } = &mut blocks[1].terminators[0].kind {
        *false_target = BlockId(u32::try_from(exit).expect("bounded"));
    }
    Program {
        entry_module: ModuleId(0),
        modules: vec![Module {
            id: ModuleId(0),
            source_file: file(sources, "src/main.zry"),
            functions: vec![Function {
                id: FunctionId { module: ModuleId(0), declaration: 0 },
                entry_export: Some("loops".to_owned()),
                span,
                parameters: vec![value(0, Type::Bool, span)],
                result: Type::I32,
                blocks,
            }],
        }],
    }
}

#[test]
fn loop_nesting_accepts_128_and_rejects_129_without_recursion() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    verify(nested_loop_program(&sources, MAX_LOOP_NESTING), &sources, entry)
        .expect("loop nesting 128");
    assert!(
        codes(
            &verify(nested_loop_program(&sources, MAX_LOOP_NESTING + 1), &sources, entry)
                .expect_err("loop nesting 129")
        )
        .contains(&"ZRYNA-I2201")
    );
}

#[test]
fn diagnostic_budget_is_bounded_and_terminal() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    let span = span(&sources, "src/main.zry");
    let functions = (0..MAX_DIAGNOSTICS + 32)
        .map(|index| {
            let mut function =
                literal_function(0, u32::try_from(index).expect("bounded"), span, None);
            function.id.module = ModuleId(1);
            function
        })
        .collect();
    let program = Program {
        entry_module: ModuleId(0),
        modules: vec![Module { id: ModuleId(0), source_file: entry, functions }],
    };
    let diagnostics = verify(program, &sources, entry).expect_err("many invalid identities");
    assert_eq!(diagnostics.len(), MAX_DIAGNOSTICS);
    assert_eq!(diagnostics.last().map(Diagnostic::code), Some("ZRYNA-I2202"));
}

#[test]
fn resource_limit_is_one_shot_terminal_and_blocks_state_commits() {
    let sources = sources(&["src/main.zry"]);
    let span = span(&sources, "src/main.zry");
    let mut errors = Errors::default();
    errors.push(error("ZRYNA-I2001", "sentinel before limit", "fixture"));
    errors.limit("sentinel resource", 1);
    errors.limit("later resource", 2);
    errors.push(error("ZRYNA-I2002", "sentinel after limit", "fixture"));

    let mut values = Vec::new();
    define_value(
        &value(0, Type::I32, span),
        DefinitionLocation::Parameter,
        &mut values,
        &mut errors,
    );

    assert!(errors.exhausted());
    assert!(values.is_empty(), "terminal exhaustion must reject later state commits");
    assert_eq!(codes(&errors.diagnostics), vec!["ZRYNA-I2001", "ZRYNA-I2201"]);
}

#[test]
fn resource_limit_stops_later_verifier_phases() {
    let sources = sources(&["src/main.zry"]);
    let entry = file(&sources, "src/main.zry");
    let mut program = nested_loop_program(&sources, MAX_LOOP_NESTING + 1);
    program.modules[0].functions[0].entry_export = Some("1invalid".to_owned());

    let diagnostics = verify(program, &sources, entry).expect_err("loop limit must be terminal");
    assert_eq!(codes(&diagnostics), vec!["ZRYNA-I2201"]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn rejects_every_remaining_identity_type_edge_and_abi_invariant() {
    let sources = sources(&["src/dep.zry", "src/main.zry"]);
    let dep_file = file(&sources, "src/dep.zry");
    let entry = file(&sources, "src/main.zry");
    let dep_span = span(&sources, "src/dep.zry");
    let main_span = span(&sources, "src/main.zry");
    let base = || Program {
        entry_module: ModuleId(1),
        modules: vec![
            Module {
                id: ModuleId(0),
                source_file: dep_file,
                functions: vec![literal_function(0, 0, dep_span, None)],
            },
            Module {
                id: ModuleId(1),
                source_file: entry,
                functions: vec![literal_function(1, 0, main_span, Some("main"))],
            },
        ],
    };
    let assert_code = |program: Program, expected: &str| {
        let diagnostics = verify(program, &sources, entry).expect_err("mutation must fail");
        assert!(
            codes(&diagnostics).contains(&expected),
            "missing {expected}: {:?}",
            codes(&diagnostics)
        );
    };

    let mut dependency_export = base();
    dependency_export.modules[0].functions[0].entry_export = Some("dependency".to_owned());
    assert_code(dependency_export, "ZRYNA-I2004");

    let mut block_identity = base();
    block_identity.modules[1].functions[0].blocks[0].id = BlockId(1);
    assert_code(block_identity, "ZRYNA-I2005");

    let mut entry_parameter = base();
    entry_parameter.modules[1].functions[0].blocks[0].parameters =
        vec![value(0, Type::I32, main_span)];
    entry_parameter.modules[1].functions[0].blocks[0].instructions[0].result.id = ValueId(1);
    entry_parameter.modules[1].functions[0].blocks[0].terminators = ret(main_span, 1);
    assert_code(entry_parameter, "ZRYNA-I2006");

    let mut unit = base();
    unit.modules[1].functions[0].blocks[0].instructions[0].result.ty = Type::Unit;
    assert_code(unit, "ZRYNA-I2009");

    let mut unknown_call = base();
    unknown_call.modules[1].functions[0].blocks[0].instructions[0].kind =
        InstructionKind::DirectCall {
            callee: FunctionId { module: ModuleId(99), declaration: 0 },
            arguments: Vec::new(),
        };
    assert_code(unknown_call, "ZRYNA-I2011");

    let mut signature_call = base();
    signature_call.modules[0].functions[0].parameters = vec![value(0, Type::I32, dep_span)];
    signature_call.modules[0].functions[0].blocks[0].instructions[0].result.id = ValueId(1);
    signature_call.modules[0].functions[0].blocks[0].terminators = ret(dep_span, 1);
    signature_call.modules[1].functions[0].blocks[0].instructions[0].kind =
        InstructionKind::DirectCall {
            callee: FunctionId { module: ModuleId(0), declaration: 0 },
            arguments: Vec::new(),
        };
    assert_code(signature_call, "ZRYNA-I2011");

    let mut unknown_value = base();
    unknown_value.modules[1].functions[0].blocks[0].terminators = ret(main_span, u32::MAX);
    assert_code(unknown_value, "ZRYNA-I2012");

    let mut wrong_return = base();
    wrong_return.modules[1].functions[0].result = Type::Bool;
    assert_code(wrong_return, "ZRYNA-I2014");

    let mut wrong_branch = base();
    wrong_branch.modules[1].functions[0].blocks = vec![
        Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: vec![instruction(
                0,
                Type::I32,
                main_span,
                InstructionKind::I32Literal(1),
            )],
            terminators: vec![SpannedTerminator {
                span: main_span,
                kind: Terminator::Branch {
                    condition: ValueId(0),
                    true_target: BlockId(1),
                    true_arguments: Vec::new(),
                    false_target: BlockId(1),
                    false_arguments: Vec::new(),
                },
            }],
        },
        Block {
            id: BlockId(1),
            parameters: Vec::new(),
            instructions: vec![instruction(
                1,
                Type::I32,
                main_span,
                InstructionKind::I32Literal(2),
            )],
            terminators: ret(main_span, 1),
        },
    ];
    assert_code(wrong_branch, "ZRYNA-I2015");

    let mut edge_arity = base();
    edge_arity.modules[1].functions[0].blocks = vec![
        Block {
            id: BlockId(0),
            parameters: Vec::new(),
            instructions: vec![instruction(
                0,
                Type::I32,
                main_span,
                InstructionKind::I32Literal(1),
            )],
            terminators: vec![SpannedTerminator {
                span: main_span,
                kind: Terminator::Jump { target: BlockId(1), arguments: Vec::new() },
            }],
        },
        Block {
            id: BlockId(1),
            parameters: vec![value(1, Type::I32, main_span)],
            instructions: Vec::new(),
            terminators: ret(main_span, 1),
        },
    ];
    assert_code(edge_arity, "ZRYNA-I2017");

    let mut unreachable = base();
    unreachable.modules[1].functions[0].blocks.push(Block {
        id: BlockId(1),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: ret(main_span, 0),
    });
    assert_code(unreachable, "ZRYNA-I2018");

    let mut invalid_abi = base();
    invalid_abi.modules[1].functions[0].entry_export = Some("1invalid".to_owned());
    assert_code(invalid_abi, "ZRYNA-I2022");

    let mut duplicate_abi = base();
    duplicate_abi.modules[1].functions.push(literal_function(1, 1, main_span, Some("main")));
    assert_code(duplicate_abi, "ZRYNA-I2022");
}
