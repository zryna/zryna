//! New same-seal owned-memory target checkpoint; first6 oracles remain unchanged.
use zryna_backend_webassembly::generic_execution_v1::{emit, emit_observed, memory};
#[path = "generic_execution_v1/source.rs"]
mod source;
const CASES: [(&str, i32); 5] = [
    ("scalar-i32-clone", 14),
    ("scalar-bool-clone", 7),
    ("nominal-enum-clone", 7),
    ("option-nominal-enum-clone", 7),
    ("fixed-array-identity", 18),
];
#[test]
fn real_first_five_get_independently_checked_owned_memory_and_exact_scalar_results() {
    for (case, expected) in CASES {
        source::program(case, "native", |program| {
            let claim = memory::derive(program, false).expect("new target-memory claims");
            let plan = memory::verify(claim, program)
                .expect("independent sameprogram/layout/runtime memory authority");
            assert_eq!(plan.claims().memory_pages, 256);
            assert!(plan.claims().frames_start >= plan.claims().null_end);
            assert!(
                plan.claims().heap_start >= plan.claims().frames_start + plan.claims().frames_bytes
            );
            let artifact = emit(program).expect("new separately audited ordinary artifact");
            let engine = wasmtime::Engine::default();
            let module = wasmtime::Module::new(&engine, artifact.bytes())
                .expect("fixed genuine source/target control");
            assert_eq!(module.imports().len(), 0);
            assert_eq!(
                module.exports().map(|export| export.name()).collect::<Vec<_>>(),
                vec!["entry"]
            );
            let mut store = wasmtime::Store::new(&engine, ());
            let instance = wasmtime::Instance::new(&mut store, &module, &[])
                .expect("fixed genuine source/target control");
            let entry = instance
                .get_typed_func::<(), i32>(&mut store, "entry")
                .expect("fixed genuine source/target control");
            assert_eq!(
                entry.call(&mut store, ()).expect("fixed genuine source/target control"),
                expected
            );
            assert_eq!(
                entry.call(&mut store, ()).expect("fixed genuine source/target control"),
                expected,
                "pristine reentry"
            );
        });
    }
}
#[test]
fn overlapping_static_frames_heap_and_foreign_program_claims_cannot_make_a_memory_token() {
    source::program("nominal-enum-clone", "native", |program| {
        let original = memory::derive(program, true).expect("fixed genuine source/target control");
        assert_eq!(original.trace_bytes, 16384);
        let mut wrong = original.clone();
        wrong.frames_start = 0;
        assert!(memory::verify(wrong, program).is_err());
        let mut wrong = original.clone();
        wrong.heap_start = original.frames_start;
        assert!(memory::verify(wrong, program).is_err());
        let mut wrong = original.clone();
        wrong.memory_pages = 1024;
        assert!(memory::verify(wrong, program).is_err());
        let mut wrong = original.clone();
        wrong.scratch_start = original.frames_start;
        assert!(memory::verify(wrong, program).is_err());
        let mut wrong = original.clone();
        wrong.frames_bytes = u32::MAX;
        assert!(memory::verify(wrong, program).is_err());
        source::program("nominal-enum-clone", "typescript", |other| {
            assert!(
                memory::verify(original.clone(), other).is_err(),
                "same bytes do not replace the retained program"
            );
        });
        memory::verify(original, program).expect("pristine retry");
    });
}
#[test]
fn observed_artifact_is_separate_and_has_only_the_reserved_extra_observer() {
    source::program("nominal-enum-clone", "native", |program| {
        let artifact = emit_observed(program).expect("explicit test-only policy");
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, artifact.as_bytes())
            .expect("fixed genuine source/target control");
        assert_eq!(module.imports().len(), 0);
        assert_eq!(
            module.exports().map(|export| export.name()).collect::<Vec<_>>(),
            vec!["entry", "$zryna$observation"]
        );
    });
}
