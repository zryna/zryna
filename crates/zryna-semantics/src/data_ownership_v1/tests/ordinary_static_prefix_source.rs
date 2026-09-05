use super::generic_vec_fixture::ordinary_array_composition_fixture::ordinary_static_prefix_fixture::fixture;
use super::*;

#[test]
fn ordinary_static_prefix_replacement_clones_disjoint_vec_sibling_with_exact_cleanup_order() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated disjoint RHS clone");
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("caller");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let bounds = instructions
            .iter()
            .enumerate()
            .filter(|(_, i)| i.indexed_borrow().is_some())
            .collect::<Vec<_>>();
        assert_eq!(bounds.len(), 2);
        let destination = bounds[0].1.indexed_borrow().expect("destination");
        let source = bounds[1].1.indexed_borrow().expect("source");
        assert_eq!(destination.access(), VerifiedBorrowAccess::Exclusive);
        assert_eq!(source.access(), VerifiedBorrowAccess::Shared);
        let mut roots = Vec::new();
        for (index, region) in [destination.container(), source.container()].into_iter().enumerate()
        {
            let place = function.places().find(|p| p.id() == region).expect("Vec region");
            let VerifiedPlaceKind::FixedArrayConstant { base, index: actual } = place.kind() else {
                panic!("maximal static prefix")
            };
            assert_eq!(actual, u32::try_from(index).expect("sibling"));
            roots.push(base);
        }
        assert_eq!(roots[0], roots[1]);
        assert_ne!(destination.container(), source.container());
        assert!(instructions.iter().all(|i| i.indexed_projection().is_none()));
        let calls = instructions
            .iter()
            .enumerate()
            .filter(|(_, i)| i.callee().is_some())
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1.callee().expect("firstIndex").declaration(), 1);
        assert_eq!(calls[1].1.callee().expect("secondIndex").declaration(), 2);
        assert!(calls[0].0 < bounds[0].0 && bounds[0].0 < calls[1].0 && calls[1].0 < bounds[1].0);
        assert_eq!(calls[1].1.failure_ended_borrows().collect::<Vec<_>>(), [destination.borrow()]);
        assert_eq!(bounds[1].1.failure_ended_borrows().collect::<Vec<_>>(), [destination.borrow()]);
        let clone_at =
            instructions.iter().position(|i| i.generic_clone().is_some()).expect("RHS clone");
        let clone = instructions[clone_at].generic_clone().expect("clone");
        assert_eq!(
            instructions[clone_at].failure_ended_borrows().collect::<Vec<_>>(),
            [source.borrow(), destination.borrow()]
        );
        assert_eq!(
            instructions[clone_at].derived_drop_actions().map(|d| d.root()).collect::<Vec<_>>(),
            [roots[0]]
        );
        let commit_at =
            instructions.iter().position(|i| i.borrow_replacement().is_some()).expect("commit");
        let commit = instructions[commit_at].borrow_replacement().expect("replacement");
        assert_eq!(commit.borrow(), destination.borrow());
        assert_eq!(commit.value(), clone.result());
        assert_eq!(instructions[clone_at + 1].kind(), VerifiedInstructionKind::EndBorrow);
        assert_eq!(instructions[clone_at + 1].borrow(), Some(source.borrow()));
        assert!(clone_at + 1 < commit_at);
        assert_eq!(instructions[commit_at + 1].borrow(), Some(destination.borrow()));
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    }
}

#[test]
fn ordinary_static_prefix_same_vec_rhs_clone_remains_a_conservative_conflict() {
    let (source, raw) = fixture(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated same-region RHS clone");
    let reject = || lower(pair_input(&syntax, &sources)).expect_err("same Vec remains exclusive");
    let errors = reject();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3014");
    assert_eq!(
        errors[0].message(),
        "Vec operation conflicts with an active whole-container access"
    );
    assert_eq!(errors, reject());
}
