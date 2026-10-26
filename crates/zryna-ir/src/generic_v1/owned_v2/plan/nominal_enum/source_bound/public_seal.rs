//! Public source/layout/runtime authority and hostile wire replay for nominal enums.
use super::*;

fn verify_claim(
    claim: &owned_v2::raw::Program,
    f: &Fixture,
    linear: &layout::VerifiedLayouts,
    linux: &layout::VerifiedLayouts,
) -> Result<(), Failure> {
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux).expect("runtime claims"),
        linear,
        linux,
    )
    .expect("runtime declaration authority");
    let bytes = owned_v2::wire::encode(claim)?;
    owned_v2::verify(
        owned_v2::wire::decode(&bytes)?,
        &f.syntax,
        &f.sources,
        f.sources.verify_file_id(0).expect("entry"),
        linear,
        linux,
        &runtime,
    )
    .map(|_| ())
}

#[test]
fn both_nominal_forms_reject_hostile_wire_plans_inventories_ordinals_payloads_and_replay() {
    for form in [1, 2] {
        let f = Fixture::new(form);
        let (linear, linux) = f.layouts(&f.graph);
        let keys = f
            .syntax
            .files()
            .iter()
            .flat_map(|unit| {
                unit.functions
                    .iter()
                    .enumerate()
                    .filter(|(_, function)| function.type_parameters.is_some())
                    .map(move |(index, function)| {
                        assert_eq!(
                            function.type_parameters.as_ref().expect("generic").parameters.len(),
                            1
                        );
                        key(0x40, unit.id, u32::try_from(index).expect("function"), &[&[2]])
                    })
            })
            .collect::<Vec<_>>();
        let borrowed = keys.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let claim = owned_v2::produce_claim(&f.syntax, &f.sources, &linear, &linux, &borrowed)
            .expect("nominal raw source producer reaches the independent public verifier");
        verify_claim(&claim, &f, &linear, &linux)
            .expect("public nominal source/layout/runtime seal");
        let pristine = owned_v2::wire::encode(&claim).expect("canonical nominal wire");
        for attack in 0..8 {
            let mut hostile = claim.clone();
            tamper(&mut hostile, attack);
            assert!(
                verify_claim(&hostile, &f, &linear, &linux).is_err(),
                "public hostile nominal attack {attack}"
            );
            verify_claim(&claim, &f, &linear, &linux)
                .expect("pristine public sealing after hostile retry");
            assert_eq!(owned_v2::wire::encode(&claim).expect("pristine wire"), pristine);
        }
        let foreign = Fixture::new(form);
        assert!(
            verify_claim(&claim, &foreign, &linear, &linux).is_err(),
            "equal text has foreign source authority"
        );
    }
}

fn tamper(hostile: &mut owned_v2::raw::Program, attack: u8) {
    match attack {
        0 => {
            hostile.graph.declarations.pop();
        }
        1 => hostile.graph.declarations[0].span.start += 1,
        2 => hostile.graph.type_keys[0].push(0),
        3 => {
            hostile
                .plans
                .iter_mut()
                .find(|p| p.steps.iter().any(|s| !s.cleanup.is_empty()))
                .expect("owned plan")
                .steps
                .iter_mut()
                .find(|s| !s.cleanup.is_empty())
                .expect("cleanup")
                .cleanup
                .pop();
        }
        4 => {
            let op = hostile
                .graph
                .functions
                .iter_mut()
                .flat_map(|f| &mut f.blocks)
                .flat_map(|b| &mut b.instructions)
                .find_map(|i| match &mut i.operation {
                    graph::Operation::ClosedEnumConstruct { ordinal, .. } => Some(ordinal),
                    _ => None,
                })
                .expect("constructor");
            *op = u32::MAX;
        }
        5 => {
            let instruction = hostile
                .graph
                .functions
                .iter_mut()
                .flat_map(|f| &mut f.blocks)
                .flat_map(|b| &mut b.instructions)
                .find(|i| {
                    matches!(
                        i.operation,
                        graph::Operation::ClosedEnumConstruct { payload: Some(_), .. }
                    )
                })
                .expect("payload constructor");
            let graph::Operation::ClosedEnumConstruct { payload, .. } = &mut instruction.operation
            else {
                panic!("constructor")
            };
            *payload = None;
        }
        6 => {
            hostile.extensions.iter_mut().find(|e| !e.is_empty()).expect("effects").pop();
        }
        _ => hostile.graph.modules[0].functions += 1,
    }
}
