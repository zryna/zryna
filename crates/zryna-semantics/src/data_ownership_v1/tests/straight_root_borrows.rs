use super::*;

#[test]
fn shared_root_aliases_read_copy_values_end_in_reverse_and_restore_owner_access() {
    let sources = sources_for(SHARED_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_BORROW_JSON).expect("shared-root borrow snapshot"),
        &sources,
    )
    .expect("source-faithful shared-root borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("shared-root borrow lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::BeginBorrow,
            VerifiedInstructionKind::BorrowRead,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::BeginBorrow,
            VerifiedInstructionKind::BorrowRead,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::EndBorrow,
            VerifiedInstructionKind::EndBorrow,
            VerifiedInstructionKind::CopyFromPlace,
        ]
    );
    let ended = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .map(|instruction| instruction.borrow().expect("ended borrow").index())
        .collect::<Vec<_>>();
    assert_eq!(ended, vec![1, 0]);
    assert_eq!(function.borrow_parameters().count(), 0);
    assert_eq!(function.cleanup_plans().next().expect("return cleanup").actions().count(), 0);
}

#[test]
fn shared_root_borrow_lowering_is_deterministic() {
    let lower_once = || {
        let sources = sources_for(SHARED_ROOT_BORROW_SOURCE);
        let syntax = verify_snapshot(
            decode_snapshot(SHARED_ROOT_BORROW_JSON).expect("shared-root borrow snapshot"),
            &sources,
        )
        .expect("source-faithful shared-root borrow v4");
        let program = lower(pair_input(&syntax, &sources)).expect("shared-root borrow lowering");
        program
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
            .map(|instruction| {
                (
                    instruction.kind(),
                    instruction.borrow().map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(lower_once(), lower_once());
}

#[test]
fn root_borrow_resources_are_preflighted_at_exact_and_first_extra_limits() {
    let exact = zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION;
    assert_eq!(straight_root_borrow_budget_violation(exact, 1, 0), None);
    assert_eq!(
        straight_root_borrow_budget_violation(exact + 1, 1, 0),
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    );
    let exact_values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 2;
    assert_eq!(straight_root_borrow_budget_violation(1, exact_values, 0), None);
    assert_eq!(
        straight_root_borrow_budget_violation(1, exact_values + 1, 0),
        Some(RootBorrowBudgetLimit::Values)
    );
    assert_eq!(straight_root_borrow_budget_violation(1, 0, exact_values), None);
    assert_eq!(
        straight_root_borrow_budget_violation(1, 0, exact_values + 1),
        Some(RootBorrowBudgetLimit::Values)
    );
    assert_eq!(
        straight_root_borrow_budget_violation(usize::MAX, usize::MAX, usize::MAX),
        Some(RootBorrowBudgetLimit::Values)
    );
}

#[test]
fn shared_root_borrow_rejects_owner_replacement_before_ir_construction() {
    let sources = sources_for(SHARED_ROOT_BORROW_REPLACE_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_BORROW_REPLACE_JSON).expect("replacement snapshot"),
        &sources,
    )
    .expect("source-faithful replacement v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("borrowed replacement");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
    assert_eq!(
        diagnostics[0].message(),
        "borrow blocks cannot replace the root or an ordinary local"
    );
}

#[test]
fn shared_root_borrow_alias_cannot_escape_its_lexical_block() {
    let sources = sources_for(SHARED_ROOT_BORROW_ESCAPE_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_BORROW_ESCAPE_JSON).expect("escape snapshot"),
        &sources,
    )
    .expect("source-faithful escape v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("escaping alias");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
    assert_eq!(
        diagnostics[0].message(),
        "a lexical borrow alias or block-local read cannot escape"
    );
}

#[test]
fn shared_root_bool_uses_the_same_verified_borrow_authority() {
    let sources = sources_for(SHARED_ROOT_BORROW_BOOL_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_BORROW_BOOL_JSON).expect("bool borrow snapshot"),
        &sources,
    )
    .expect("source-faithful bool borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("bool shared-root borrow");
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
    assert_eq!(instructions[0], VerifiedInstructionKind::BoolLiteral);
    assert!(instructions.contains(&VerifiedInstructionKind::BorrowRead));
    assert_eq!(instructions.last(), Some(&VerifiedInstructionKind::CopyFromPlace));
}

#[test]
fn shared_root_owner_copy_read_remains_compatible_while_alias_is_active() {
    let sources = sources_for(SHARED_ROOT_OWNER_READ_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_OWNER_READ_JSON).expect("owner-read snapshot"),
        &sources,
    )
    .expect("source-faithful owner-read v4");
    let program = lower(pair_input(&syntax, &sources)).expect("shared owner read");
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
    let begin = instructions
        .iter()
        .position(|kind| *kind == VerifiedInstructionKind::BeginBorrow)
        .expect("begin borrow");
    let end = instructions
        .iter()
        .position(|kind| *kind == VerifiedInstructionKind::EndBorrow)
        .expect("end borrow");
    assert_eq!(
        instructions[begin + 1..end]
            .iter()
            .filter(|kind| **kind == VerifiedInstructionKind::CopyFromPlace)
            .count(),
        1,
        "one owner Copy read must remain inside the active shared-borrow interval"
    );
}

#[test]
fn shared_root_borrow_rejects_mutable_wrong_referent_and_unused_aliases() {
    for (source, snapshot, expected) in [
        (
            SHARED_ROOT_BORROW_MUTABLE_SOURCE,
            SHARED_ROOT_BORROW_MUTABLE_JSON,
            "borrow block bindings must be const",
        ),
        (
            SHARED_ROOT_BORROW_WRONG_REFERENT_SOURCE,
            SHARED_ROOT_BORROW_WRONG_REFERENT_JSON,
            "borrow alias referent does not match the root's exact scalar type",
        ),
        (
            SHARED_ROOT_BORROW_UNUSED_SOURCE,
            SHARED_ROOT_BORROW_UNUSED_JSON,
            "each lexical borrow alias must be used by an exact Copy read or write",
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(
            decode_snapshot(snapshot).expect("hostile shared-root snapshot"),
            &sources,
        )
        .expect("source-faithful hostile shared-root v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(expected);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
        assert_eq!(diagnostics[0].message(), expected);
    }
}

#[test]
fn exclusive_root_borrow_reads_writes_and_restores_owner_access() {
    let sources = sources_for(EXCLUSIVE_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(EXCLUSIVE_ROOT_BORROW_JSON).expect("exclusive-root borrow snapshot"),
        &sources,
    )
    .expect("source-faithful exclusive-root borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("exclusive-root borrow lowering");
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
    let kinds = instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::BeginBorrow,
            VerifiedInstructionKind::BorrowRead,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::BorrowWrite,
            VerifiedInstructionKind::BorrowRead,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::EndBorrow,
            VerifiedInstructionKind::CopyFromPlace,
        ]
    );
    assert_eq!(instructions[2].borrow_access(), Some(VerifiedBorrowAccess::Exclusive));
    assert_eq!(instructions[6].borrow().expect("write authority").index(), 0);
    assert_eq!(
        instructions.last().expect("restored owner read").kind(),
        VerifiedInstructionKind::CopyFromPlace
    );
}

#[test]
fn exclusive_bool_write_uses_the_same_verified_authority() {
    let sources = sources_for(EXCLUSIVE_ROOT_BORROW_BOOL_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(EXCLUSIVE_ROOT_BORROW_BOOL_JSON).expect("exclusive bool snapshot"),
        &sources,
    )
    .expect("source-faithful exclusive bool v4");
    let program = lower(pair_input(&syntax, &sources)).expect("exclusive bool lowering");
    let kinds = program
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
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::BoolLiteral).count(),
        2
    );
    assert!(kinds.contains(&VerifiedInstructionKind::BorrowWrite));
    assert!(kinds.contains(&VerifiedInstructionKind::BorrowRead));
}

#[test]
fn shared_from_shared_reborrow_resolves_to_the_same_root() {
    let sources = sources_for(SHARED_ROOT_REBORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(SHARED_ROOT_REBORROW_JSON).expect("shared reborrow snapshot"),
        &sources,
    )
    .expect("source-faithful shared reborrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("shared reborrow lowering");
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
    let begins = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .collect::<Vec<_>>();
    assert_eq!(begins.len(), 2);
    assert_eq!(
        begins
            .iter()
            .map(|instruction| instruction.borrow().expect("borrow identity").index())
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(
        begins
            .iter()
            .all(|instruction| instruction.borrow_access() == Some(VerifiedBorrowAccess::Shared))
    );
    let ended = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .map(|instruction| instruction.borrow().expect("ended borrow").index())
        .collect::<Vec<_>>();
    assert_eq!(ended, vec![1, 0]);
}

#[test]
fn complete_root_alias_conflict_matrix_fails_before_ir_construction() {
    for (source, snapshot) in [
        (BORROW_CONFLICT_SHARED_EXCLUSIVE_SOURCE, BORROW_CONFLICT_SHARED_EXCLUSIVE_JSON),
        (BORROW_CONFLICT_EXCLUSIVE_SHARED_SOURCE, BORROW_CONFLICT_EXCLUSIVE_SHARED_JSON),
        (BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_SOURCE, BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_JSON),
    ] {
        let sources = sources_for(source);
        let syntax =
            verify_snapshot(decode_snapshot(snapshot).expect("conflict snapshot"), &sources)
                .expect("source-faithful conflict v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("alias conflict");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
        assert_eq!(
            diagnostics[0].message(),
            "borrow access conflicts with an active alias of the same root"
        );
    }
}

#[test]
fn unsupported_reborrow_directions_fail_closed() {
    for (source, snapshot) in [
        (BORROW_REBORROW_MUT_FROM_SHARED_SOURCE, BORROW_REBORROW_MUT_FROM_SHARED_JSON),
        (BORROW_REBORROW_SHARED_FROM_EXCLUSIVE_SOURCE, BORROW_REBORROW_SHARED_FROM_EXCLUSIVE_JSON),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(
            decode_snapshot(snapshot).expect("unsupported reborrow snapshot"),
            &sources,
        )
        .expect("source-faithful unsupported reborrow v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unsupported reborrow");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
        assert_eq!(diagnostics[0].message(), "only shared-from-shared reborrowing is admitted");
    }
}

#[test]
fn exclusive_write_authority_type_and_owner_exclusion_fail_closed() {
    for (source, snapshot, expected) in [
        (
            BORROW_SHARED_WRITE_SOURCE,
            BORROW_SHARED_WRITE_JSON,
            "shared aliases do not grant write authority",
        ),
        (
            BORROW_EXCLUSIVE_WRONG_WRITE_SOURCE,
            BORROW_EXCLUSIVE_WRONG_WRITE_JSON,
            "exclusive-borrow writes require an exact referent-typed literal",
        ),
        (
            BORROW_EXCLUSIVE_OWNER_READ_SOURCE,
            BORROW_EXCLUSIVE_OWNER_READ_JSON,
            "owner reads are hidden while an exclusive alias is active",
        ),
        (
            BORROW_EXCLUSIVE_ROOT_WRITE_SOURCE,
            BORROW_EXCLUSIVE_ROOT_WRITE_JSON,
            "borrow blocks cannot replace the root or an ordinary local",
        ),
        (
            BORROW_EXCLUSIVE_IMMUTABLE_ROOT_SOURCE,
            BORROW_EXCLUSIVE_IMMUTABLE_ROOT_JSON,
            "exclusive borrowing requires a mutable root local",
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(
            decode_snapshot(snapshot).expect("hostile exclusive snapshot"),
            &sources,
        )
        .expect("source-faithful hostile exclusive v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(expected);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
        assert_eq!(diagnostics[0].message(), expected);
    }
}

#[test]
fn exclusive_nonreference_operand_is_rejected_by_syntax_authority() {
    let sources = sources_for(BORROW_EXCLUSIVE_NONREFERENCE_SOURCE);
    let diagnostics = verify_snapshot(
        decode_snapshot(BORROW_EXCLUSIVE_NONREFERENCE_JSON).expect("nonreference snapshot"),
        &sources,
    )
    .expect_err("borrowMut literal operand");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-Y4002");
    assert_eq!(diagnostics[0].message(), "borrow operand is not syntactically a place");
}

#[test]
fn exclusive_lowering_and_conflict_diagnostics_are_deterministic() {
    let sources = sources_for(EXCLUSIVE_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(EXCLUSIVE_ROOT_BORROW_JSON).expect("exclusive replay snapshot"),
        &sources,
    )
    .expect("source-faithful exclusive replay v4");
    let trace = || {
        let program = lower(pair_input(&syntax, &sources)).expect("exclusive replay lowering");
        program
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
            .map(|instruction| {
                (
                    instruction.kind(),
                    instruction.borrow().map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
                    instruction.borrow_access(),
                    instruction.i32_literal(),
                    instruction.bool_literal(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(trace(), trace());

    for (source, snapshot) in [
        (BORROW_CONFLICT_SHARED_EXCLUSIVE_SOURCE, BORROW_CONFLICT_SHARED_EXCLUSIVE_JSON),
        (BORROW_CONFLICT_EXCLUSIVE_SHARED_SOURCE, BORROW_CONFLICT_EXCLUSIVE_SHARED_JSON),
        (BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_SOURCE, BORROW_CONFLICT_EXCLUSIVE_EXCLUSIVE_JSON),
    ] {
        let sources = sources_for(source);
        let syntax =
            verify_snapshot(decode_snapshot(snapshot).expect("conflict replay snapshot"), &sources)
                .expect("source-faithful conflict replay v4");
        let diagnostics = || {
            lower(pair_input(&syntax, &sources))
                .expect_err("conflict replay")
                .into_iter()
                .map(|diagnostic| {
                    let primary = diagnostic.primary_span().expect("source diagnostic");
                    (
                        diagnostic.code().to_owned(),
                        diagnostic.message().to_owned(),
                        primary.start(),
                        primary.end(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(diagnostics(), diagnostics());
    }
}
