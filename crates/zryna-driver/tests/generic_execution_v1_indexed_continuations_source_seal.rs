//! Additive genuine source owner controls; original dynamic16 inputs remain immutable.
#[path = "generic_execution_v1_indexed_continuations_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_indexed_continuations_source_seal/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::{self, raw};
#[test]
fn indexed_callee_bounds_failure_propagates_the_exact_caller_cleanup() {
    let case = "called-indexed-shared-copy";
    let text = source::text(case);
    let start = u32::try_from(text.find("read(loan, index)").expect("actual called source site"))
        .expect("bounded source");
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        execution_v1::check_original_bodies(&syntax, &sources).expect("genuine called original");
        source::with_program(case, provider, |_, claim, _| {
            let mut sites = 0;
            for function in &claim.functions {
                for block in &function.blocks {
                    for instruction in &block.instructions {
                        if instruction.site_span.start != start {
                            continue;
                        }
                        let raw::Operation::SourceCall { failure, .. } = instruction.operation
                        else {
                            continue;
                        };
                        sites += 1;
                        let plan = &function.cleanup_plans
                            [failure.expect("called indexed bounds is fallible") as usize];
                        assert!(
                            plan.actions
                                .iter()
                                .any(|action| matches!(action, raw::CleanupAction::EndLoan(_))),
                            "call failure ends caller's actual live parent loan"
                        );
                    }
                }
            }
            assert_eq!(sites, 1, "one genuine original call and failure site");
        });
    }
}
#[test]
fn indexed_push_retains_one_exclusive_target_through_rhs_and_growth() {
    let case = "indexed-push-copy";
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        execution_v1::check_original_bodies(&syntax, &sources)
            .expect("genuine selected Vec receiver");
        source::with_program(case, provider, |_, claim, _| {
            let mut sites = 0;
            for function in &claim.functions {
                for block in &function.blocks {
                    for instruction in &block.instructions {
                        let raw::Operation::VecPush {
                            vector: raw::PlaceOrLoan::Loan(id),
                            failure,
                            ..
                        } = instruction.operation
                        else {
                            continue;
                        };
                        sites += 1;
                        let loan = &function.loans[id as usize];
                        assert_eq!(loan.access, raw::Access::Exclusive);
                        assert_eq!(loan.region.role, raw::RegionRole::TransientIndexedAccess);
                        assert!(matches!(loan.origin, raw::LoanOrigin::Indexed { .. }));
                        let cleanup = &function.cleanup_plans[failure as usize].actions;
                        let ended = cleanup
                            .iter()
                            .position(|action| *action == raw::CleanupAction::EndLoan(id))
                            .expect("growth failure ends prepared indexed receiver");
                        if let Some(drop) = cleanup
                            .iter()
                            .position(|action| matches!(action, raw::CleanupAction::DropPlace(_)))
                        {
                            assert!(ended < drop, "authority ends before storage cleanup");
                        }
                    }
                }
            }
            assert_eq!(sites, 1, "one selected receiver push operation");
        });
    }
}
