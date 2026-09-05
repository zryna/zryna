use super::*;

fn indexed_sites(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    array: raw::TypeId,
    count: usize,
    simultaneous: bool,
) -> raw::Program {
    let mut program = projected_copy_borrow_program(
        sources,
        linear,
        linux,
        inner,
        array,
        ProjectedBorrowShape::FixedArray,
    );
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.places.truncate(1);
    function.blocks[0].instructions.clear();
    function.cleanup_plans.clear();
    for ordinal in 0..count {
        let id = u32::try_from(ordinal).expect("bounded test identity");
        function.cleanup_plans.push(raw::CleanupPlan {
            id: raw::CleanupPlanId(id),
            span,
            actions: vec![],
        });
        function.blocks[0].instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginIndexedBorrow {
                definition: raw::BorrowDefinition {
                    id: raw::BorrowId(id),
                    place: raw::PlaceId(0),
                    access: raw::BorrowAccess::Shared,
                    span,
                },
                index: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(id),
            },
        });
        if !simultaneous {
            function.blocks[0].instructions.push(end_borrow(id, span));
        }
    }
    if simultaneous {
        for ordinal in (0..count).rev() {
            function.blocks[0]
                .instructions
                .push(end_borrow(u32::try_from(ordinal).expect("bounded test identity"), span));
        }
    }
    let exit = raw::CleanupPlanId(u32::try_from(count).expect("bounded cleanup"));
    function.cleanup_plans.push(raw::CleanupPlan { id: exit, span, actions: vec![] });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: exit };
    program
}

#[test]
fn indexed_borrow_resource_sites_replay_without_element_place_expansion() {
    let (sources, linear, linux, inner, _, array) = projected_copy_borrow_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for simultaneous in [false, true] {
        let raw = indexed_sites(&sources, &linear, &linux, inner, array, 8, simultaneous);
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("balanced indexed authorities");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        assert_eq!(function.places().count(), 1);
        let block = function.blocks().next().expect("entry block");
        assert_eq!(block.instructions().count(), 16);
        assert_eq!(
            block
                .instructions()
                .filter(|i| i.kind() == VerifiedInstructionKind::BeginIndexedBorrow)
                .count(),
            8
        );
    }
}

#[test]
#[ignore = "full indexed active-authority exact/first-extra boundary"]
fn indexed_borrow_active_authority_exact_first_extra_and_recovery() {
    let (sources, linear, linux, inner, _, array) = projected_copy_borrow_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let exact = indexed_sites(
        &sources,
        &linear,
        &linux,
        inner,
        array,
        MAX_ACTIVE_BORROWS_PER_FUNCTION,
        true,
    );
    verify(exact, &sources, entry, linear.clone(), linux.clone())
        .expect("exact indexed active authority count");
    let extra = indexed_sites(
        &sources,
        &linear,
        &linux,
        inner,
        array,
        MAX_ACTIVE_BORROWS_PER_FUNCTION + 1,
        true,
    );
    let errors = verify(extra, &sources, entry, linear.clone(), linux.clone())
        .expect_err("first extra indexed authority");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    let recovered = indexed_sites(&sources, &linear, &linux, inner, array, 2, false);
    verify(recovered, &sources, entry, linear, linux).expect("recovery after terminal rejection");
}

#[test]
fn indexed_borrow_cleanup_counter_exact_first_extra_and_overflow_are_terminal() {
    let (sources, linear, linux, inner, _, array) = projected_copy_borrow_authorities();
    let mut raw = indexed_sites(&sources, &linear, &linux, inner, array, 1, false);
    // This lane checks preflight counters, not complete authentication of synthetic plans.
    let function = &mut raw.modules[0].functions[0];
    let template = function.cleanup_plans[0].clone();
    function.cleanup_plans.resize(MAX_CLEANUP_PLANS_PER_FUNCTION, template.clone());
    let mut exact = Errors::default();
    super::super::preflight(&raw, &linear, &mut exact);
    assert!(exact.is_empty());
    raw.modules[0].functions[0].cleanup_plans.push(template);
    let mut extra = Errors::default();
    super::super::preflight(&raw, &linear, &mut extra);
    let diagnostics = extra.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3201");
    let mut overflow = Errors::default();
    assert_eq!(
        super::super::checked_add(usize::MAX, 1, "indexed cleanup count", &mut overflow),
        usize::MAX
    );
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
}
