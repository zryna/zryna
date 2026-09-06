use super::*;
use crate::data_ownership_v1::VerifiedBorrowAccess;

fn seed(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    struct_ty: raw::TypeId,
    place: u32,
) -> raw::Program {
    let mut raw = mixed_projected_borrow_program(sources, linear, linux, struct_ty);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].blocks[0].instructions =
        vec![begin_borrow(0, place, raw::BorrowAccess::Shared, span), end_borrow(0, span)];
    raw
}

fn verify_program(
    raw: raw::Program,
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> Result<super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
    verify(raw, sources, sources.verify_file_id(0).expect("entry"), linear.clone(), linux.clone())
}

#[test]
fn nonindexed_owned_root_and_static_subobject_borrows_have_independent_ir_authority() {
    let (sources, linear, linux, struct_ty, _, _) = mixed_aggregate_authorities();
    for place in [0, 2] {
        let verified = verify_program(
            seed(&sources, &linear, &linux, struct_ty, place),
            &sources,
            &linear,
            &linux,
        )
        .expect("owned root or static String field borrow");
        let function = verified.modules().next().expect("module").functions().next().expect("fn");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].kind(), VerifiedInstructionKind::BeginBorrow);
        assert_eq!(instructions[1].kind(), VerifiedInstructionKind::EndBorrow);
        assert_eq!(instructions[0].borrow(), instructions[1].borrow());
        assert_eq!(instructions[0].place_operands().next().expect("borrowed place").index(), place);
        assert_eq!(instructions[0].borrow_access(), Some(VerifiedBorrowAccess::Shared));
    }
}

#[test]
fn nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers() {
    let (sources, linear, linux, struct_ty, _, _) = mixed_aggregate_authorities();
    let valid = seed(&sources, &linear, &linux, struct_ty, 2);
    let mut hostile = valid.clone();
    let span = hostile.modules[0].functions[0].span;
    hostile.modules[0].functions[0].blocks[0].instructions = vec![
        begin_borrow(0, 0, raw::BorrowAccess::Exclusive, span),
        begin_borrow(1, 2, raw::BorrowAccess::Shared, span),
        end_borrow(1, span),
        end_borrow(0, span),
    ];
    let reject = || {
        diagnostic_trace(
            verify_program(hostile.clone(), &sources, &linear, &linux)
                .expect_err("root and child authorities overlap"),
        )
    };
    let first = reject();
    assert_eq!(first, reject());
    assert!(first.iter().any(|(code, _, _)| code == "ZRYNA-I3011"), "{first:?}");

    verify_program(valid, &sources, &linear, &linux)
        .expect("valid static authority recovers after hostile rejection");
}

#[test]
fn nonindexed_owned_static_place_and_lifetime_forgery_replay_deterministically() {
    let (sources, linear, linux, struct_ty, _, _) = mixed_aggregate_authorities();
    let valid = seed(&sources, &linear, &linux, struct_ty, 2);

    let mut wrong_type = valid.clone();
    wrong_type.modules[0].functions[0].places[2].ty = raw::TypeId(1);
    let mut escaping = valid.clone();
    escaping.modules[0].functions[0].blocks[0].instructions.pop();

    for (hostile, code) in [(wrong_type, "ZRYNA-I3006"), (escaping, "ZRYNA-I3011")] {
        let reject = || {
            diagnostic_trace(
                verify_program(hostile.clone(), &sources, &linear, &linux)
                    .expect_err("forged static authority"),
            )
        };
        let first = reject();
        assert_eq!(first, reject());
        assert!(first.iter().any(|(actual, _, _)| actual == code), "{first:?}");
        verify_program(valid.clone(), &sources, &linear, &linux)
            .expect("valid authority remains independently recoverable");
    }
}
