//! Genuine same-seal native match preflight and retained physical address controls.
use super::{Record, Shape, VerifiedFunction, raw, terminator};
use crate::generic_execution_v1::{LoanAddress, MAX_CODEGEN_UNITS, lower, lower_from, shapes};
use zryna_ir::generic_v1::execution_v1::VerifiedLoanOrigin;
use zryna_layout::StorageTarget;
#[path = "../../../../../zryna-driver/tests/generic_execution_v1_borrowed_match_source_seal/source.rs"]
mod source;
fn selected(
    check: impl FnOnce(VerifiedFunction<'_, '_>, &raw::Terminator, &[Record], &[Shape<'_>]),
) {
    source::with_program("nominal-shared-clone", "native", |program, _| {
        let mut setup = 0;
        let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut setup)
            .expect("independent checked Linux shapes");
        let (f, t) = program
            .functions()
            .find_map(|f| {
                f.blocks().find_map(|b| {
                    matches!(
                        b.terminator(),
                        raw::Terminator::ClosedEnumMatch { mode: raw::MatchMode::SharedLoan, .. }
                    )
                    .then_some((f, b.terminator()))
                })
            })
            .expect("actual original shared-match terminator");
        let values =
            super::super::values::derive(f, &types, &mut setup).expect("genuine value inventory");
        check(f, t, &values.records, &types);
    });
}
#[test]
fn genuine_borrowed_match_phase_exact_extra_overflow_and_pristine_retry() {
    selected(|f, t, values, types| {
        let mut used = 0;
        terminator(f, t, values, types, &mut used).expect("real native selected preflight");
        assert!(used > 0);
        let mut exact = MAX_CODEGEN_UNITS - used;
        terminator(f, t, values, types, &mut exact).expect("exact complete phase");
        assert_eq!(exact, MAX_CODEGEN_UNITS);
        let mut extra = MAX_CODEGEN_UNITS - used + 1;
        assert_eq!(
            terminator(f, t, values, types, &mut extra).expect_err("first extra").code(),
            "ZRYNA-N7001"
        );
        assert!(extra <= MAX_CODEGEN_UNITS);
        let mut overflow = u64::MAX;
        assert_eq!(
            terminator(f, t, values, types, &mut overflow).expect_err("overflow").code(),
            "ZRYNA-N7001"
        );
        assert_eq!(overflow, u64::MAX);
        let mut retry = 0;
        terminator(f, t, values, types, &mut retry).expect("pristine same-seal retry");
        assert_eq!(retry, used);
    });
}
#[test]
fn private_wrong_source_mode_type_or_selected_definition_cannot_borrow_another_address() {
    selected(|f, t, values, types| {
        terminator(f, t, values, types, &mut 0).expect("genuine selected positive");
        for change in 0..8 {
            let mut wrong = t.clone();
            let raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } = &mut wrong else {
                unreachable!()
            };
            match change {
                0 => *mode = raw::MatchMode::Value,
                1 => *mode = raw::MatchMode::ExclusiveLoan,
                2 => *scrutinee = raw::PlaceOrLoan::Place(0),
                3 => *ty = u32::MAX,
                4 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.id = 999;
                    }
                }
                5 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.access = raw::Access::Exclusive;
                    }
                }
                6 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.referent = u32::MAX;
                    }
                }
                7 => arms[1].ordinal = 0,
                _ => unreachable!(),
            }
            assert_eq!(
                terminator(f, &wrong, values, types, &mut 0)
                    .expect_err("raw metadata has no authority")
                    .code(),
                "ZRYNA-N7002"
            );
        }
    });
}
#[test]
fn genuine_rooted_and_formal_payload_addresses_keep_exact_offsets_and_full_owner_budget() {
    for case in ["nominal-shared-clone", "opaque-payload-forward"] {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _| {
                let mut used = 0;
                let mir = lower_from(program, &mut used).expect("actual borrowed native owner");
                let expected = addresses(&mir);
                let mut exact = MAX_CODEGEN_UNITS - used;
                let accepted = lower_from(program, &mut exact).expect("exact whole native plan");
                assert_eq!(exact, MAX_CODEGEN_UNITS);
                assert_eq!(addresses(&accepted), expected);
                let mut extra = MAX_CODEGEN_UNITS - used + 1;
                assert_eq!(
                    lower_from(program, &mut extra).err().expect("extra full plan").code(),
                    "ZRYNA-N7001"
                );
                let mut overflow = u64::MAX;
                assert_eq!(
                    lower_from(program, &mut overflow).err().expect("overflow full plan").code(),
                    "ZRYNA-N7001"
                );
                assert_eq!(overflow, u64::MAX);
                assert_eq!(addresses(&lower(program).expect("fresh full-plan retry")), expected);
                let mut count = 0;
                for f in mir.functions() {
                    for loan in f.source().loans() {
                        let Some(VerifiedLoanOrigin::StaticProjection {
                            parent,
                            projection: raw::StaticProjection::EnumPayload(_),
                        }) = loan.origin()
                        else {
                            continue;
                        };
                        let payload = mir
                            .stored_type(parent.referent())
                            .expect("actual parent layout")
                            .payload()
                            .expect("enum payload")
                            .0;
                        match (
                            f.loan(parent.id()).expect("parent address"),
                            f.loan(loan.id()).expect("selected address"),
                        ) {
                            (
                                LoanAddress::Formal { parameter: a, offset: x },
                                LoanAddress::Formal { parameter: b, offset: y },
                            ) => {
                                assert_eq!(a, b);
                                assert_eq!(u64::from(y), u64::from(x) + payload);
                            }
                            (LoanAddress::Frame(a), LoanAddress::Frame(b)) => {
                                assert_eq!(u64::from(b.offset()), u64::from(a.offset()) + payload);
                                assert_eq!(b.ty(), raw::ValueType::Stored(loan.referent()));
                            }
                            _ => panic!("projection cannot change root provenance"),
                        }
                        assert_eq!(f.loan_type(loan.id()), Some(loan.referent()));
                        count += 1;
                    }
                }
                assert!(count > 0, "actual source selected projection");
            });
        }
    }
}
fn addresses(mir: &crate::generic_execution_v1::VerifiedProgram<'_, '_>) -> Vec<String> {
    mir.functions()
        .iter()
        .flat_map(|f| {
            f.source().loans().map(move |l| {
                format!(
                    "{}:{}:{:?}:{}",
                    f.source().index(),
                    l.id(),
                    f.loan(l.id()),
                    f.loan_type(l.id()).expect("retained type")
                )
            })
        })
        .collect()
}
