//! Genuine source-bound hostile actual-artifact controls; no manufactured authority.
use super::*;
use wasm_encoder::{Module, RawSection};
use wasmparser::{Parser, Payload};
#[path = "../../tests/generic_execution_v1_containers/source.rs"]
pub(super) mod container_source;
#[path = "../../tests/generic_execution_v1/source.rs"]
pub(super) mod source;
fn replacement(bytes: &[u8], id: u8, change: impl FnOnce(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let mut replacement = Some(change);
    let mut module = Module::new();
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.expect("fixed genuine source/target control");
        if let Some((section, range)) = payload.as_section() {
            let old = &bytes[usize::try_from(range.start)
                .expect("fixed genuine source/target control")
                ..usize::try_from(range.end).expect("fixed genuine source/target control")];
            if section == id {
                module.section(&RawSection {
                    id: section,
                    data: &replacement.take().expect("fixed genuine source/target control")(old),
                });
            } else {
                module.section(&RawSection { id: section, data: old });
            }
        }
    }
    module.finish()
}
#[test]
fn defined_memory_globals_and_actual_zero_partition_are_audited() {
    source::program("nominal-enum-clone", "native", |program| {
        let plan = memory::verify(
            memory::derive(program, false).expect("fixed genuine source/target control"),
            program,
        )
        .expect("fixed genuine source/target control");
        let m = model::Model::new(program, &plan).expect("fixed genuine source/target control");
        let code = code::build(&m).expect("fixed genuine source/target control");
        let bytes = artifact::encode(&m, &code).expect("fixed genuine source/target control");
        artifact::audit(&bytes, &m, &code).expect("fixed genuine source/target control");
        let wrong = replacement(&bytes, 5, |_| vec![1, 1, 0x80, 8, 0x80, 8]);
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "1024page imported-route ceiling cannot enter new defined-memory policy"
        );
        let wrong = replacement(&bytes, 6, |old| {
            let mut data = old.to_vec();
            assert_eq!(data[3], 0x41);
            data[4] ^= 8;
            data
        });
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "allocator base must follow actual checked intervals"
        );
        let wrong = replacement(&bytes, 11, |old| {
            let mut data = old.to_vec();
            let last = data.len() - 1;
            data[last] = 1;
            data
        });
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "actual arena data is zero, not assumed free"
        );
        let literal = &plan.claims().literals[0];
        let wrong = replacement(&bytes, 11, |old| {
            let mut data = old.to_vec();
            let start = data.len() - plan.claims().heap_start as usize;
            data[start + literal.offset as usize] ^= 1;
            data
        });
        assert!(
            artifact::audit(&wrong, &m, &code).is_err(),
            "literal bytes stay bound to source operation"
        );
        artifact::audit(&bytes, &m, &code).expect("fixed genuine source/target control");
    });
}
#[test]
fn helper_body_and_observer_export_are_bound_to_explicit_policy() {
    source::program("nominal-enum-clone", "native", |program| {
        let plan = memory::verify(
            memory::derive(program, true).expect("fixed genuine source/target control"),
            program,
        )
        .expect("fixed genuine source/target control");
        let m = model::Model::new(program, &plan).expect("fixed genuine source/target control");
        let code = code::build(&m).expect("fixed genuine source/target control");
        let bytes = artifact::encode(&m, &code).expect("fixed genuine source/target control");
        let mut wrong = bytes.clone();
        let body = Parser::new(0)
            .parse_all(&bytes)
            .filter_map(|p| match p.expect("fixed genuine source/target control") {
                Payload::CodeSectionEntry(body) => Some(body),
                _ => None,
            })
            .nth(m.observer as usize)
            .expect("fixed genuine source/target control");
        let mut operators =
            body.get_operators_reader().expect("fixed genuine source/target control");
        let position = loop {
            let at = operators.original_position();
            if matches!(
                operators.read().expect("fixed genuine source/target control"),
                wasmparser::Operator::GlobalGet { global_index: 1 }
            ) {
                break at + 1;
            }
        };
        wrong[usize::try_from(position).expect("fixed genuine source/target control")] = 0;
        assert!(
            wasmparser::Validator::new().validate_all(&wrong).is_ok(),
            "valid changed helper body"
        );
        assert!(artifact::audit(&wrong, &m, &code).is_err());
        let ordinary = memory::verify(
            memory::derive(program, false).expect("fixed genuine source/target control"),
            program,
        )
        .expect("fixed genuine source/target control");
        let ordinary_model =
            model::Model::new(program, &ordinary).expect("fixed genuine source/target control");
        let ordinary_code =
            code::build(&ordinary_model).expect("fixed genuine source/target control");
        assert!(
            artifact::audit(&bytes, &ordinary_model, &ordinary_code).is_err(),
            "observer artifact cannot satisfy ordinary policy"
        );
        let wrong = replacement(&bytes, 7, |old| {
            let mut data = old.to_vec();
            let at = data
                .windows(18)
                .position(|s| s == b"$zryna$observation")
                .expect("fixed genuine source/target control");
            data[at] = b'x';
            data
        });
        assert!(artifact::audit(&wrong, &m, &code).is_err());
        artifact::audit(&bytes, &m, &code).expect("fixed genuine source/target control");
    });
}

mod independent;

mod container_control;

mod wrapper_output_partition;
