//! Actual mutated Vec operators are checked without the emitter recipe table as authority.
use super::*;
use crate::generic_execution_v1::model::containers as C;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{BinaryReader, FunctionBody, Operator as O};
fn change(body: &[u8], kind: u8, m: &Model<'_, '_, '_>) -> Vec<u8> {
    let function = FunctionBody::new(BinaryReader::new(body, 0));
    let mut reader = function.get_operators_reader().expect("actual decoded body");
    while !reader.eof() {
        let start = usize::try_from(reader.original_position()).expect("bounded operator start");
        let op = reader.read().expect("actual operator");
        let end = usize::try_from(reader.original_position()).expect("bounded operator end");
        let replacement = match (kind, &op) {
            (0, O::I32Const { value: 1_048_576 }) => Some(I::I32Const(1_048_577)),
            (1, O::GlobalGet { global_index: 12 }) => Some(I::GlobalGet(8)),
            (2, O::LocalGet { local_index: 9 }) => Some(I::LocalGet(7)),
            (3, O::I32Const { value: 16 }) => Some(I::I32Const(8)),
            (4, O::I32Sub) => Some(I::I32Add),
            (5, O::Call { function_index }) if *function_index == m.container(C::VEC_ALLOCATE) => {
                Some(I::Call(m.container_base + C::VEC_ALLOCATE))
            }
            (6, O::GlobalSet { global_index: 11 }) => Some(I::GlobalSet(10)),
            _ => None,
        };
        if let Some(replacement) = replacement {
            let mut bytes = body[..start].to_vec();
            replacement.encode(&mut bytes);
            bytes.extend_from_slice(&body[end..]);
            return bytes;
        }
    }
    panic!("hostile Vec control {kind} must mutate an actual owned operator");
}
#[test]
fn vec_context_storage_credit_header_release_source_and_wrapper_hostiles_fail_independent_byte_audit()
 {
    with_vec(|m, wire| {
        let code = code::build(m).expect("actual Vec bodies");
        let bytes = artifact::encode(m, &code).expect("actual module");
        artifact::audit(&bytes, m, &code).expect("full actual audit");
        artifact::independent_audit(&bytes, m).expect("decoded original");
        let directory = std::env::temp_dir()
            .join(format!("m416-wasm-vec-accounting-hostiles-{}", std::process::id()));
        std::fs::create_dir(&directory).expect("new evidence");
        std::fs::write(directory.join("original.wasm"), &bytes).expect("original");
        std::fs::write(directory.join("program.zir"), wire).expect("genuine wire");
        let source = m
            .program
            .operations()
            .find(|op| {
                matches!(
                    op.kind(),
                    zryna_ir::generic_v1::execution_v1::raw::Operation::VecConstruct { .. }
                )
            })
            .expect("actual constructor")
            .site()
            .function;
        let mut receipts = Vec::new();
        for kind in 0..7 {
            let target = match kind {
                0 => m.container(C::VEC_ALLOCATE),
                1..=3 => m.container(C::VEC_RESERVE),
                4 => m.container(C::VEC_RELEASE),
                5 => m.source_base + u32::try_from(source).expect("retained function"),
                _ => m.wrapper_base,
            };
            let wrong = super::super::super::private_controls::harness::body_replacement(
                &bytes,
                usize::try_from(target).expect("bounded target"),
                |body| change(body, kind, m),
            );
            std::fs::write(directory.join(format!("hostile-{kind}.wasm")), &wrong)
                .expect("retained hostile");
            let result = artifact::independent_audit(&wrong, m);
            receipts.push(serde_json::json!({"kind":kind,"target":target,"sha256":format!("{:x}",Sha256::digest(&wrong)),"error":result.as_ref().err().map(ToString::to_string)}));
            std::fs::write(
                directory.join("actual-before-assertions.json"),
                serde_json::to_vec_pretty(&receipts).expect("receipt"),
            )
            .expect("retain before assertion");
            assert!(result.is_err(), "hostile {kind} cannot borrow emitter authority");
        }
    });
}
#[test]
fn impossible_raw_vec_allocate_status_cancels_prospective_credit_before_private_failure() {
    with_vec(|m, wire| {
        let h = Harness::new(
            m,
            wire,
            "vec-impossible-status",
            Some((usize::try_from(m.container_base + C::VEC_ALLOCATE).expect("raw index"), 4)),
        );
        let _deadline = super::super::super::deadline::Deadline::new(h.engine.clone());
        let (_, element) = vector(m);
        let out = scratch(m) + 64;
        let mut s = h.session();
        s.poison(usize::try_from(out).expect("out"));
        let status = s.call3("vecAllocate", (element, 1, out));
        let after = s.state();
        let output = s.record(usize::try_from(out).expect("out"));
        h.retain("raw-status4",&serde_json::json!({"status":status,"post":after,"out":output,"untrusted_status_shim":true,"source_reachability":false}));
        assert_eq!(status, 255);
        assert_eq!(after, serde_json::json!({"A":1,"L":0,"S":1,"H":0}));
        assert_eq!(output, (0, 0, 0));
    });
}
