//! Actual Proof append with a full initialized retained prefix, not an empty-vector proxy.
use super::*;
fn with_model(check: impl FnOnce(&Model<'_, '_, '_>)) {
    crate::generic_execution_v1::tests::concat_program("same-source", |program, _| {
        let plan = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false).expect("genuine claim"),
            program,
        )
        .expect("same-program memory");
        let model = Model::new(program, &plan).expect("actual source-bound model");
        check(&model);
    });
}
fn full<'m, 'p, 'a, 's>(model: &'m Model<'p, 'a, 's>) -> Proof<'m, 'p, 'a, 's> {
    let mut obligations = Vec::with_capacity(256);
    obligations.resize(obligations.capacity(), A(6, 0));
    Proof { model, obligations }
}
#[test]
fn proof_growth_pays_the_existing_initialized_prefix_and_one_new_row() {
    with_model(|model| {
        let mut proof = full(model);
        let prefix = proof.obligations.len();
        assert_eq!(prefix, proof.obligations.capacity());
        let cost = (prefix + 1) * size_of::<A>();
        model.work.set(0);
        proof.add(&[A(28, 0)]).expect("actual append baseline");
        assert_eq!(model.work.get(), cost, "relocated old obligations need their own copy credit");
        assert!(proof.obligations[prefix] == A(28, 0));
        assert!(proof.obligations[..prefix].iter().all(|atom| *atom == A(6, 0)));
        let mut exact = full(model);
        model.work.set(crate::generic_execution_v1::MAX_BYTES - cost);
        exact.add(&[A(28, 0)]).expect("exact old-prefix plus incoming credit");
        assert_eq!(model.work.get(), crate::generic_execution_v1::MAX_BYTES);
    });
}
#[test]
fn first_missing_prefix_credit_and_overflow_reject_before_reserve_without_mutation() {
    with_model(|model| {
        let prefix = full(model).obligations.len();
        let cost = (prefix + 1) * size_of::<A>();
        let limit = crate::generic_execution_v1::MAX_BYTES;
        for start in [limit - cost + 1, limit - size_of::<A>(), usize::MAX] {
            let mut proof = full(model);
            let capacity = proof.obligations.capacity();
            model.work.set(start);
            crate::generic_execution_v1::work::reset_seen();
            assert_eq!(
                proof.add(&[A(28, 0)]).expect_err("prefix debit rejects").code(),
                "ZRYNA-W4101"
            );
            assert_eq!(crate::generic_execution_v1::work::seen(), 0);
            assert_eq!(model.work.get(), start);
            assert_eq!(proof.obligations.len(), prefix);
            assert_eq!(proof.obligations.capacity(), capacity);
            assert!(proof.obligations.iter().all(|atom| *atom == A(6, 0)));
        }
        let mut retry = full(model);
        model.work.set(0);
        retry.add(&[A(28, 0)]).expect("pristine retained-prefix retry");
        assert_eq!(model.work.get(), cost);
    });
}
