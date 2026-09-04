use super::*;
use crate::data_ownership_v1::owned_vec_lowering::PrivateVecLowerer;
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy)]
enum Case {
    Integer,
    Unknown,
    Duplicate,
}

// Fixed syntax fixtures only: no resource estimation or semantic classification here.
#[allow(clippy::too_many_lines)]
fn fixture(case: Case) -> (String, RawProjectSyntaxSnapshot, u32) {
    let element = if matches!(case, Case::Integer) { "i32" } else { "String" };
    let seed = if matches!(case, Case::Duplicate) { "const seed: String = \"a\"; " } else { "" };
    let values = match case {
        Case::Integer => "2147483648",
        Case::Unknown => "\"a\", bad",
        Case::Duplicate => "seed, seed",
    };
    let source =
        format!("function order(): Vec<{element}> {{ {seed}return Vec<{element}>([{values}]); }}");
    let vec_spelling = format!("Vec<{element}>");
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let range = |start, ordinal, end, end_ordinal| {
        untrusted_range(&source, (start, ordinal), (end, end_ordinal))
    };
    let mut types = Vec::new();
    let mut vectors = Vec::new();
    for ordinal in 0..2 {
        let whole = token(&vec_spelling, ordinal);
        let inner =
            zryna_source::UntrustedSpan { file: 0, start: whole.start + 4, end: whole.end - 1 };
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: inner,
            kind: if element == "String" {
                RawTypeSyntaxKind::String { keyword_span: inner }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: element.to_owned(), span: inner },
                }
            },
        });
        vectors.push(id + 1);
        types.push(RawTypeSyntax {
            span: whole,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: whole.start,
                    end: whole.start + 3,
                },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: whole.start + 3,
                    end: whole.start + 4,
                },
                argument: id,
                greater_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: whole.end - 1,
                    end: whole.end,
                },
            },
        });
    }
    let mut expressions = Vec::new();
    let mut statements = Vec::new();
    if matches!(case, Case::Duplicate) {
        // Insert the local annotation in canonical source order between the two Vec types.
        let at = token("String", 1);
        types.insert(
            2,
            RawTypeSyntax { span: at, kind: RawTypeSyntaxKind::String { keyword_span: at } },
        );
        let RawTypeSyntaxKind::Vec { argument, .. } = &mut types[4].kind else {
            panic!("second Vec type")
        };
        *argument = 3;
        vectors[1] = 4;
        expressions.push(RawExpressionSyntax {
            span: token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
        });
        statements.push(RawStatementSyntax {
            span: range("const", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "seed".to_owned(), span: token("seed", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        });
    }
    let mut elements = Vec::new();
    let leaves: &[(&str, usize, bool)] = match case {
        Case::Integer => &[("2147483648", 0, false)],
        Case::Unknown => &[("\"a\"", 0, false), ("bad", 0, true)],
        Case::Duplicate => &[("seed", 1, true), ("seed", 2, true)],
    };
    for &(text, ordinal, reference) in leaves {
        let at = token(text, ordinal);
        elements.push(u32::try_from(expressions.len()).expect("expression id"));
        let kind = if reference {
            RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: text.to_owned(), span: at },
            }
        } else if matches!(case, Case::Integer) {
            RawExpressionKind::I32Literal { spelling: text.to_owned() }
        } else {
            RawExpressionKind::StringLiteral { spelling: text.to_owned() }
        };
        expressions.push(RawExpressionSyntax { span: at, kind });
    }
    let constructor = u32::try_from(expressions.len()).expect("constructor id");
    expressions.push(RawExpressionSyntax {
        span: range("Vec", 1, ")", 1),
        kind: RawExpressionKind::VecConstruction {
            type_syntax: vectors[1],
            open_paren_span: token("(", 1),
            open_bracket_span: token("[", 0),
            elements,
            close_bracket_span: token("]", 0),
            close_paren_span: token(")", 1),
        },
    });
    let last = usize::from(matches!(case, Case::Duplicate));
    statements.push(RawStatementSyntax {
        span: range("return", 0, ";", last),
        kind: RawStatementKind::Return {
            keyword_span: token("return", 0),
            value: constructor,
            semicolon_span: token(";", last),
        },
    });
    let root = range("{", 0, "}", 0);
    let function = RawFunctionSyntax {
        span: range("function", 0, "}", 0),
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "order".to_owned(), span: token("order", 0) },
        parameters: Vec::new(),
        result_type: vectors[0],
        body: RawFunctionBodySyntax {
            span: root,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: root,
                open_brace_span: token("{", 0),
                statements: (0..=u32::try_from(last).expect("statement id")).collect(),
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
        constructor,
    )
}

