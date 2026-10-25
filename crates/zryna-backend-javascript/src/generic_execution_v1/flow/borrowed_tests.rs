//! The real private target phase uses genuine sealed views; raw hostiles issue no authority.
use super::{Context, Function, Writer, enum_match, raw};
use crate::generic_execution_v1::borrowed_match_tests::with_program;
fn selected(check: impl FnOnce(&Context<'_, '_>, &Function<'_, '_>, &raw::Terminator)) {
    with_program("nominal-shared-clone", "native", |program, _wire| {
        let c = Context::new(program).expect("actual same-seal JS indices");
        let (f, t) = c
            .functions
            .iter()
            .find_map(|f| {
                f.blocks.iter().find_map(|b| {
                    let t = b.terminator();
                    matches!(
                        t,
                        raw::Terminator::ClosedEnumMatch { mode: raw::MatchMode::SharedLoan, .. }
                    )
                    .then_some((f, t))
                })
            })
            .expect("genuine shared selected source terminator");
        check(&c, f, t);
    });
}
fn run(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    t: &raw::Terminator,
    w: &mut Writer,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } = t else {
        panic!("exact fixture match")
    };
    enum_match(c, f, *scrutinee, *mode, arms, w)
}
#[test]
fn borrowed_match_phase_exact_extra_overflow_and_pristine_retry_preserve_budget_and_bytes() {
    selected(|c, f, t| {
        let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
        c.work.set(0);
        let mut baseline = Writer::counter(limit);
        run(c, f, t, &mut baseline).expect("actual selected binding target phase");
        let cost = c.work.get();
        c.work.set(limit - cost);
        let mut exact = Writer::counter(limit);
        run(c, f, t, &mut exact).expect("exact complete selected target work");
        assert_eq!(c.work.get(), limit);
        assert_eq!(exact.bytes(), baseline.bytes());
        c.work.set(limit - cost + 1);
        let mut short = Writer::counter(limit);
        assert_eq!(
            run(c, f, t, &mut short).expect_err("one missing phase unit rejects").code(),
            "ZRYNA-J2003"
        );
        c.work.set(usize::MAX);
        let mut overflow = Writer::counter(limit);
        assert_eq!(
            run(c, f, t, &mut overflow)
                .expect_err("overflow before first selected traversal")
                .code(),
            "ZRYNA-J2003"
        );
        assert_eq!(c.work.get(), usize::MAX);
        assert_eq!(overflow.bytes(), 0);
        c.work.set(0);
        let mut retry = Writer::counter(limit);
        run(c, f, t, &mut retry).expect("pristine sealed-view retry");
        assert_eq!(c.work.get(), cost);
        assert_eq!(retry.bytes(), baseline.bytes());
    });
}
#[test]
fn wrong_mode_source_and_generated_definition_never_select_an_unrelated_loan_address() {
    selected(|c, f, t| {
        run(c, f, t, &mut Writer::counter(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES))
            .expect("genuine source-bound selected address positive");
        for mutate in 0..5 {
            let mut wrong = t.clone();
            let raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } = &mut wrong else {
                unreachable!()
            };
            match mutate {
                0 => *mode = raw::MatchMode::Value,
                1 => *scrutinee = raw::PlaceOrLoan::Place(0),
                2 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.id = 999;
                    }
                }
                3 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.access = raw::Access::Exclusive;
                    }
                }
                4 => {
                    if let Some(raw::MatchBinding::Loan(d)) = &mut arms[1].binding {
                        d.referent = u32::MAX;
                    }
                }
                _ => unreachable!(),
            }
            assert_eq!(
                run(
                    c,
                    f,
                    &wrong,
                    &mut Writer::counter(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
                )
                .expect_err("private raw hostile cannot manufacture a different verified lens")
                .code(),
                "ZRYNA-I7001"
            );
        }
    });
}
