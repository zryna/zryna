//! Independent source-law structural obligations after genuine whole issuance.
use super::source;
use zryna_ir::generic_v1::execution_v1::raw;
pub(super) fn claim(case: &str, claim: &raw::Program) {
    let indexed = claim
        .functions
        .iter()
        .flat_map(|function| &function.loans)
        .filter(|loan| matches!(loan.origin, raw::LoanOrigin::Indexed { .. }))
        .count();
    if case == "copy-read-control" {
        assert_eq!(indexed, 0, "existing IndexCopy control is not new indexed loan evidence");
        assert_eq!(
            claim
                .functions
                .iter()
                .flat_map(|function| &function.blocks)
                .flat_map(|block| &block.instructions)
                .filter(|instruction| matches!(
                    instruction.operation,
                    raw::Operation::IndexCopy { .. }
                ))
                .count(),
            2
        );
        return;
    }
    if case == "zero-length-bounds" {
        // This always-failing source may retain checked indexed access or a proved Bounds trap.
        // The genuine source/type/state issuer owns that equivalence; no lexical loan becomes live.
        return;
    }
    assert!(
        indexed > 0,
        "accepted indexed place uses actual indexed authority, never a fake whole-root loan"
    );
    for function in &claim.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let raw::Operation::BeginLoan { loan, failure } = instruction.operation
                    && matches!(
                        function.loans[loan as usize].origin,
                        raw::LoanOrigin::Indexed { .. }
                    )
                {
                    assert!(
                        instruction.result.is_none(),
                        "Borrow remains an unstored loan, not owned SSA"
                    );
                    assert!(
                        failure.is_some(),
                        "indexed authority retains exact fallible bounds site"
                    );
                }
            }
        }
    }
    if matches!(case, "vec-indexed-replace" | "index-once-before-rhs" | "array-indexed-rhs-match") {
        assert!(claim.functions.iter().flat_map(|function| &function.loans).any(|loan| matches!(
            loan.origin,
            raw::LoanOrigin::Indexed { .. }
        )
            && loan.region.role == raw::RegionRole::TransientIndexedAccess));
    }
    if case == "index-once-before-rhs" {
        index_once(claim);
    }
    if case == "array-indexed-rhs-match" {
        rhs_match(claim);
    }
    if case == "chained-shared-copy" {
        assert!(indexed >= 2, "each dynamic path segment retains bounds/referent authority");
    }
}
fn index_once(claim: &raw::Program) {
    let text = source::text("index-once-before-rhs");
    let spelling = "select(borrowMut(counter), index)";
    let start = u32::try_from(text.find(spelling).expect("exact original index call"))
        .expect("bounded span");
    let end = start + u32::try_from(spelling.len()).expect("bounded spelling");
    let mut calls = 0;
    for function in &claim.functions {
        for block in &function.blocks {
            for (position, instruction) in block.instructions.iter().enumerate() {
                if instruction.site_span.start != start || instruction.site_span.end != end {
                    continue;
                }
                if !matches!(instruction.operation, raw::Operation::SourceCall { .. }) {
                    continue;
                }
                calls += 1;
                let result = instruction.result.as_ref().expect("i32 index result").id;
                let selected=block.instructions.iter().enumerate().find(|(_,instruction)| {
                    matches!(instruction.operation,raw::Operation::BeginLoan{loan,..}
                        if matches!(function.loans[loan as usize].origin,raw::LoanOrigin::Indexed{index,..} if index==result))
                }).expect("one-time index SSA is the actual checked access operand");
                assert!(position < selected.0, "index producer precedes bounds");
                let rhs = block
                    .instructions
                    .iter()
                    .position(|instruction| {
                        matches!(instruction.operation, raw::Operation::Clone { .. })
                    })
                    .expect("fallible RHS clone");
                assert!(selected.0 < rhs, "bounds/access precedes RHS clone");
                assert!(
                    block.instructions[rhs + 1..].iter().any(|instruction| matches!(
                        instruction.operation,
                        raw::Operation::ReplaceLoan { .. }
                    )),
                    "replace only after RHS"
                );
            }
        }
    }
    assert_eq!(
        calls, 1,
        "original index expression is evaluated once, including nested borrow argument"
    );
}
fn rhs_match(claim: &raw::Program) {
    let mut witnessed = false;
    for function in &claim.functions {
        let indexed = function
            .loans
            .iter()
            .filter(|loan| matches!(loan.origin, raw::LoanOrigin::Indexed { .. }))
            .map(|loan| loan.id)
            .collect::<Vec<_>>();
        if indexed.is_empty() {
            continue;
        }
        assert_eq!(
            indexed.len(),
            1,
            "RHS arms do not recheck/end-and-reborrow the selected target"
        );
        let loan = indexed[0];
        let mut begins = 0;
        let mut arm_clones = 0;
        for block in &function.blocks {
            for instruction in &block.instructions {
                if matches!(instruction.operation,raw::Operation::BeginLoan{loan:actual,..} if actual==loan)
                {
                    begins += 1;
                }
                let raw::Operation::Clone { outer_failure: Some(failure), .. } =
                    instruction.operation
                else {
                    continue;
                };
                arm_clones += 1;
                let cleanup = &function.cleanup_plans[failure as usize].actions;
                let ended = cleanup
                    .iter()
                    .position(|action| *action == raw::CleanupAction::EndLoan(loan))
                    .expect("RHS failure ends retained indexed authority before owners");
                if let Some(dropped) = cleanup
                    .iter()
                    .position(|action| matches!(action, raw::CleanupAction::DropPlace(_)))
                {
                    assert!(ended < dropped, "failure ends authority before owner cleanup");
                }
                let result = instruction.result.as_ref().expect("prepared RHS").id;
                if let Some(destination) = function
                    .places
                    .iter()
                    .find(|place| place.kind == raw::PlaceKind::Temporary(result))
                {
                    assert!(
                        !cleanup.contains(&raw::CleanupAction::DropPlace(destination.id)),
                        "failed RHS is unpublished"
                    );
                }
            }
            if matches!(block.terminator, raw::Terminator::ClosedEnumMatch { .. }) {
                witnessed = true;
            }
        }
        assert_eq!(begins, 1);
        assert_eq!(arm_clones, 2, "both normative Option arms have independent RHS sites");
    }
    assert!(witnessed, "actual source RHS Match reaches genuine CFG");
}