#[derive(Debug, PartialEq, Eq)]
struct State {
    arena: Vec<String>,
    cfg: String,
    bindings: String,
    places: Vec<raw::Place>,
    cleanup: Vec<raw::CleanupPlan>,
    types: Vec<raw::TypeId>,
    owners: OwnerState,
    known: BTreeMap<raw::PlaceId, u64>,
    counts: [usize; 4],
    credits: [usize; 5],
}

fn state(l: &PrivateVecLowerer<'_, '_, '_>) -> State {
    State {
        arena: l
            .cfg
            .arena
            .blocks
            .iter()
            .map(|b| format!("{:?}", (b.populated, &b.parameters, &b.instructions, &b.terminator)))
            .collect(),
        cfg: format!(
            "{:?}",
            (l.cfg.current, &l.cfg.incoming, l.cfg.edges, l.cfg.function_parameters_open)
        ),
        bindings: format!("{:?}", l.bindings),
        places: l.places.clone(),
        cleanup: l.cleanup_plans.clone(),
        types: l.cfg.value_types.clone(),
        owners: l.owners.clone(),
        known: l.known_string_bytes.clone(),
        counts: [
            usize::try_from(l.next_value).expect("value"),
            usize::try_from(l.next_local).expect("local"),
            l.cleanup_actions,
            l.cfg.transitions,
        ],
        credits: [
            l.cfg.reserved_values,
            l.reserved_places,
            l.cfg.reserved_transitions,
            l.reserved_cleanup_plans,
            l.reserved_cleanup_actions,
        ],
    }
}

fn seed_local(l: &mut PrivateVecLowerer<'_, '_, '_>) {
    // Direct fixture setup using real literal emission, initialization and owner transfer.
    // This is not a claim that the complete invalid function passed IR verification.
    let value = l.value(0, l.element).expect("preceding String literal");
    let at = span(l.input.sources(), l.function.body.statements[0].span);
    let place = raw::PlaceId(u32::try_from(l.places.len()).expect("local id"));
    l.places.push(raw::Place {
        id: place,
        ty: l.element.ir,
        span: at,
        kind: raw::PlaceKind::Local(l.next_local),
    });
    l.next_local += 1;
    assert!(l.cfg.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::InitializePlace { place, value }
        },
        l.errors
    ));
    let delta = l.owners.rename(value, place).expect("real initializer owner");
    crate::data_ownership_v1::owner_state::apply_owner_delta(&mut l.known_string_bytes, delta);
    l.bindings.insert(
        "seed".to_owned(),
        crate::data_ownership_v1::Binding { ty: l.element, place, mutable: false },
    );
    assert_eq!(l.owners.pending(), &[place]);
}

fn retained_move(
    l: &PrivateVecLowerer<'_, '_, '_>,
    mut before: State,
    at: zryna_source::Span,
) -> State {
    let source = l.bindings["seed"].place;
    let value = raw::ValueId(u32::try_from(before.counts[0]).expect("result"));
    let owner = raw::PlaceId(u32::try_from(before.places.len()).expect("owner"));
    let block = l.cfg.current_block().expect("entry");
    let mut instructions = block.instructions.clone();
    let actual = instructions.pop().expect("retained first move");
    assert_eq!(
        actual,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: l.element.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source }
        }
    );
    assert_eq!(
        before.arena[0],
        format!("{:?}", (block.populated, &block.parameters, &instructions, &block.terminator))
    );
    instructions.push(actual);
    before.arena[0] =
        format!("{:?}", (block.populated, &block.parameters, &instructions, &block.terminator));
    before.places.push(raw::Place {
        id: owner,
        ty: l.element.ir,
        span: at,
        kind: raw::PlaceKind::Temporary(value),
    });
    before.types.push(l.element.ir);
    before.owners =
        OwnerState { pending: vec![owner], value_owners: BTreeMap::from([(value, owner)]) };
    before.known = BTreeMap::from([(owner, 1)]);
    before.counts[0] += 1;
    before.counts[3] += 1;
    before
}

