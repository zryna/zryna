//! Genuine recursive source mutations cannot borrow the emitter's supplemental body authority.
use super::source;
use crate::generic_execution_v1::{artifact, code, memory, model, recursive};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};

fn valid(bytes: &[u8]) {
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(bytes)
        .expect("hostile candidate remains valid core Wasm");
}

fn instruction(
    bytes: &[u8],
    code: &mut [code::FunctionCode],
    function: usize,
    find: impl Fn(&O<'_>) -> bool,
    replacement: &I<'static>,
) -> Vec<u8> {
    let body = Parser::new(0)
        .parse_all(bytes)
        .filter_map(|p| match p.expect("actual module") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .nth(function)
        .expect("exact private function role");
    let origin = usize::try_from(body.range().start).expect("actual body offset");
    let mut reader = body.get_operators_reader().expect("actual operators");
    while !reader.eof() {
        let start = usize::try_from(reader.original_position()).expect("bounded offset");
        let op = reader.read().expect("actual instruction");
        if find(&op) {
            let end = usize::try_from(reader.original_position()).expect("bounded end");
            let mut encoded = Vec::new();
            replacement.encode(&mut encoded);
            assert_eq!(encoded.len(), end - start, "preserve all section framing");
            let mut changed = bytes.to_vec();
            changed[start..end].copy_from_slice(&encoded);
            // Deliberately update the supplemental encoder body as well.
            code[function].body.bytes[start - origin..end - origin].copy_from_slice(&encoded);
            valid(&changed);
            return changed;
        }
    }
    panic!("actual private operation role must exist");
}

#[test]
fn fixed_kernel_roles_are_checked_without_trusting_the_supplemental_body() {
    source::with_clone(|program| {
        for attack in 0..5 {
            let claim = memory::derive(program, false).expect("original recursive memory");
            let plan = memory::verify(claim, program).expect("independent source-bound memory");
            let m = model::Model::new(program, &plan).expect("original source model");
            let mut code = code::build(&m).expect("original private kernel");
            let bytes = artifact::encode(&m, &code).expect("actual final module");
            artifact::independent_audit(&bytes, &m).expect("pristine independent authority");
            let base = m.recursive_base() as usize;
            let used = m.recursive_used_global();
            let metadata = plan.recursive_arena().expect("actual arena").metadata_start;
            let strong = m.container(model::containers::STRONG_CLONE);
            let weak = m.container(model::containers::WEAK_CLONE);
            let granted = m.clones.iter().position(Option::is_some).expect("source Clone grant");
            let (function, replacement) = match attack {
                0 => (base + 11, I::I32Const(25)),
                1 => (base + 11, I::GlobalSet(0)),
                2 => (base + recursive::CLONE as usize, I::Call(weak)),
                3 => (base, I::I32Const((metadata ^ 1).cast_signed())),
                _ => (
                    5 + granted,
                    I::I32Const(
                        u32::try_from(granted ^ 1).expect("bounded source type").cast_signed(),
                    ),
                ),
            };
            let wrong = instruction(
                &bytes,
                &mut code,
                function,
                |op| match attack {
                    0 => matches!(op, O::I32Const { value: 24 }),
                    1 => matches!(op, O::GlobalSet { global_index } if *global_index == used),
                    2 => matches!(op, O::Call { function_index } if *function_index == strong),
                    3 => matches!(op, O::I32Const { value } if *value == metadata.cast_signed()),
                    _ => matches!(op, O::I32Const { .. }),
                },
                &replacement,
            );
            assert!(artifact::independent_audit(&wrong, &m).is_err(), "hostile role {attack}");
            assert!(artifact::audit(&wrong, &m, &code).is_err(), "supplemental body {attack}");
            artifact::independent_audit(&bytes, &m).expect("pristine retry after rejection");
        }
    });
}

#[test]
fn immutable_metadata_is_checked_against_original_layouts_and_clone_authority() {
    source::with_clone(|program| {
        let claim = memory::derive(program, false).expect("original recursive memory");
        let plan = memory::verify(claim, program).expect("source-bound memory");
        let m = model::Model::new(program, &plan).expect("original model");
        let code = code::build(&m).expect("actual kernel");
        let bytes = artifact::encode(&m, &code).expect("actual artifact");
        let data = Parser::new(0)
            .parse_all(&bytes)
            .find_map(|p| match p.expect("actual module") {
                Payload::DataSection(reader) => Some(
                    reader.into_iter().next().expect("actual data segment").expect("valid data"),
                ),
                _ => None,
            })
            .expect("single private data segment");
        let start = usize::try_from(data.range.end).expect("bounded data end") - data.data.len();
        let base = start
            + plan.recursive_arena().expect("genuine recursive arena").metadata_start as usize;
        let granted = m.clones.iter().position(Option::is_some).expect("real Clone authority");
        let enumeration = m
            .types
            .iter()
            .position(|t| t.category() == zryna_layout::TypeCategory::Enum)
            .expect("real source enum");
        let vector = m
            .types
            .iter()
            .position(|t| t.category() == zryna_layout::TypeCategory::Vec)
            .expect("real recursive Vec");
        for offset in
            [12, 16 + granted * 32 + 12, 16 + enumeration * 32 + 24, 16 + vector * 32 + 16]
        {
            let mut wrong = bytes.clone();
            wrong[base + offset] ^= 1;
            valid(&wrong);
            assert!(artifact::audit(&wrong, &m, &code).is_err(), "wrong immutable word {offset}");
            artifact::audit(&bytes, &m, &code).expect("pristine metadata retry");
        }
    });
}
