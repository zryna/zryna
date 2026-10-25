//! A valid typed replacement of the genuine source body cannot acquire module authority.
use super::{harness::transport, with_model};
use crate::generic_execution_v1::{artifact, code};
use wasm_encoder::Instruction as I;
#[test]
fn status_only_source_body_cannot_replace_the_genuine_private_step() {
    with_model("control-live-count-call-loop", "native", |m, _| {
        let code = code::build(m).expect("actual same-program source code");
        let original = artifact::encode(m, &code).expect("ordinary bytes");
        artifact::audit(&original, m, &code).expect("complete pristine source proof");
        let steps = m
            .functions
            .iter()
            .filter(|f| {
                f.parameters().len() == 0
                    && f.result_type()
                        == zryna_ir::generic_v1::execution_v1::raw::ValueType::Stored(1)
                    && !m.program.export_functions().contains(&f.index())
            })
            .collect::<Vec<_>>();
        assert_eq!(steps.len(), 1);
        let index = usize::try_from(m.source_base).expect("bounded base") + steps[0].index();
        let mutated = transport::body_replacement(&original, index, |_| {
            let mut body = crate::generic_execution_v1::bytes::Bytes::new();
            body.extend(&[0]).expect("no locals");
            body.op(&I::I32Const(0)).expect("fake success");
            body.op(&I::End).expect("end");
            body.bytes
        });
        wasmparser::Validator::new()
            .validate_all(&mutated)
            .expect("hostile must be a valid typed module");
        assert!(
            artifact::audit(&mutated, m, &code).is_err(),
            "no fabricated source success from a replacement body"
        );
    });
}