#[allow(clippy::too_many_lines)]
fn exercise(case: Case, exhausted: usize) {
    let (source, snapshot, constructor) = fixture(case);
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(snapshot, &sources).expect("source-authenticated ordering fixture");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = crate::data_ownership_v1::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("layout");
    let node_types = crate::data_ownership_v1::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|t| t.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("Vec");
    let element_layout =
        layouts.type_by_id(vec_ty.layout).expect("Vec layout").referenced_type().expect("element");
    let element = node_types
        .iter()
        .flatten()
        .find(|t| t.layout == element_layout)
        .copied()
        .expect("element type");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let parent =
        span(&sources, function.body.expressions[usize::try_from(constructor).expect("id")].span);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    for _ in 0..2 {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(parent, &mut errors).expect("entry");
        let mut l = PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            layouts: &layouts,
            catalog: &catalog,
            vec_ty,
            element,
            errors: &mut errors,
            bindings: BTreeMap::new(),
            places: Vec::new(),
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState::default(),
            known_string_bytes: BTreeMap::new(),
            next_value: 0,
            next_local: 0,
        };
        if matches!(case, Case::Duplicate) {
            seed_local(&mut l);
        }
        // Synthetic held-capacity frontiers, not source-reachable maximum-size programs.
        if exhausted != 0 {
            match case {
                Case::Integer if exhausted == 1 => {
                    l.cfg.reserved_values = ir::MAX_VALUES_PER_FUNCTION;
                }
                Case::Integer => l.reserved_places = ir::MAX_PLACES_PER_FUNCTION,
                _ => l.reserved_cleanup_plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
            }
        }
        let before = state(&l);
        assert!(l.value(constructor, vec_ty).is_none());
        let expected = match case {
            Case::Integer if exhausted == 1 => Diagnostic::error_at(
                "ZRYNA-M3201",
                parent,
                format!(
                    "owned CFG values exceed the per-function M3 limit of {}",
                    ir::MAX_VALUES_PER_FUNCTION
                ),
                "reduce owned function parameters, block parameters, and result-producing expressions",
            ),
            Case::Integer if exhausted != 0 => Diagnostic::error_at(
                "ZRYNA-M3201",
                parent,
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned parameters, expressions, and local declarations",
            ),
            Case::Integer => Diagnostic::error_at(
                "ZRYNA-M3013",
                span(&sources, function.body.expressions[0].span),
                "Vec element integer is outside i32",
                "use an in-range i32 element",
            ),
            Case::Unknown => Diagnostic::error_at(
                "ZRYNA-M3013",
                span(&sources, function.body.expressions[1].span),
                "Vec<String> element expression is outside checked String preparation",
                "use a String literal, available String move, clone, concat, or private String call",
            ),
            Case::Duplicate if exhausted != 0 => Diagnostic::error_at(
                "ZRYNA-M3201",
                parent,
                "recursive owned String preparation exceeds the per-function cleanup limits",
                "reduce nested String-producing expressions or simultaneously live owners",
            ),
            Case::Duplicate => Diagnostic::error_at(
                "ZRYNA-M3014",
                span(&sources, function.body.expressions[2].span),
                "Vec String element has no available owner",
                "move each String element at most once",
            ),
        };
        if matches!(case, Case::Duplicate) && exhausted == 0 {
            assert_eq!(
                state(&l),
                retained_move(&l, before, span(&sources, function.body.expressions[1].span)),
                "legacy first move remains; no constructor result or second move"
            );
        } else {
            assert_eq!(state(&l), before);
        }
        drop(l);
        assert_eq!(errors.finish(), vec![expected]);
    }
}

#[test]
fn vec_order_copy_parent_capacity_precedes_out_of_range_child() {
    for exhausted in [0, 1, 2] {
        exercise(Case::Integer, exhausted);
    }
}

#[test]
fn vec_order_string_estimator_unknown_precedes_cleanup_exhaustion() {
    for exhausted in [0, 1] {
        exercise(Case::Unknown, exhausted);
    }
}

#[test]
fn vec_order_duplicate_move_follows_enclosing_cleanup_preflight() {
    for exhausted in [0, 1] {
        exercise(Case::Duplicate, exhausted);
    }
}
