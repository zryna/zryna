//! Cross-check exact active payload roles against same-seal type and pointer-cell identities.
use super::{model, raw};
use zryna_ir::generic_v1::execution_v1::VerifiedLoanOrigin;
pub(super) fn check(model: &model::Model<'_, '_, '_>) {
    let mut borrowed = 0;
    for function in &model.functions {
        for block in function.blocks() {
            let raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } = block.terminator()
            else {
                continue;
            };
            if *mode == raw::MatchMode::Value {
                continue;
            }
            borrowed += 1;
            let raw::PlaceOrLoan::Loan(parent_id) = *scrutinee else {
                panic!("borrowed original scrutinee retains its evaluated loan");
            };
            let parent = function.loans().nth(parent_id as usize).expect("retained parent");
            assert_eq!(parent.referent(), *ty);
            let access = match mode {
                raw::MatchMode::SharedLoan => raw::Access::Shared,
                raw::MatchMode::ExclusiveLoan => raw::Access::Exclusive,
                raw::MatchMode::Value => unreachable!(),
            };
            let frame = &model.memory.claims().functions[function.index()];
            for arm in arms {
                let (_, payload) = model.types[*ty as usize]
                    .variants()
                    .nth(arm.ordinal as usize)
                    .expect("original declaration-ordered variant");
                match (&arm.binding, payload) {
                    (None, None) => {}
                    (Some(raw::MatchBinding::Loan(binding)), Some(payload)) => {
                        assert_eq!(binding.referent, payload.index());
                        assert_eq!(binding.access, access);
                        let child = function.loans().nth(binding.id as usize).expect("arm loan");
                        assert_eq!(child.referent(), payload.index());
                        assert_eq!(child.access(), access);
                        let Some(VerifiedLoanOrigin::StaticProjection { parent, projection }) =
                            child.origin()
                        else {
                            panic!("exact selected payload keeps its source parent projection");
                        };
                        assert_eq!(parent.id(), parent_id);
                        assert_eq!(projection, raw::StaticProjection::EnumPayload(arm.ordinal));
                        let cell = &frame.loans[binding.id as usize];
                        assert_eq!(cell.referent, payload.index());
                        assert_eq!(cell.access, access);
                        assert_eq!(cell.offset % 4, 0);
                        assert_ne!(cell.offset, frame.loans[parent_id as usize].offset);
                    }
                    _ => panic!("payload bindings are exact nonstored loans, never owned Values"),
                }
            }
        }
    }
    assert!(borrowed > 0, "the genuine witness must reach a borrowed terminator");
}
#[test]
fn generated_payload_cell_metadata_cannot_be_retyped_repermissioned_or_aliased() {
    for provider in super::source::PROVIDERS {
        super::source::with_program("nominal-exclusive-copy", provider, |program, _| {
            for field in 0..3 {
                let mut claim = super::memory::derive(program, false).expect("genuine memory");
                let (function, child, parent) = selected(program);
                let frame = &mut claim.functions[function];
                match field {
                    0 => {
                        frame.loans[child as usize].referent =
                            frame.loans[parent as usize].referent;
                    }
                    1 => frame.loans[child as usize].access = raw::Access::Shared,
                    _ => frame.loans[child as usize].offset = frame.loans[parent as usize].offset,
                }
                assert!(super::memory::verify(claim, program).is_err());
            }
            super::memory::verify(
                super::memory::derive(program, false).expect("pristine claim retry"),
                program,
            )
            .expect("hostile raw claims retain no authority");
        });
    }
}
fn selected(program: &super::VerifiedExecution<'_>) -> (usize, u32, u32) {
    for function in program.functions() {
        for block in function.blocks() {
            if let raw::Terminator::ClosedEnumMatch {
                scrutinee: raw::PlaceOrLoan::Loan(parent),
                arms,
                ..
            } = block.terminator()
                && let Some(raw::MatchBinding::Loan(child)) =
                    arms.iter().find_map(|a| a.binding.as_ref())
            {
                return (function.index(), child.id, *parent);
            }
        }
    }
    panic!("genuine nominal payload witness");
}
