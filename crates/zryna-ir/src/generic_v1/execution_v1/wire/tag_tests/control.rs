//! Independent CFG, binding and cleanup payloads; symbolic prefix has no raw count.
use crate::generic_v1::execution_v1::raw::*;
use zryna_source::UntrustedSpan;
fn edge() -> Edge {
    Edge { target: 1, arguments: vec![2] }
}
#[test]
fn terminator_tags_have_literal_edges_and_generated_bindings() {
    check!(terminator, Terminator::Return { value: None, cleanup: 1 }, "000001000000");
    check!(terminator, Terminator::Return { value: Some(1), cleanup: 2 }, "00010100000002000000");
    check!(terminator, Terminator::Jump(edge()), "01010000000100000002000000");
    check!(
        terminator,
        Terminator::Branch { condition: 3, yes: edge(), no: Edge { target: 4, arguments: vec![] } },
        "02030000000100000001000000020000000400000000000000"
    );
    check!(
        terminator,
        Terminator::ClosedEnumMatch {
            ty: 1,
            scrutinee: PlaceOrLoan::Place(2),
            mode: MatchMode::Value,
            arms: vec![EnumArm { ordinal: 0, binding: None, edge: edge() }]
        },
        "0301000000000200000000010000000000000000010000000100000002000000"
    );
    check!(
        terminator,
        Terminator::WeakUpgradeBranch {
            weak: PlaceOrLoan::Loan(1),
            success_binding: ValueDefinition { id: 2, ty: ValueType::Stored(3) },
            success: edge(),
            expired: Edge { target: 4, arguments: vec![] },
            failure: 5
        },
        "040101000000020000000003000000010000000100000002000000040000000000000005000000"
    );
    check!(
        terminator,
        Terminator::Trap { identity: TrapIdentity::Utf8, cleanup: 1 },
        "050401000000"
    );
}
#[test]
fn all_match_trap_and_cleanup_subtags_are_literal() {
    check_copy!(match_mode, MatchMode::Value, "00");
    check_copy!(match_mode, MatchMode::SharedLoan, "01");
    check_copy!(match_mode, MatchMode::ExclusiveLoan, "02");
    check!(
        match_binding,
        MatchBinding::Value(ValueDefinition { id: 1, ty: ValueType::Unit }),
        "000100000001"
    );
    check!(
        match_binding,
        MatchBinding::Loan(LoanDefinition { id: 1, referent: 2, access: Access::Shared }),
        "01010000000200000000"
    );
    check_copy!(trap_identity, TrapIdentity::Bounds, "00");
    check_copy!(trap_identity, TrapIdentity::Allocation, "01");
    check_copy!(trap_identity, TrapIdentity::Capacity, "02");
    check_copy!(trap_identity, TrapIdentity::Refcount, "03");
    check_copy!(trap_identity, TrapIdentity::Utf8, "04");
    check!(cleanup_action, CleanupAction::EndLoan(1), "0001000000");
    check!(cleanup_action, CleanupAction::DropPlace(1), "0101000000");
    check!(
        cleanup_action,
        CleanupAction::DropInitializedPrefix {
            destination: 1,
            preparation_site: PreparationSite { function: 2, block: 3, instruction: 4 }
        },
        "0201000000020000000300000004000000"
    );
    check!(
        cleanup_plan,
        CleanupPlan {
            id: 1,
            span: UntrustedSpan { file: 2, start: 3, end: 4 },
            actions: vec![CleanupAction::DropPlace(5)]
        },
        "01000000020000000300000004000000010000000105000000"
    );
    check!(
        enum_arm,
        EnumArm {
            ordinal: 1,
            binding: Some(MatchBinding::Value(ValueDefinition { id: 2, ty: ValueType::Unit })),
            edge: edge()
        },
        "0100000001000200000001010000000100000002000000"
    );
}
