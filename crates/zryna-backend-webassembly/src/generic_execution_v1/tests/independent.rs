//! Genuine same-seal hostile controls update both actual bytes and supplemental body copies.
use super::{artifact, code, memory, model, source};
use std::ops::Range;
use wasm_encoder::{CodeSection, Encode, Instruction as I, Module, RawSection, Section};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
mod attacks;
use attacks::Attack;
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
fn reject(case: &str, observed: bool, attack: Attack) {
    source::program(case, "native", |program| {
        let plan = memory::verify(
            memory::derive(program, observed).expect("genuine memory claim"),
            program,
        )
        .expect("independent memory authority");
        let m = model::Model::new(program, &plan).expect("genuine model");
        let mut code = code::build(&m).expect("fixed encoder");
        let bytes = artifact::encode(&m, &code).expect("fixed module");
        artifact::independent_audit(&bytes, &m).expect("pristine independent audit");
        let function = attack.function(&m);
        let wrong = mutate(&bytes, &mut code, function, |body, ops| attack.change(body, ops, &m));
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(&wrong)
            .expect("attack remains valid WebAssembly 1.0");
        assert!(
            artifact::independent_audit(&wrong, &m).is_err(),
            "independent role audit rejects {attack:?}"
        );
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "updated supplementary oracle cannot admit {attack:?}"
        );
        artifact::independent_audit(&bytes, &m).expect("pristine next audit");
    });
}
#[test]
fn independently_bound_source_calls_outputs_status_regions_cfg_and_index_roles() {
    for attack in [
        Attack::RuntimeTarget,
        Attack::OutputOffset,
        Attack::InvertedStatus,
        Attack::UnconditionalCleanup,
        Attack::EdgeTarget,
    ] {
        reject("nominal-enum-clone", false, attack);
    }
    reject("fixed-array-identity", false, Attack::IndexSource);
    reject("scalar-bool-clone", false, Attack::BranchCondition);
}
#[test]
fn independently_bound_copy_array_heap_and_wrapper_progress() {
    for attack in [Attack::CopyStall, Attack::HeapBound, Attack::BusyReset] {
        reject("nominal-enum-clone", false, attack);
    }
    reject("fixed-array-identity", false, Attack::ArrayBound);
}
#[test]
fn observed_fault_status_and_trace_bounds_are_independent_of_body_byte_matching() {
    for attack in [Attack::TraceBound, Attack::CloneFaultKind, Attack::ObserverStatus] {
        reject("nominal-enum-clone", true, attack);
    }
}

#[test]
fn unclassified_guard_cannot_skip_the_actual_copy_kernel() {
    reject("nominal-enum-clone", false, Attack::UnclassifiedGuard);
}
#[test]
fn early_trap_cannot_skip_the_actual_copy_kernel() {
    reject("nominal-enum-clone", false, Attack::EarlyTrap);
}
