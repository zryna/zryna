//! Real borrowed source emission owns its debit; a failed first debit publishes no code or writer progress.
use super::{artifact, code, memory, model, source};
#[test]
fn borrowed_code_first_debit_overflow_and_pristine_retry_preserve_actual_bytes() {
    source::with_program("branch-arm-cleanup", "native", |program, _| {
        let plan =
            memory::verify(memory::derive(program, true).expect("genuine geometry"), program)
                .expect("same-seal geometry");
        let baseline = model::Model::new(program, &plan).expect("genuine model");
        baseline.work.set(0);
        let original = code::build(&baseline).expect("actual borrowed emission baseline");
        let cost = baseline.work.get();
        assert!(cost > 0);
        let bytes = artifact::encode(&baseline, &original).expect("actual baseline bytes");
        for start in [crate::generic_execution_v1::MAX_BYTES, usize::MAX] {
            let model = model::Model::new(program, &plan).expect("same retained model");
            model.work.set(start);
            let writer = model.writer.get();
            let Err(error) = code::build(&model) else {
                panic!("unfunded first code index reserve must reject");
            };
            assert_eq!(error.code(), "ZRYNA-W4101");
            assert_eq!(model.work.get(), start);
            assert_eq!(model.writer.get(), writer);
            model.work.set(0);
            let retry = code::build(&model).expect("pristine retry after first-debit failure");
            assert_eq!(model.work.get(), cost);
            assert_eq!(artifact::encode(&model, &retry).expect("retry actual bytes"), bytes);
        }
        let model = model::Model::new(program, &plan).expect("exact credit model");
        model.work.set(crate::generic_execution_v1::MAX_BYTES - cost);
        let exact = code::build(&model).expect("exact complete own-phase code credit");
        assert_eq!(model.work.get(), crate::generic_execution_v1::MAX_BYTES);
        // Artifact serialization has its separate retained-work charges.
        model.work.set(0);
        assert_eq!(artifact::encode(&model, &exact).expect("exact emission bytes"), bytes);
    });
}
