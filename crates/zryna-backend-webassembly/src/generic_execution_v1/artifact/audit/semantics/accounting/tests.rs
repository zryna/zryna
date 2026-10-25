//! The actual streaming checker prepays before touching its real operator reader.
use super::*;
#[test]
fn exact_first_decoder_credit_overflow_and_pristine_retry_preserve_reader_on_failure() {
    crate::generic_execution_v1::tests::concat_program("same-source", |program, _| {
        let memory = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false).expect("claim"),
            program,
        )
        .expect("plan");
        let m = Model::new(program, &memory).expect("actual model");
        assert!(m.accounted());
        let code = crate::generic_execution_v1::code::build(&m).expect("actual byte owner");
        let bytes = crate::generic_execution_v1::artifact::encode(&m, &code).expect("actual bytes");
        let mut index = 0;
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            if let wasmparser::Payload::CodeSectionEntry(body) = payload.expect("actual module") {
                if index == m.accounting_base() as usize {
                    let limit = crate::generic_execution_v1::MAX_BYTES;
                    let mut exact = Proof {
                        reader: body.get_operators_reader().expect("actual owned stream"),
                        model: &m,
                    };
                    m.work.set(limit - 1);
                    exact.need(&[T(1, 1)]).expect("exact one-token debit");
                    assert_eq!(m.work.get(), limit);
                    for before in [limit, usize::MAX] {
                        let mut denied = Proof {
                            reader: body.get_operators_reader().expect("actual stream"),
                            model: &m,
                        };
                        let position = denied.reader.original_position();
                        m.work.set(before);
                        assert_eq!(
                            denied.need(&[T(1, 1)]).expect_err("first debit rejects").code(),
                            "ZRYNA-W4101"
                        );
                        assert_eq!(denied.reader.original_position(), position);
                        assert_eq!(m.work.get(), before);
                    }
                    let mut retry = Proof {
                        reader: body.get_operators_reader().expect("same actual stream"),
                        model: &m,
                    };
                    m.work.set(0);
                    retry.need(&[T(1, 1)]).expect("pristine retry");
                    assert_eq!(m.work.get(), 1);
                    return;
                }
                index += 1;
            }
        }
        panic!("actual adapter body must exist");
    });
}

#[test]
fn actual_vec_first_decoder_debit_is_exact_atomic_and_retryable() {
    crate::generic_execution_v1::accounting_tests::vec_program(|program, _| {
        let memory = crate::generic_execution_v1::memory::verify(
            crate::generic_execution_v1::memory::derive(program, false).expect("claim"),
            program,
        )
        .expect("independent memory");
        let m = Model::new(program, &memory).expect("genuine Vec profile");
        assert!(m.accounted());
        let code = crate::generic_execution_v1::code::build(&m).expect("actual emitted bodies");
        let bytes =
            crate::generic_execution_v1::artifact::encode(&m, &code).expect("actual module");
        let target = m.container(crate::generic_execution_v1::model::containers::VEC_ALLOCATE);
        let mut index = 0;
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            if let wasmparser::Payload::CodeSectionEntry(body) = payload.expect("actual module") {
                if index == target {
                    let limit = crate::generic_execution_v1::MAX_BYTES;
                    let mut exact = Proof {
                        reader: body.get_operators_reader().expect("actual Vec stream"),
                        model: &m,
                    };
                    m.work.set(limit - 1);
                    exact.need(&[T(1, 2)]).expect("exact first token");
                    assert_eq!(m.work.get(), limit);
                    for before in [limit, usize::MAX] {
                        let mut denied = Proof {
                            reader: body.get_operators_reader().expect("same actual Vec stream"),
                            model: &m,
                        };
                        let position = denied.reader.original_position();
                        m.work.set(before);
                        assert_eq!(
                            denied.need(&[T(1, 2)]).expect_err("first debit").code(),
                            "ZRYNA-W4101"
                        );
                        assert_eq!(denied.reader.original_position(), position);
                        assert_eq!(m.work.get(), before);
                    }
                    let mut retry = Proof {
                        reader: body.get_operators_reader().expect("pristine owned stream"),
                        model: &m,
                    };
                    m.work.set(0);
                    retry.need(&[T(1, 2)]).expect("retry");
                    assert_eq!(m.work.get(), 1);
                    return;
                }
                index += 1;
            }
        }
        panic!("actual source-bound Vec adapter required");
    });
}
