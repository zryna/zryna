//! Genuine container owner controls mutate valid bytes and the supplemental body copy together.
use super::{artifact, code, memory, model};
use std::ops::Range;
use wasm_encoder::{CodeSection, Encode, Instruction as I, Module, RawSection, Section};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
mod attacks;
use super::container_source as source;
use attacks::Attack;
#[test]
fn pristine_container_bytes_pass_both_independent_and_final_artifact_audits() {
    for case in ["vec-copy", "vec-owned-prepare", "shared-weak-count", "opaque-count-unused"] {
        for observed in [false, true] {
            source::with_program(case, "native", |program| {
                let memory =
                    memory::verify(memory::derive(program, observed).expect("claims"), program)
                        .expect("memory");
                let model = model::Model::new(program, &memory).expect("model");
                let code = code::build(&model).expect("raw code");
                let bytes = artifact::encode(&model, &code).expect("raw bytes");
                artifact::independent_audit(&bytes, &model).expect("independent semantic audit");
                artifact::audit(&bytes, &model, &code).expect("final section and exact-body audit");
            });
        }
    }
}
struct Inst<'a> {
    op: O<'a>,
    range: Range<usize>,
}
fn replace(body: &mut Vec<u8>, at: &Range<usize>, op: &I<'static>) {
    let mut encoded = Vec::new();
    op.encode(&mut encoded);
    body.splice(at.clone(), encoded);
}
fn mutate(
    bytes: &[u8],
    code: &mut [code::FunctionCode],
    function: usize,
    change: impl FnOnce(&mut Vec<u8>, &[Inst<'_>]),
) -> Vec<u8> {
    let target = Parser::new(0)
        .parse_all(bytes)
        .filter_map(|p| match p.expect("fixed module") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .nth(function)
        .expect("exact retained function role");
    let origin = target.range().start;
    let mut reader = target.get_operators_reader().expect("fixed operators");
    let mut instructions = Vec::new();
    while !reader.eof() {
        let start = reader.original_position();
        let op = reader.read().expect("fixed instruction");
        instructions.push(Inst {
            op,
            range: usize::try_from(start - origin).expect("bounded start")
                ..usize::try_from(reader.original_position() - origin).expect("bounded end"),
        });
    }
    let mut body = target.as_bytes().to_vec();
    change(&mut body, &instructions);
    code[function].body.bytes = body.clone(); // Supplemental exact-body binding is deliberately updated too.
    let mut section = CodeSection::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CodeSectionEntry(old) = payload.expect("fixed code") {
            section.raw(if old.range() == target.range() { &body } else { old.as_bytes() });
        }
    }
    let mut module = Module::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Some((id, range)) = payload.expect("fixed sections").as_section() {
            if id == section.id() {
                module.section(&section);
            } else {
                module.section(&RawSection {
                    id,
                    data: &bytes[usize::try_from(range.start).expect("bounded start")
                        ..usize::try_from(range.end).expect("bounded end")],
                });
            }
        }
    }
    module.finish()
}

#[test]
fn owned_vec_cursor_and_stride_are_bound_independently_of_body_matching() {
    for attack in [Attack::Cursor, Attack::Stride] {
        reject("vec-owned-prepare", attack);
    }
}
#[test]
fn count_terminal_last_strong_and_packed_allocation_roles_are_independent() {
    for attack in [Attack::CountTarget, Attack::LastStrong, Attack::PackedStatus] {
        reject("shared-weak-count", attack);
    }
}
fn reject(case: &str, attack: Attack) {
    assert!(source::SETTLED.contains(&case));
    source::with_program(case, "native", |program| {
        let plan = memory::verify(memory::derive(program, true).expect("genuine claims"), program)
            .expect("independent memory");
        let model = model::Model::new(program, &plan).expect("genuine same-seal model");
        let mut code = code::build(&model).expect("private raw code");
        let bytes = artifact::encode(&model, &code).expect("private raw assembly");
        artifact::independent_audit(&bytes, &model).expect("pristine independent owner");
        let function = attack.function(&model);
        let wrong = mutate(&bytes, &mut code, function, |body, ops| attack.change(body, ops));
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(&wrong)
            .expect("actual hostile candidate remains valid Wasm");
        assert!(
            artifact::independent_audit(&wrong, &model).is_err(),
            "independent container audit rejects {attack:?}"
        );
        assert!(
            artifact::audit(&wrong, &model, &code).is_err(),
            "updated supplemental body cannot admit {attack:?}"
        );
        artifact::independent_audit(&bytes, &model).expect("pristine next audit");
    });
}

#[test]
fn last_strong_output_cannot_escape_its_actual_zero_strong_guard() {
    reject("shared-weak-count", Attack::LastStrongGuardEscape);
}
#[test]
fn actual_live_owner_decrement_cannot_be_replaced_by_zero() {
    reject("vec-copy", Attack::LiveCountDecrement);
}

#[test]
fn canonical_failure_return_stays_inside_its_exact_validation_guard() {
    reject("shared-weak-count", Attack::ValidationBoundary);
}
#[test]
fn runtime_failure_cannot_return_success_while_setting_failure_status() {
    reject("shared-weak-count", Attack::ReturnStatus);
}

#[test]
fn control_size_and_pending_form_one_exact_predicate() {
    reject("shared-weak-count", Attack::ControlPredicate);
}

#[test]
fn packed_allocation_result_cannot_be_modified_before_its_record_store() {
    reject("vec-owned-prepare", Attack::PackedCallResult);
}
