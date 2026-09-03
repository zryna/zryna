use super::*;

// This is an authenticated internal IR boundary. Source-level borrowing still
// admits only its bounded single-loop checkpoint, not these nested source forms.
fn nested_borrow_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    nesting: usize,
) -> raw::Program {
    assert!(nesting > 0);
    let mut raw = lexical_borrow_loop_program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let header_span = function.blocks[1].terminators[0].span;
    let body_span = function.blocks[2].terminators[0].span;
    let exit_span = function.blocks[3].terminators[0].span;
    let entry = function.blocks[0].clone();
    let body = nesting + 1;
    let first_latch = nesting + 2;
    let exit = 2 * nesting + 2;
    let mut blocks = vec![entry];
    for level in 0..nesting {
        let header = level + 1;
        blocks.push(raw::Block {
            id: block_id(header),
            parameters: Vec::new(),
            instructions: vec![copy_root(header, header_span)],
            terminators: vec![raw::SpannedTerminator {
                span: header_span,
                kind: raw::Terminator::Branch {
                    condition: value_id(header),
                    when_true: edge(header + 1),
                    when_false: edge(if level == 0 { exit } else { first_latch + level - 1 }),
                },
            }],
        });
    }
    blocks.push(raw::Block {
        id: block_id(body),
        parameters: Vec::new(),
        instructions: vec![
            begin_borrow(0, 0, raw::BorrowAccess::Shared, body_span),
            raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: value_id(body),
                    ty: raw::TypeId(0),
                    span: body_span,
                }),
                span: body_span,
                kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
            },
            end_borrow(0, body_span),
        ],
        terminators: vec![raw::SpannedTerminator {
            span: body_span,
            kind: raw::Terminator::Jump(edge(first_latch + nesting - 1)),
        }],
    });
    for level in 0..nesting {
        blocks.push(raw::Block {
            id: block_id(first_latch + level),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![raw::SpannedTerminator {
                span: body_span,
                kind: raw::Terminator::Jump(edge(level + 1)),
            }],
        });
    }
    blocks.push(raw::Block {
        id: block_id(exit),
        parameters: Vec::new(),
        instructions: vec![copy_root(nesting + 2, exit_span)],
        terminators: vec![raw::SpannedTerminator {
            span: exit_span,
            kind: raw::Terminator::Return {
                value: value_id(nesting + 2),
                cleanup: raw::CleanupPlanId(0),
            },
        }],
    });
    function.blocks = blocks;
    raw
}

fn block_id(index: usize) -> raw::BlockId {
    raw::BlockId(u32::try_from(index).expect("bounded dense block"))
}

fn value_id(index: usize) -> raw::ValueId {
    raw::ValueId(u32::try_from(index).expect("bounded dense value"))
}

fn edge(target: usize) -> raw::Edge {
    raw::Edge { target: block_id(target), arguments: Vec::new() }
}

fn copy_root(value: usize, span: zryna_source::Span) -> raw::Instruction {
    raw::Instruction {
        result: Some(raw::ValueDefinition { id: value_id(value), ty: raw::TypeId(0), span }),
        span,
        kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(0) },
    }
}

#[derive(Debug, PartialEq)]
struct BlockTrace {
    id: u32,
    instructions: Vec<(VerifiedInstructionKind, Option<u32>)>,
    terminator: VerifiedTerminatorKind,
    targets: Vec<u32>,
}

