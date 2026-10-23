use super::{original_types, project, raw, seal, with_claim};

const VALUES: &str =
    include_str!("../../../../../../../tests/m7-generic-nominal-copy-transport/values.zry");
const MAIN: &str =
    include_str!("../../../../../../../tests/m7-generic-nominal-copy-transport/main.zry");

fn packet(cross: bool, check: impl FnOnce(&[(&str, &str)])) {
    let joined = format!("{VALUES}\n{MAIN}");
    let imported = format!("import {{transport}} from \"./values.zry\";\n{MAIN}");
    let files = if cross {
        vec![("main.zry", imported.as_str()), ("values.zry", VALUES)]
    } else {
        vec![("main.zry", joined.as_str())]
    };
    check(&files);
}

#[test]
fn nominal_copy_target_packet_authenticates_both_independent_dtos_before_transport() {
    for cross in [false, true] {
        packet(cross, |files| {
            let p = project(files);
            assert!(p.syntax.is_bound_to(&p.sources));
            if let Some(dir) = std::env::var_os("ZRYNA_GENERIC_NOMINAL_COPY_FIXTURE_DIR") {
                let dto = crate::bounded_generics_v1::tests::body_fixtures::snapshot(files);
                std::fs::write(
                    std::path::PathBuf::from(dir).join(format!("{}-reference.json", files.len())),
                    serde_json::to_vec_pretty(&dto).expect("independent complete original DTO"),
                )
                .expect("explicit packet output");
            }
            original_types(files);
        });
    }
}

#[test]
fn source_first_nominal_copy_target_packet_seals_simultaneous_loop_carriers() {
    for cross in [false, true] {
        packet(cross, |files| {
            with_claim(files, |claim, syntax, sources, linear, linux| {
                let template = claim
                    .graph
                    .functions
                    .iter()
                    .find(|function| function.key[0] == 0x40)
                    .expect("actual transport specialization");
                let nominal_parameters = template
                    .blocks
                    .iter()
                    .flat_map(|block| &block.parameters)
                    .filter_map(|parameter| match parameter.ty {
                        raw::Type::Stored(id) => linear.types().find(|ty| ty.id().index() == id),
                        _ => None,
                    })
                    .filter(|ty| matches!(ty.key().first(), Some(0x11 | 0x13)))
                    .count();
                assert!(
                    nominal_parameters >= 3,
                    "both nested owners and the phantom marker are transported"
                );
                seal(&claim, syntax, sources, linear, linux);
            });
        });
    }
}
