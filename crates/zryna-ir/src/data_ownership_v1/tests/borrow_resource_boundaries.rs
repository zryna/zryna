use super::*;

// These are full raw-IR verifier boundaries, not source-admission or preflight-only
// claims. Lexical authorities need not each produce a value; borrow parameters
// must have a real use. Reading every source alias would encounter the independent
// value limit before the active-borrow limit because initialization/return also
// need values. Keep one real lexical read and read every parameter used here.
fn dense_borrow_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    lexical_count: usize,
    parameter: bool,
    sequential: bool,
) -> raw::Program {
    assert!(lexical_count > 0);
    let mut raw = shared_borrow_read_program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let first_lexical = u32::from(parameter);
    if parameter {
        function.borrow_parameters.push(raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent: raw::TypeId(1),
            access: raw::BorrowAccess::Shared,
            span,
        });
    }
    function.blocks[0].instructions.truncate(1);
    let instructions = &mut function.blocks[0].instructions;
    let mut next_value = 1;
    if parameter {
        instructions.push(read_borrow(0, next_value, span));
        next_value += 1;
    }
    for index in 0..lexical_count {
        let id = first_lexical + u32::try_from(index).expect("bounded borrow identity");
        instructions.push(begin_borrow(id, 0, raw::BorrowAccess::Shared, span));
        if index + 1 == lexical_count {
            instructions.push(read_borrow(id, next_value, span));
        }
        if sequential {
            instructions.push(end_borrow(id, span));
        }
    }
    if !sequential {
        for index in (0..lexical_count).rev() {
            instructions.push(end_borrow(
                first_lexical + u32::try_from(index).expect("bounded borrow identity"),
                span,
            ));
        }
    }
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(next_value), cleanup: raw::CleanupPlanId(0) };
    raw
}

fn read_borrow(id: u32, value: u32, span: zryna_source::Span) -> raw::Instruction {
    raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(value), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(id) },
    }
}

fn assert_verified_trace(
    raw: raw::Program,
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    lexical_count: usize,
    parameter: bool,
    expected_peak: usize,
) {
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, sources, entry, linear.clone(), linux.clone())
        .expect("dense, initialized and balanced authority passes the complete verifier");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.borrow_parameters().count(), usize::from(parameter));
    let mut active = BTreeSet::new();
    if parameter {
        active.insert(0);
    }
    let mut peak = active.len();
    let mut begins = 0;
    let mut ends = 0;
    let mut reads = 0;
    for instruction in function.blocks().next().expect("block").instructions() {
        match instruction.kind() {
            VerifiedInstructionKind::BeginBorrow => {
                let id = instruction.borrow().expect("begin identity").index();
                assert_eq!(id as usize, usize::from(parameter) + begins);
                assert!(active.insert(id));
                begins += 1;
                peak = peak.max(active.len());
            }
            VerifiedInstructionKind::BorrowRead => {
                assert!(active.contains(&instruction.borrow().expect("read identity").index()));
                reads += 1;
            }
            VerifiedInstructionKind::EndBorrow => {
                assert!(active.remove(&instruction.borrow().expect("end identity").index()));
                ends += 1;
            }
            _ => {}
        }
    }
    assert_eq!((begins, ends, reads), (lexical_count, lexical_count, 1 + usize::from(parameter)));
    assert_eq!(peak, expected_peak);
    assert_eq!(active, if parameter { BTreeSet::from([0]) } else { BTreeSet::new() });
}

fn assert_stable_limit_and_replay(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    parameter: bool,
) {
    let count = MAX_ACTIVE_BORROWS_PER_FUNCTION + 1 - usize::from(parameter);
    let hostile = dense_borrow_program(sources, linear, linux, count, parameter, false);
    let expected = vec![(
        "ZRYNA-I3201".to_owned(),
        format!(
            "DataOwnershipV1 simultaneously active borrows per function exceeds its limit of {MAX_ACTIVE_BORROWS_PER_FUNCTION}"
        ),
        None,
    )];
    let entry = sources.verify_file_id(0).expect("entry");
    for _ in 0..2 {
        let diagnostics = verify(hostile.clone(), sources, entry, linear.clone(), linux.clone())
            .expect_err("the first extra active authority fails before ownership replay");
        assert_eq!(diagnostic_trace(diagnostics), expected);
        assert_verified_trace(
            dense_borrow_program(sources, linear, linux, 2, parameter, false),
            sources,
            linear,
            linux,
            2,
            parameter,
            2 + usize::from(parameter),
        );
    }
}

#[test]
fn dense_borrow_boundary_builders_verify_small_baselines() {
    let (sources, linear, linux) = authorities();
    for parameter in [false, true] {
        for sequential in [false, true] {
            assert_verified_trace(
                dense_borrow_program(&sources, &linear, &linux, 3, parameter, sequential),
                &sources,
                &linear,
                &linux,
                3,
                parameter,
                usize::from(parameter) + if sequential { 1 } else { 3 },
            );
        }
    }
}

#[test]
fn dense_lexical_active_borrow_exact_and_first_extra_are_fully_verified() {
    let (sources, linear, linux) = authorities();
    assert_verified_trace(
        dense_borrow_program(
            &sources,
            &linear,
            &linux,
            MAX_ACTIVE_BORROWS_PER_FUNCTION,
            false,
            false,
        ),
        &sources,
        &linear,
        &linux,
        MAX_ACTIVE_BORROWS_PER_FUNCTION,
        false,
        MAX_ACTIVE_BORROWS_PER_FUNCTION,
    );
    assert_stable_limit_and_replay(&sources, &linear, &linux, false);
}

#[test]
fn parameter_and_lexical_authorities_share_the_authenticated_active_limit() {
    let (sources, linear, linux) = authorities();
    let lexical = MAX_ACTIVE_BORROWS_PER_FUNCTION - 1;
    assert_verified_trace(
        dense_borrow_program(&sources, &linear, &linux, lexical, true, false),
        &sources,
        &linear,
        &linux,
        lexical,
        true,
        MAX_ACTIVE_BORROWS_PER_FUNCTION,
    );
    assert_stable_limit_and_replay(&sources, &linear, &linux, true);
}

#[test]
fn sequential_dense_lexical_sites_may_exceed_the_active_borrow_limit() {
    let (sources, linear, linux) = authorities();
    let lexical = MAX_ACTIVE_BORROWS_PER_FUNCTION + 1;
    assert_verified_trace(
        dense_borrow_program(&sources, &linear, &linux, lexical, false, true),
        &sources,
        &linear,
        &linux,
        lexical,
        false,
        1,
    );
}
