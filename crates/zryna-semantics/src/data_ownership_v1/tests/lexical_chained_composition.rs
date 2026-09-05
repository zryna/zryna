use super::generic_vec_fixture::ordinary_array_composition_fixture::lexical_chained_fixture::{
    Case, composed_fixture,
};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn lexical_chained_static_siblings_keep_exact_disjoint_regions_and_reverse_end_order() {
    for owned in [false, true] {
        let (source, raw) = composed_fixture(false, owned, true, [Some(0), None], Case::Pair);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated static siblings");
        for _ in 0..2 {
            let program =
                lower(pair_input(&syntax, &sources)).unwrap_or_else(|e| panic!("{source}: {e:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("caller");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let bindings =
                instructions.iter().filter_map(|i| i.indexed_binding()).collect::<Vec<_>>();
            assert_eq!(bindings.len(), 2);
            assert_ne!(bindings[0].container(), bindings[1].container());
            let mut roots = Vec::new();
            for (expected, bind) in bindings.iter().enumerate() {
                assert_eq!(bind.access(), VerifiedBorrowAccess::Exclusive);
                let place = function
                    .places()
                    .find(|p| p.id() == bind.container())
                    .expect("static inner container");
                let VerifiedPlaceKind::FixedArrayConstant { base, index } = place.kind() else {
                    panic!("static sibling")
                };
                assert_eq!(index, u32::try_from(expected).expect("index"));
                roots.push(base);
            }
            assert_eq!(roots[0], roots[1]);
            let bounds =
                instructions.iter().filter(|i| i.indexed_borrow().is_some()).collect::<Vec<_>>();
            assert_eq!(bounds.len(), 2);
            assert_eq!(
                bounds[1].failure_ended_borrows().collect::<Vec<_>>(),
                [bindings[0].borrow()]
            );
            assert_eq!(
                instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .map(|i| i.borrow())
                    .collect::<Vec<_>>(),
                [Some(bindings[1].borrow()), Some(bindings[0].borrow())]
            );
        }
    }
}

#[test]
fn lexical_chained_dynamic_regions_reject_unequal_exclusive_indices_but_allow_shared_pairs() {
    for vector in [false, true] {
        for owned in [false, true] {
            for exclusive in [false, true] {
                let (source, raw) =
                    composed_fixture(vector, owned, exclusive, [None, None], Case::Pair);
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated dynamic pair");
                if exclusive {
                    let reject = || {
                        lower(pair_input(&syntax, &sources)).expect_err("whole-container conflict")
                    };
                    let errors = reject();
                    assert!(!errors.is_empty());
                    assert_eq!(errors, reject());
                } else {
                    let program = lower(pair_input(&syntax, &sources))
                        .unwrap_or_else(|e| panic!("{source}: {e:?}"));
                    let function = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("caller");
                    let bindings = function
                        .blocks()
                        .next()
                        .expect("block")
                        .instructions()
                        .filter_map(|i| i.indexed_binding())
                        .collect::<Vec<_>>();
                    assert_eq!(bindings.len(), 2);
                    assert!(bindings.iter().all(|b| b.access() == VerifiedBorrowAccess::Shared));
                }
            }
        }
    }
}

#[test]
fn lexical_chained_calls_pass_only_final_bound_child_after_source_ordered_rhs() {
    for vector in [false, true] {
        for owned in [false, true] {
            for exclusive in [false, true] {
                let (source, raw) =
                    composed_fixture(vector, owned, exclusive, [None, None], Case::Call);
                let caller_raw = &raw.files[0].functions[0];
                let source_call = caller_raw
                    .body
                    .expressions
                    .iter()
                    .find_map(|e| match &e.kind {
                        RawExpressionKind::Call { callee, arguments, .. }
                            if callee.text == "consume" =>
                        {
                            Some(arguments.clone())
                        }
                        _ => None,
                    })
                    .expect("source call");
                assert!(matches!(&caller_raw.body.expressions[source_call[0] as usize].kind,
                    RawExpressionKind::Reference { name } if name.text == "loan"));
                let rhs_span = caller_raw.body.expressions[source_call[1] as usize].span;
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated chained call");
                for _ in 0..2 {
                    let program = lower(pair_input(&syntax, &sources))
                        .unwrap_or_else(|e| panic!("{source}: {e:?}"));
                    let function = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("caller");
                    let instructions =
                        function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                    let bind_at = instructions
                        .iter()
                        .position(|i| i.indexed_binding().is_some())
                        .expect("bind");
                    let bind = instructions[bind_at].indexed_binding().expect("final child");
                    let rhs_at = instructions
                        .iter()
                        .position(|i| i.callee().is_some_and(|c| c.declaration() == 3))
                        .expect("RHS");
                    let call_at = instructions
                        .iter()
                        .position(|i| i.callee().is_some_and(|c| c.declaration() == 4))
                        .expect("borrowed callee");
                    assert!(bind_at < rhs_at && rhs_at < call_at);
                    assert_eq!(instructions[rhs_at].span(), span(&sources, rhs_span));
                    let rhs = instructions[rhs_at].result().expect("RHS value");
                    assert_eq!(
                        instructions[call_at].call_arguments().collect::<Vec<_>>(),
                        [
                            VerifiedCallArgument::Value(rhs),
                            VerifiedCallArgument::Borrow(bind.borrow())
                        ]
                    );
                    for at in [rhs_at, call_at] {
                        assert_eq!(
                            instructions[at].failure_ended_borrows().collect::<Vec<_>>(),
                            [bind.borrow()]
                        );
                        if vector || owned {
                            assert!(
                                instructions[at]
                                    .derived_drop_actions()
                                    .any(|d| d.root() == bind.container())
                            );
                        }
                    }
                    if owned {
                        let rhs_owner = function
                            .places()
                            .find(|p| p.kind() == VerifiedPlaceKind::Temporary(rhs))
                            .expect("RHS owner");
                        assert!(
                            !instructions[call_at]
                                .derived_drop_actions()
                                .any(|d| d.root() == rhs_owner.id())
                        );
                    }
                    let ends = instructions
                        .iter()
                        .enumerate()
                        .filter(|(_, i)| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .collect::<Vec<_>>();
                    assert_eq!(ends.len(), 1);
                    assert!(call_at < ends[0].0);
                    assert_eq!(ends[0].1.borrow(), Some(bind.borrow()));
                    let callee = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .nth(4)
                        .expect("callee");
                    let formal = callee.borrow_parameters().next().expect("borrow formal");
                    assert_eq!(formal.referent(), bind.referent());
                    assert_eq!(formal.access(), bind.access());
                }
            }
        }
    }
}
