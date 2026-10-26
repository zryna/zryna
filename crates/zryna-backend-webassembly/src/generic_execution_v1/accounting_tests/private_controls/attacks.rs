//! Hostile actual bytes must fail without an emitter body table as authority.
use super::*;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{BinaryReader, FunctionBody, Operator as O};
fn change(body: &[u8], kind: u8, m: &Model<'_, '_, '_>) -> Vec<u8> {
    let function = FunctionBody::new(BinaryReader::new(body, 0));
    let mut reader = function.get_operators_reader().expect("actual operators");
    while !reader.eof() {
        let start =
            usize::try_from(reader.original_position()).expect("bounded actual operator offset");
        let op = reader.read().expect("actual operator");
        let end = usize::try_from(reader.original_position()).expect("bounded actual operator end");
        let replacement = match (kind, &op) {
            (0, O::I32Const { value: 4_194_304 }) => Some(I::I32Const(4_194_305)),
            (1, O::GlobalSet { global_index: 12 }) => Some(I::GlobalSet(11)),
            (2, O::I32Sub) => Some(I::I32Add),
            (3, O::Call { function_index: 1 }) => Some(I::Call(3)),
            (4, O::GlobalGet { global_index: 12 }) => Some(I::GlobalGet(8)),
            (5, O::GlobalSet { global_index: 11 }) => Some(I::GlobalSet(10)),
            (6, O::Call { function_index }) if *function_index == m.string_call(1) => {
                Some(I::Call(1))
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            let mut out = body[..start].to_vec();
            replacement.encode(&mut out);
            out.extend_from_slice(&body[end..]);
            return out;
        }
        if kind == 7 {
            let mut out = body[..start].to_vec();
            I::I32Const(0).encode(&mut out);
            I::GlobalSet(11).encode(&mut out);
            out.extend_from_slice(&body[start..]);
            return out;
        }
    }
    panic!("hostile control {kind} must change an actual owned operator")
}
#[test]
fn accounting_guard_rollback_wrapper_reset_and_source_bypass_hostiles_are_rejected_independently() {
    with_empty(|m, wire| {
        let code = code::build(m).expect("actual code");
        let bytes = artifact::encode(m, &code).expect("actual module");
        artifact::audit(&bytes, m, &code).expect("original full audit");
        artifact::independent_audit(&bytes, m).expect("original independent decoded authority");
        let directory = std::env::temp_dir()
            .join(format!("m416-wasm-accounting-hostiles-{}", std::process::id()));
        std::fs::create_dir(&directory).expect("new evidence");
        std::fs::write(directory.join("original.wasm"), &bytes).expect("original bytes");
        std::fs::write(directory.join("program.zir"), wire).expect("genuine seal wire");
        let source = m
            .program
            .operations()
            .find(|op| {
                matches!(
                    op.kind(),
                    zryna_ir::generic_v1::execution_v1::raw::Operation::StringFromUtf8 { .. }
                )
            })
            .expect("genuine String operation")
            .site()
            .function;
        let mut receipts = Vec::new();
        for kind in 0..8 {
            let target = if kind < 4 {
                m.accounting_base() as usize
            } else if kind < 6 {
                m.wrapper_base as usize
            } else {
                m.source_base as usize + source
            };
            let wrong = harness::body_replacement(&bytes, target, |body| change(body, kind, m));
            std::fs::write(directory.join(format!("hostile-{kind}.wasm")), &wrong)
                .expect("retain actual hostile artifact");
            let result = artifact::independent_audit(&wrong, m);
            receipts.push(serde_json::json!({"hostile":kind,"target":target,"error":result.as_ref().err().map(ToString::to_string)}));
            std::fs::write(
                directory.join("actual-before-checks.json"),
                serde_json::to_vec_pretty(&receipts).expect("receipts"),
            )
            .expect("retain independent result");
            assert!(result.is_err(), "hostile {kind} cannot inherit emitter authority");
        }
    });
}