fn verified_nested_trace(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    nesting: usize,
) -> Vec<BlockTrace> {
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(
        nested_borrow_program(sources, linear, linux, nesting),
        sources,
        entry,
        linear.clone(),
        linux.clone(),
    )
    .expect("real reducible nested loops preserve the initialized owner and discharge borrowing");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.places().count(), 1);
    assert_eq!(function.borrow_parameters().count(), 0);
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 2 * nesting + 3);
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert!(
        blocks
            .iter()
            .all(|block| { block.terminator().edges().all(|edge| edge.arguments().count() == 0) })
    );
    let trace = blocks
        .iter()
        .map(|block| BlockTrace {
            id: block.id().index(),
            instructions: block
                .instructions()
                .map(|instruction| {
                    (
                        instruction.kind(),
                        instruction.borrow().map(super::super::BorrowIdentity::index),
                    )
                })
                .collect(),
            terminator: block.terminator().kind(),
            targets: block.terminator().edges().map(|edge| edge.target().index()).collect(),
        })
        .collect::<Vec<_>>();
    for (index, block) in trace.iter().enumerate() {
        assert_eq!(block.id as usize, index);
    }
    assert_eq!(trace[0].targets, [1]);
    let body = nesting + 1;
    let first_latch = nesting + 2;
    let exit = 2 * nesting + 2;
    for level in 0..nesting {
        let header = level + 1;
        assert_eq!(trace[header].terminator, VerifiedTerminatorKind::Branch);
        assert_eq!(
            trace[header].targets,
            [
                block_id(header + 1).0,
                block_id(if level == 0 { exit } else { first_latch + level - 1 }).0
            ]
        );
        assert_eq!(trace[first_latch + level].terminator, VerifiedTerminatorKind::Jump);
        assert_eq!(trace[first_latch + level].targets, [block_id(header).0]);
    }
    assert_eq!(trace[body].terminator, VerifiedTerminatorKind::Jump);
    assert_eq!(trace[body].targets, [block_id(first_latch + nesting - 1).0]);
    assert_eq!(
        trace[body].instructions,
        [
            (VerifiedInstructionKind::BeginBorrow, Some(0)),
            (VerifiedInstructionKind::BorrowRead, Some(0)),
            (VerifiedInstructionKind::EndBorrow, Some(0)),
        ]
    );
    assert_eq!(trace[exit].terminator, VerifiedTerminatorKind::Return);
    assert_eq!(trace[exit].instructions, [(VerifiedInstructionKind::CopyFromPlace, None)]);
    trace
}

#[test]
fn authenticated_nested_borrow_loops_replay_the_header_latch_and_scope_trace() {
    let (sources, linear, linux) = authorities();
    for nesting in [1, 2, 3] {
        assert_eq!(
            verified_nested_trace(&sources, &linear, &linux, nesting),
            verified_nested_trace(&sources, &linear, &linux, nesting)
        );
    }
}

#[test]
fn authenticated_borrow_loop_nesting_accepts_exact_and_rejects_first_extra() {
    let (sources, linear, linux) = authorities();
    let exact = verified_nested_trace(&sources, &linear, &linux, MAX_LOOP_NESTING);
    let entry = sources.verify_file_id(0).expect("entry");
    for _ in 0..2 {
        let diagnostics = verify(
            nested_borrow_program(&sources, &linear, &linux, MAX_LOOP_NESTING + 1),
            &sources,
            entry,
            linear.clone(),
            linux.clone(),
        )
        .expect_err("first extra genuine nesting fails the full verifier");
        assert_eq!(
            diagnostic_trace(diagnostics),
            vec![(
                "ZRYNA-I3201".to_owned(),
                format!(
                    "DataOwnershipV1 verified loop nesting exceeds its limit of {MAX_LOOP_NESTING}"
                ),
                None,
            )]
        );
        assert_eq!(exact, verified_nested_trace(&sources, &linear, &linux, MAX_LOOP_NESTING));
    }
}

#[test]
fn authenticated_nested_loop_rejects_a_borrow_carried_to_its_latch() {
    let (sources, linear, linux) = authorities();
    let expected = verified_nested_trace(&sources, &linear, &linux, 2);
    let mut raw = nested_borrow_program(&sources, &linear, &linux, 2);
    raw.modules[0].functions[0].blocks[3].instructions.pop();
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear.clone(), linux.clone())
        .expect_err("the real read authority must end before the innermost latch edge");
    assert_eq!(
        diagnostic_trace(diagnostics),
        vec![(
            "ZRYNA-I3011".to_owned(),
            "borrow remains active at a control-flow edge".to_owned(),
            Some((20, 21)),
        )]
    );
    assert_eq!(expected, verified_nested_trace(&sources, &linear, &linux, 2));
}
