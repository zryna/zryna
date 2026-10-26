//! Actual source frames exercise large offsets without broadening the memory-op grammar.
use super::super::tests::container_source as source;
use super::super::{artifact, code, memory, model};
use std::ops::Range;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};

struct Instruction<'a> {
    operation: O<'a>,
    range: Range<usize>,
}

#[test]
fn genuine_large_frame_loads_keep_the_closed_memory_grammar_and_exact_source_address() {
    assert!(source::SETTLED.contains(&"vec-copy"));
    for provider in source::PROVIDERS {
        for observed in [false, true] {
            source::with_program("vec-copy", provider, |program| {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine same-seal memory claim"),
                    program,
                )
                .expect("actual original frame geometry");
                let model = model::Model::new(program, &plan).expect("same-program model");
                let mut code = code::build(&model).expect("canonical raw code");
                let pristine = artifact::encode(&model, &code).expect("actual assembled bytes");
                artifact::independent_audit(&pristine, &model).expect("independent source events");
                artifact::audit(&pristine, &model, &code).expect("complete final artifact audit");
                let (function, range, value) = large_frame_load(&pristine, &model);
                let mut replacement = Vec::new();
                I::I32Const(value.checked_add(4).expect("small checked source frame offset"))
                    .encode(&mut replacement);
                // Deliberately update the supplemental exact body as well as final bytes.
                code[function].body.bytes.splice(range, replacement);
                let wrong = artifact::encode(&model, &code).expect("changed actual bytes");
                assert_ne!(wrong, pristine, "the hostile address must actually change");
                Validator::new_with_features(WasmFeatures::WASM1)
                    .validate_all(&wrong)
                    .expect("changed frame address remains structurally valid Wasm");
                assert!(
                    artifact::independent_audit(&wrong, &model).is_err(),
                    "the decoded address must retain its exact original source slot"
                );
                assert!(
                    artifact::audit(&wrong, &model, &code).is_err(),
                    "a changed supplemental body cannot launder the changed address"
                );
                let pristine_code = code::build(&model).expect("pristine recipe retry");
                artifact::audit(&pristine, &model, &pristine_code)
                    .expect("pristine next full audit");
            });
        }
    }
}

fn large_frame_load(bytes: &[u8], model: &model::Model<'_, '_, '_>) -> (usize, Range<usize>, i32) {
    let mut selected = None;
    let mut function = 0usize;
    for payload in Parser::new(0).parse_all(bytes) {
        let Payload::CodeSectionEntry(body) = payload.expect("actual bounded Wasm module") else {
            continue;
        };
        let origin = body.range().start;
        let mut reader = body.get_operators_reader().expect("actual function operators");
        let mut instructions = Vec::new();
        while !reader.eof() {
            let start = reader.original_position();
            let operation = reader.read().expect("valid actual operator");
            if let O::I32Load { memarg } = operation {
                assert_eq!(memarg.memory, 0);
                assert_eq!(memarg.align, 2);
                assert!(memarg.offset <= 16, "the existing closed audit bound is retained");
            }
            instructions.push(Instruction {
                operation,
                range: usize::try_from(start - origin).expect("bounded start")
                    ..usize::try_from(reader.original_position() - origin).expect("bounded end"),
            });
        }
        if function >= usize::try_from(model.source_base).expect("bounded source index")
            && function < usize::try_from(model.wrapper_base).expect("bounded wrapper index")
        {
            for window in instructions.windows(4) {
                if matches!(window[0].operation, O::LocalGet { local_index: 0 })
                    && matches!(window[2].operation, O::I32Add)
                    && matches!(window[3].operation, O::I32Load { memarg } if memarg.offset == 0)
                    && let O::I32Const { value } = window[1].operation
                    && value > 16
                {
                    selected.get_or_insert((function, window[1].range.clone(), value));
                }
            }
        }
        function += 1;
    }
    selected.expect("genuine Vec source frame contains a normalized load beyond offset sixteen")
}
