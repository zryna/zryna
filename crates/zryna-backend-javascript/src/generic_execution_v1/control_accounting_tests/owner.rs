//! Same-seal effect eligibility and checked actual census boundary; no forged owner.
use super::*;
use crate::generic_execution_v1::{Context, runtime::accounting};
#[test]
fn control_effect_owner_prepays_exact_scan_extra_overflow_and_retry() {
    source::with_program("control-live-count-call-loop", "native", |program, _| {
        let context = Context::new(program).expect("actual owner context");
        context.work.set(0);
        let plan = accounting::Plan::derive(&context)
            .expect("paid actual scan")
            .expect("control activation");
        assert!(plan.bound(&context));
        let cost = context.work.get();
        let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
        context.work.set(limit - cost);
        assert!(accounting::Plan::derive(&context).expect("exact full scan").is_some());
        assert_eq!(context.work.get(), limit);
        for start in [limit - context.types.len() + 1, usize::MAX] {
            context.work.set(start);
            let Err(error) = accounting::Plan::derive(&context) else {
                panic!("missing first debit");
            };
            assert_eq!(error.code(), "ZRYNA-J2003");
            assert_eq!(context.work.get(), start);
        }
        context.work.set(0);
        assert!(accounting::Plan::derive(&context).expect("pristine retry").is_some());
        assert_eq!(context.work.get(), cost);
        source::with_program("control-live-count-call-loop", "native", |other, _| {
            assert!(
                !plan.bound(&Context::new(other).expect("second genuine byte-equivalent issuer"))
            );
        });
    });
}
