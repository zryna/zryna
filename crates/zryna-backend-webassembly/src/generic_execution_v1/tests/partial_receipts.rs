//! Genuine same-operation partial receipts precede actual byte hostiles; no manufactured source/state proof.
use super::{artifact, code, memory, model};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, raw};
mod attacks;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_partial_canonical_empty/source.rs"]
mod empty;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_partial_static_source_seal/source.rs"]
mod source;
use attacks::Attack;
pub(super) fn program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&VerifiedExecution<'_>, &[u8]),
) {
    if case == "whole-vs-fields" {
        empty::with_program(case, provider, check);
    } else {
        source::with_program(case, provider, check);
    }
}
#[test]
fn genuine_partial_replacement_and_zero_frontier_drop_pass_both_actual_byte_owners() {
    for case in ["partial-root-replacement", "whole-vs-fields"] {
        for provider in source::PROVIDERS {
            program(case, provider, |program, _| {
                let receipt = program
                    .operations()
                    .find_map(|op| op.drop_targets().filter(|r| !r.is_whole()))
                    .expect("source-issued partial or administrative receipt");
                assert_eq!(receipt.targets().len(), usize::from(case != "whole-vs-fields"));
                for observed in [false, true] {
                    let plan = memory::verify(
                        memory::derive(program, observed).expect("actual geometry"),
                        program,
                    )
                    .expect("same-program address authority");
                    let m = model::Model::new(program, &plan).expect("same-seal retained views");
                    let code = code::build(&m).expect("exact receipt lowering");
                    let bytes = artifact::encode(&m, &code).expect("actual module");
                    artifact::independent_audit(&bytes, &m)
                        .expect("pristine independent obligations");
                    artifact::audit(&bytes, &m, &code)
                        .expect("pristine final sections/body/frame proof");
                }
            });
        }
    }
}
#[test]
fn partial_old_drop_order_target_type_address_and_empty_receipt_are_independent_actual_byte_obligations()
 {
    for attack in [
        Attack::Omitted,
        Attack::TargetType,
        Attack::TargetAddress,
        Attack::EarlyCommit,
        Attack::EmptyWholeDrop,
    ] {
        reject(attack);
    }
}
fn reject(attack: Attack) {
    program(attack.case(), "native", |program, _| {
        let plan =
            memory::verify(memory::derive(program, true).expect("actual memory claim"), program)
                .expect("genuine memory authority");
        let m = model::Model::new(program, &plan).expect("genuine type/source views");
        let mut code = code::build(&m).expect("actual receipt code");
        let pristine = artifact::encode(&m, &code).expect("actual pristine module");
        artifact::independent_audit(&pristine, &m).expect("pristine owner before hostile");
        artifact::audit(&pristine, &m, &code).expect("pristine full owner before hostile");
        let f = m.program.export_functions()[0];
        let function = m.source_base as usize + f;
        let body = Parser::new(0)
            .parse_all(&pristine)
            .filter_map(|p| match p.expect("actual sections") {
                Payload::CodeSectionEntry(body) => Some(body),
                _ => None,
            })
            .nth(function)
            .expect("actual source body");
        let origin = usize::try_from(body.range().start).expect("bounded body offset");
        let mut reader = body.get_operators_reader().expect("actual operators");
        let mut ops = Vec::new();
        while !reader.eof() {
            let start =
                usize::try_from(reader.original_position()).expect("bounded operator position");
            let op = reader.read().expect("valid actual opcode");
            let end =
                usize::try_from(reader.original_position()).expect("bounded operator position");
            ops.push((op, start - origin..end - origin));
        }
        attack.change(&ops, &m, f, &mut code[function].body.bytes);
        let wrong = artifact::encode(&m, &code).expect("matching changed supplemental body");
        assert_ne!(wrong, pristine);
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(&wrong)
            .expect("candidate stays valid Wasm");
        assert!(artifact::independent_audit(&wrong, &m).is_err(), "independent {attack:?}");
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "supplemental body cannot issue {attack:?}"
        );
    });
}
