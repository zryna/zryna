//! Both module forms authenticate independently before ownership admission or layout issuance.
use super::{assert_original_types, owned_v2, project, raw, with_claim, with_packet};

#[test]
fn nominal_enum_target_packet_authenticates_and_emits_both_independent_dtos() {
    for cross in [false, true] {
        with_packet(cross, |files| {
            let p = project(files);
            assert!(p.syntax.is_bound_to(&p.sources));
            // Export each original source DTO before the separate ownership admission tests.
            // In particular, a first-form discovery rejection cannot suppress the second form.
            if let Some(dir) = std::env::var_os("ZRYNA_GENERIC_NOMINAL_ENUM_FIXTURE_DIR") {
                let dto = crate::bounded_generics_v1::tests::body_fixtures::snapshot(files);
                std::fs::write(
                    std::path::PathBuf::from(dir).join(format!("{}-reference.json", files.len())),
                    serde_json::to_vec_pretty(&dto).expect("independent source DTO"),
                )
                .expect("explicit fixture output");
            }
            assert_original_types(files);
        });
    }
}

#[test]
fn source_first_nominal_enum_target_packet_seals_and_rejects_claim_mutations() {
    for cross in [false, true] {
        with_packet(cross, |files| {
            with_claim(files, |claim, syntax, sources, linear, linux| {
                let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
                    zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux)
                        .expect("runtime claims"),
                    linear,
                    linux,
                )
                .expect("runtime authority");
                let check = |claim: &owned_v2::raw::Program| {
                    owned_v2::verify(
                        owned_v2::wire::decode(&owned_v2::wire::encode(claim).expect("wire"))
                            .expect("decode"),
                        syntax,
                        sources,
                        sources.verify_file_id(0).expect("entry"),
                        linear,
                        linux,
                        &runtime,
                    )
                    .map(|_| ())
                };
                check(&claim).expect("single/imported nominal source seal");
                let scalar = linear
                    .types()
                    .find(|ty| ty.key() == [1])
                    .expect("genuine i32 type")
                    .id()
                    .index();
                for mutation in 0..5 {
                    let mut hostile = claim.clone();
                    let instruction = hostile.graph.functions.iter_mut()
                        .flat_map(|function| &mut function.blocks)
                        .flat_map(|block| &mut block.instructions)
                        .find(|instruction| matches!(instruction.operation,
                            raw::Operation::ClosedEnumConstruct { ty, .. }
                            if linear.types().find(|entry| entry.id().index() == ty)
                                .is_some_and(|entry| matches!(entry.key().first(), Some(0x11 | 0x13)))))
                        .expect("genuine nominal constructor");
                    let raw::Operation::ClosedEnumConstruct { ty, ordinal, payload } =
                        &mut instruction.operation
                    else {
                        panic!("constructor");
                    };
                    match mutation {
                        0 => *ordinal = u32::MAX,
                        1 => *ty = scalar,
                        2 => *payload = Some(u32::MAX),
                        3 => instruction.result.ty = raw::Type::Stored(scalar),
                        _ => *ordinal = ordinal.wrapping_add(1),
                    }
                    assert!(
                        check(&hostile).is_err(),
                        "nominal identity/payload mutation {mutation}"
                    );
                    check(&claim).expect("pristine source replay");
                }
                let mut missing = claim.clone();
                missing
                    .plans
                    .iter_mut()
                    .flat_map(|plan| &mut plan.steps)
                    .find(|step| !step.cleanup.is_empty())
                    .expect("nominal payload cleanup")
                    .cleanup
                    .clear();
                assert!(check(&missing).is_err(), "claimed cleanup cannot omit nominal ownership");
                check(&claim).expect("pristine replay after hostile cleanup");
            });
        });
    }
}
