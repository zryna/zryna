//! The public scalar result survives cleanup runtime scratch writes without changing order.
use super::container_source as source;
use super::{artifact, code, memory, model};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};

#[test]
fn shared_scalar_return_survives_last_strong_cleanup_faults_and_pristine_reentry() {
    assert!(source::SETTLED.contains(&"shared-weak-count"));
    let expected = source::row("shared-weak-count");
    assert_eq!(
        expected["expected"]["result"], 7,
        "the unchanged source oracle remains authoritative"
    );
    for provider in source::PROVIDERS {
        for observed in [false, true] {
            source::with_program("shared-weak-count", provider, |program| {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("same-seal memory claim"),
                    program,
                )
                .expect("independent fixed memory geometry");
                assert_eq!(plan.claims().scratch_bytes, 32);
                let result_start = plan
                    .claims()
                    .scratch_start
                    .checked_add(16)
                    .expect("small checked scratch offset");
                assert!(
                    result_start + 4 <= plan.claims().scratch_start + plan.claims().scratch_bytes
                );
                let model = model::Model::new(program, &plan).expect("same-seal model");
                let code = code::build(&model).expect("new lowered source and cleanup");
                let bytes = artifact::encode(&model, &code).expect("actual final bytes");
                artifact::independent_audit(&bytes, &model)
                    .expect("exact wrapper address obligations");
                artifact::audit(&bytes, &model, &code).expect("full final artifact authority");
                let engine = wasmtime::Engine::default();
                let module = wasmtime::Module::new(&engine, &bytes).expect("actual valid Wasm");
                assert_eq!(module.imports().len(), 0);
                let mut store = wasmtime::Store::new(&engine, ());
                let instance = wasmtime::Instance::new(&mut store, &module, &[])
                    .expect("private defined memory");
                let entry = instance
                    .get_typed_func::<(), i32>(&mut store, "entry")
                    .expect("literal scalar source entry");
                for _ in 0..3 {
                    assert_eq!(entry.call(&mut store, ()).expect("cleanup then scalar return"), 7);
                }
                if observed {
                    let observe = instance
                        .get_typed_func::<i32, i32>(&mut store, "$zryna$observation")
                        .expect("distinct observed artifact");
                    for ordinal in 1..=2 {
                        let configuration: i32 = 0x2200_0000 | ordinal;
                        assert_eq!(
                            observe.call(&mut store, configuration).expect("fault config"),
                            0
                        );
                        assert!(
                            entry.call(&mut store, ()).is_err(),
                            "failure must not publish old result"
                        );
                        assert_eq!(observe.call(&mut store, 0).expect("allocation category"), 2);
                        assert_eq!(
                            observe
                                .call(&mut store, 0x2000_0001)
                                .expect("reset requires all real owners released"),
                            0
                        );
                        assert_eq!(entry.call(&mut store, ()).expect("pristine retry"), 7);
                    }
                }
            });
        }
    }
}

#[test]
fn wrapper_output_and_read_cannot_alias_runtime_scratch_even_with_updated_body_bytes() {
    assert!(source::SETTLED.contains(&"shared-weak-count"));
    for observed in [false, true] {
        source::with_program("shared-weak-count", "native", |program| {
            let plan = memory::verify(memory::derive(program, observed).expect("claims"), program)
                .expect("independent memory");
            let model = model::Model::new(program, &plan).expect("same-seal model");
            for occurrence in 0..2 {
                let mut code = code::build(&model).expect("pristine recipe");
                let pristine = artifact::encode(&model, &code).expect("actual pristine bytes");
                let wrapper = usize::try_from(model.wrapper_base).expect("bounded wrapper index");
                let body = Parser::new(0)
                    .parse_all(&pristine)
                    .filter_map(|payload| match payload.expect("valid final sections") {
                        Payload::CodeSectionEntry(body) => Some(body),
                        _ => None,
                    })
                    .nth(wrapper)
                    .expect("actual wrapper body");
                let origin = body.range().start;
                let mut reader = body.get_operators_reader().expect("actual wrapper operators");
                let result =
                    i32::try_from(plan.claims().scratch_start + 16).expect("bounded result lane");
                let mut selected = None;
                let mut seen = 0;
                while !reader.eof() {
                    let start = reader.original_position();
                    if matches!(reader.read().expect("valid operator"), O::I32Const { value } if value == result)
                    {
                        if seen == occurrence {
                            selected = Some(
                                usize::try_from(start - origin).expect("bounded start")
                                    ..usize::try_from(reader.original_position() - origin)
                                        .expect("bounded end"),
                            );
                        }
                        seen += 1;
                    }
                }
                assert_eq!(seen, 2, "exact outgoing result pointer and final scalar read address");
                let mut replacement = Vec::new();
                I::I32Const(i32::try_from(plan.claims().scratch_start).expect("runtime scratch"))
                    .encode(&mut replacement);
                code[wrapper]
                    .body
                    .bytes
                    .splice(selected.expect("actual chosen lane address"), replacement);
                let wrong = artifact::encode(&model, &code)
                    .expect("changed actual bytes and supplemental body");
                assert_ne!(wrong, pristine);
                Validator::new_with_features(WasmFeatures::WASM1)
                    .validate_all(&wrong)
                    .expect("aliasing candidate remains structurally valid Wasm");
                assert!(
                    artifact::independent_audit(&wrong, &model).is_err(),
                    "source-derived wrapper obligations reject runtime-scratch aliasing"
                );
                assert!(
                    artifact::audit(&wrong, &model, &code).is_err(),
                    "supplemental matching bytes cannot grant the wrong address"
                );
            }
        });
    }
}
