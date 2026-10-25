//! Pristine genuine artifacts precede changed valid bytes; supplemental copies are never authority.
use super::{artifact, code, concat_source, loan_source, memory, model};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
mod attacks;
use attacks::Attack;
fn program(
    case: &str,
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    if ["local-observation", "same-source", "nested-temporaries", "formal-observation"]
        .contains(&case)
    {
        concat_source::with_program(case, "native", check);
    } else {
        loan_source::with_program(case, "native", check);
    }
}
#[test]
fn genuine_loan_concat_pristine_artifacts_pass_both_actual_byte_owners() {
    let source_diagnostics = loan_source::diagnostic_expected();
    assert!(
        source_diagnostics["cases"]
            .as_array()
            .expect("exact frozen diagnostics")
            .iter()
            .any(|r| r["case"] == "opaque-clone")
    );
    for case in [
        "formal-copy",
        "opaque-forward",
        "shared-reborrow-clone",
        "exclusive-reborrow-push",
        "generic-replace",
        "static-disjoint",
        "branch-cleanup",
        "fallible-cleanup",
        "referent-catalogue",
        "local-observation",
        "same-source",
        "nested-temporaries",
        "formal-observation",
    ] {
        program(case, |program, _| {
            for observed in [false, true] {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine memory claim"),
                    program,
                )
                .expect("same-seal memory");
                let model = model::Model::new(program, &plan).expect("exact retained views");
                let code = code::build(&model).expect("actual loan/concat code");
                let bytes = artifact::encode(&model, &code).expect("actual sections and bodies");
                artifact::independent_audit(&bytes, &model)
                    .expect("pristine independent byte proof");
                artifact::audit(&bytes, &model, &code)
                    .expect("pristine complete final artifact proof");
            }
        });
    }
}
#[test]
fn raw_loan_cell_type_access_and_alias_mutants_fail_independent_memory_rederivation() {
    program("generic-replace", |program, _| {
        for field in 0..3 {
            let mut claim = memory::derive(program, false).expect("actual raw claim");
            let frame = claim
                .functions
                .iter_mut()
                .find(|f| !f.loans.is_empty())
                .expect("actual loan frame");
            match field {
                0 => frame.loans[0].offset = frame.values[0].offset,
                1 => {
                    frame.loans[0].referent = program
                        .layouts(zryna_layout::StorageTarget::Linear32V1)
                        .types()
                        .find(|t| t.category() == zryna_layout::TypeCategory::I32)
                        .expect("actual i32 control")
                        .id()
                        .index();
                }
                _ => {
                    frame.loans[0].access = zryna_ir::generic_v1::execution_v1::raw::Access::Shared;
                }
            }
            assert!(
                memory::verify(claim, program).is_err(),
                "raw rows do not grant aliases or permissions"
            );
        }
        memory::verify(memory::derive(program, false).expect("pristine retry"), program)
            .expect("failed raw claims publish no state");
    });
}
#[test]
fn actual_loan_concat_hostile_bodies_reject_even_with_matching_supplemental_copies() {
    for attack in [
        Attack::LoanLength,
        Attack::LoanCell,
        Attack::LoanEnd,
        Attack::FormalAddress,
        Attack::DropOmitted,
        Attack::EarlyCommit,
        Attack::FirstCopyLength,
        Attack::SecondCopySource,
        Attack::SecondDestination,
        Attack::CombinedCapacity,
        Attack::OutputLength,
    ] {
        reject(attack);
    }
}
fn reject(attack: Attack) {
    program(attack.case(), |program, _| {
        let plan = memory::verify(memory::derive(program, true).expect("actual memory"), program)
            .expect("genuine memory authority");
        let model = model::Model::new(program, &plan).expect("genuine source views");
        let mut code = code::build(&model).expect("pristine code");
        let pristine = artifact::encode(&model, &code).expect("pristine actual module");
        artifact::independent_audit(&pristine, &model)
            .expect("pristine independent owner before mutant");
        artifact::audit(&pristine, &model, &code).expect("pristine final owner before mutant");
        let function = attack.function(&model);
        let body = Parser::new(0)
            .parse_all(&pristine)
            .filter_map(|p| match p.expect("valid sections") {
                Payload::CodeSectionEntry(body) => Some(body),
                _ => None,
            })
            .nth(function)
            .expect("actual helper/source role");
        let origin = usize::try_from(body.range().start).expect("bounded body offset");
        let mut reader = body.get_operators_reader().expect("actual decoded operators");
        let mut operators = Vec::new();
        while !reader.eof() {
            let start =
                usize::try_from(reader.original_position()).expect("bounded operator offset");
            let operation = reader.read().expect("valid operator");
            let end = usize::try_from(reader.original_position()).expect("bounded operator offset");
            operators.push((operation, start - origin..end - origin));
        }
        attack.change(&operators, &model, &mut code[function].body.bytes);
        let wrong = artifact::encode(&model, &code).expect("matching changed supplemental body");
        assert_ne!(wrong, pristine);
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(&wrong)
            .expect("candidate remains valid Wasm");
        assert!(
            artifact::independent_audit(&wrong, &model).is_err(),
            "independent {attack:?} obligation"
        );
        assert!(
            artifact::audit(&wrong, &model, &code).is_err(),
            "supplemental body cannot grant {attack:?}"
        );
    });
}
