//! Same-seal actual Plan scan budget: exact, first-debit/overflow atomicity and retry.
use super::*;
#[test]
fn genuine_string_profile_scan_has_own_prepaid_credit_and_pristine_retry() {
    crate::generic_execution_v1::tests::concat_program("same-source", |program, _| {
        let memory = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false).expect("claim"),
            program,
        )
        .expect("plan");
        let m = Model::new(program, &memory).expect("genuine retained arrays");
        let mut cost = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut cost)
                .expect("actual scan")
                .is_some()
        );
        assert!(cost > m.types.len());
        let limit = crate::generic_execution_v1::MAX_BYTES;
        let mut exact = limit - cost;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut exact)
                .expect("exact scan debit")
                .is_some()
        );
        assert_eq!(exact, limit);
        for before in [limit - m.types.len() + 1, usize::MAX] {
            let mut spent = before;
            let Err(error) = Plan::derive(program, &m.types, &m.functions, false, &mut spent)
            else {
                panic!("first scan must reject before visit")
            };
            assert_eq!(error.code(), "ZRYNA-W4101");
            assert_eq!(spent, before);
        }
        let mut retry = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut retry)
                .expect("pristine actual scan")
                .is_some()
        );
        assert_eq!(retry, cost);
        let mut observed = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, true, &mut observed)
                .expect("old observed profile")
                .is_none()
        );
        assert_eq!(observed, 0);
    });
}

#[test]
fn genuine_vec_profile_prepays_its_actual_type_function_and_operation_scans() {
    crate::generic_execution_v1::accounting_tests::vec_program(|program, _| {
        let memory = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false).expect("claim"),
            program,
        )
        .expect("independent memory");
        let m = Model::new(program, &memory).expect("actual Vec model");
        let mut cost = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut cost)
                .expect("actual Vec profile scan")
                .is_some()
        );
        let limit = crate::generic_execution_v1::MAX_BYTES;
        assert!(cost > m.types.len());
        let mut exact = limit - cost;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut exact)
                .expect("exact full scan debit")
                .is_some()
        );
        assert_eq!(exact, limit);
        for before in [limit - m.types.len() + 1, usize::MAX] {
            let mut spent = before;
            let Err(error) = Plan::derive(program, &m.types, &m.functions, false, &mut spent)
            else {
                panic!("prepaid first scan must reject")
            };
            assert_eq!(error.code(), "ZRYNA-W4101");
            assert_eq!(spent, before);
        }
        let mut retry = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut retry)
                .expect("pristine actual Vec retry")
                .is_some()
        );
        assert_eq!(retry, cost);
    });
}

#[test]
fn genuine_control_profile_has_exact_scan_extra_overflow_retry_and_observed_separation() {
    crate::generic_execution_v1::accounting_tests::control_program(|program, _| {
        let memory = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false)
                .expect("actual source claim"),
            program,
        )
        .expect("same owner memory");
        let m = Model::new(program, &memory).expect("retained type/function arrays");
        let mut cost = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut cost)
                .expect("actual prepaid scan")
                .is_some(),
            "control must own its complete invocation effects"
        );
        let limit = crate::generic_execution_v1::MAX_BYTES;
        let mut exact = limit - cost;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut exact)
                .expect("exact full credit")
                .is_some()
        );
        assert_eq!(exact, limit);
        for before in [limit - m.types.len() + 1, usize::MAX] {
            let mut spent = before;
            let Err(error) = Plan::derive(program, &m.types, &m.functions, false, &mut spent)
            else {
                panic!("missing first debit");
            };
            assert_eq!(error.code(), "ZRYNA-W4101");
            assert_eq!(spent, before);
        }
        let mut retry = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, false, &mut retry)
                .expect("pristine retry")
                .is_some()
        );
        assert_eq!(retry, cost);
        let mut observed = 0;
        assert!(
            Plan::derive(program, &m.types, &m.functions, true, &mut observed)
                .expect("observed checkpoint")
                .is_none()
        );
        assert_eq!(observed, 0);
    });
}
