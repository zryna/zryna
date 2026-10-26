//! Genuine new seals: pristine full audits precede valid hostile code and memory candidates.
use super::{artifact, code, memory, model};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
mod attacks;
#[path = "../../../tests/generic_execution_v1_push_upgrade/source.rs"]
mod source;
use attacks::Attack;
#[test]
fn genuine_push_upgrade_bytes_pass_independent_and_final_artifact_owners() {
    for case in source::SETTLED {
        for provider in source::PROVIDERS {
            for observed in [false, true] {
                source::with_program(case, provider, |program, _| {
                    let plan = memory::verify(
                        memory::derive(program, observed).expect("actual claim"),
                        program,
                    )
                    .expect("same-program memory authority");
                    let model =
                        model::Model::new(program, &plan).expect("retained new-domain views");
                    let code = code::build(&model).expect("new operation lowering");
                    let bytes = artifact::encode(&model, &code).expect("actual code and sections");
                    artifact::independent_audit(&bytes, &model)
                        .expect("pristine independent obligations");
                    artifact::audit(&bytes, &model, &code).expect("pristine full artifact owner");
                });
            }
        }
    }
}
#[test]
fn pending_descriptor_type_site_and_noalias_are_independently_rederived() {
    assert!(source::SETTLED.contains(&"vec-push"));
    source::with_program("vec-push", "native", |program, _| {
        for field in 0..3 {
            let mut claim = memory::derive(program, false).expect("genuine claim");
            let frame = claim
                .functions
                .iter_mut()
                .find(|frame| !frame.pending_vecs.is_empty())
                .expect("actual append temporary inventory");
            match field {
                0 => frame.pending_vecs[0].slot.offset = frame.values[0].offset,
                1 => {
                    frame.pending_vecs[0].slot.ty =
                        zryna_ir::generic_v1::execution_v1::raw::ValueType::Stored(1);
                }
                _ => frame.pending_vecs[0].site.instruction += 1,
            }
            assert!(
                memory::verify(claim, program).is_err(),
                "untrusted pending claims cannot replace the actual source role"
            );
        }
        memory::verify(memory::derive(program, false).expect("pristine retry"), program)
            .expect("no failed verification state escapes");
    });
}
#[test]
fn changed_push_upgrade_bodies_cannot_use_matching_supplemental_copies_as_authority() {
    for attack in [
        Attack::PendingZero,
        Attack::GrowthFactor,
        Attack::CopySource,
        Attack::PendingCapacity,
        Attack::ExpiredResult,
        Attack::StrongIncrement,
        Attack::LiveGuard,
        Attack::SuccessEdge,
    ] {
        reject(attack);
    }
}
fn reject(attack: Attack) {
    let case = attack.case();
    assert!(source::SETTLED.contains(&case));
    source::with_program(case, "native", |program, _| {
        let plan = memory::verify(memory::derive(program, true).expect("genuine claim"), program)
            .expect("genuine memory");
        let model = model::Model::new(program, &plan).expect("same new seal");
        let mut code = code::build(&model).expect("pristine recipes");
        let pristine = artifact::encode(&model, &code).expect("actual artifact");
        artifact::independent_audit(&pristine, &model).expect("pristine independent gate");
        artifact::audit(&pristine, &model, &code).expect("pristine complete artifact gate");
        let function = attack.function(&model);
        let body = Parser::new(0)
            .parse_all(&pristine)
            .filter_map(|payload| match payload.expect("valid actual sections") {
                Payload::CodeSectionEntry(body) => Some(body),
                _ => None,
            })
            .nth(function)
            .expect("actual independently assigned function role");
        let origin = body.range().start;
        let mut reader = body.get_operators_reader().expect("actual operators");
        let mut instructions = Vec::new();
        while !reader.eof() {
            let start = reader.original_position();
            let operation = reader.read().expect("valid operator");
            let start = usize::try_from(start - origin).expect("bounded actual body start");
            let end = usize::try_from(reader.original_position() - origin)
                .expect("bounded actual body end");
            instructions.push((operation, start..end));
        }
        let (range, replacement) = attack.change(&instructions, &model);
        let mut changed = Vec::new();
        replacement.encode(&mut changed);
        code[function].body.bytes.splice(range, changed);
        let wrong =
            artifact::encode(&model, &code).expect("actual changed bytes and supplemental copy");
        assert_ne!(wrong, pristine);
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(&wrong)
            .expect("hostile candidate remains valid Wasm");
        assert!(
            artifact::independent_audit(&wrong, &model).is_err(),
            "independent {attack:?} obligation"
        );
        assert!(
            artifact::audit(&wrong, &model, &code).is_err(),
            "supplemental bytes cannot admit {attack:?}"
        );
    });
}
