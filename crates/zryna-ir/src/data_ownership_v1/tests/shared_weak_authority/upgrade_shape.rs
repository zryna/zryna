use super::*;
use crate::data_ownership_v1::WeakUpgradeShape;
use zryna_layout::{TypeCategory, TypeId, VerifiedLayouts};

fn fixture_type(layouts: &VerifiedLayouts, ty: raw::TypeId) -> TypeId {
    layouts
        .types()
        .find(|record| record.id().index() == ty.0)
        .expect("fixture type was derived from these sealed layouts")
        .id()
}

#[test]
fn upgrade_shape_derives_all_payload_categories_and_is_reusable_across_storage_targets() {
    for payload in Payload::ALL {
        let fixture = Fixture::new(payload);
        let weak = fixture_type(&fixture.linear, fixture.weak);
        let shape = WeakUpgradeShape::derive(&fixture.linear, weak).expect("exact Weak schema");
        assert_eq!(shape.weak_type(), weak);
        assert_eq!(shape.referent_type(), fixture_type(&fixture.linear, fixture.payload));
        assert_eq!(shape.success_parameter_type(), fixture_type(&fixture.linear, fixture.shared));
        let copied = shape;
        assert_eq!(shape, copied);
        assert_eq!(Some(shape), WeakUpgradeShape::derive(&fixture.linear, weak));
        assert_eq!(Some(shape), WeakUpgradeShape::derive(&fixture.linux, weak));
        for layouts in [&fixture.linear, &fixture.linux] {
            for ty in layouts.types().filter(|ty| ty.category() != TypeCategory::Weak) {
                assert_eq!(WeakUpgradeShape::derive(layouts, ty.id()), None, "{payload:?}");
            }
        }
    }
}

#[test]
fn upgrade_shape_rejects_a_foreign_universe_even_at_the_same_valid_weak_index() {
    let string = Fixture::new(Payload::String);
    let integer = Fixture::new(Payload::I32);
    let own = fixture_type(&string.linear, string.weak);
    let foreign = fixture_type(&integer.linear, integer.weak);
    assert_eq!(own.index(), foreign.index(), "same in-range index cannot authenticate a type");
    assert_ne!(own.universe_identity(), foreign.universe_identity());
    assert!(WeakUpgradeShape::derive(&string.linear, own).is_some());
    assert!(WeakUpgradeShape::derive(&integer.linear, foreign).is_some());
    assert_eq!(WeakUpgradeShape::derive(&string.linear, foreign), None);
    assert_eq!(WeakUpgradeShape::derive(&integer.linear, own), None);
}

fn unmatched_layouts(wrong_shared: bool, target: StorageTarget) -> VerifiedLayouts {
    let (sources, _, _) = authorities();
    let file = sources.verify_file_id(0).expect("entry");
    let mut types = vec![
        raw_layout::TypeNode {
            id: raw_layout::NodeId(0),
            span: None,
            kind: raw_layout::TypeKind::Bool,
        },
        raw_layout::TypeNode {
            id: raw_layout::NodeId(1),
            span: None,
            kind: raw_layout::TypeKind::I32,
        },
        raw_layout::TypeNode {
            id: raw_layout::NodeId(2),
            span: None,
            kind: raw_layout::TypeKind::String,
        },
        raw_layout::TypeNode {
            id: raw_layout::NodeId(3),
            span: None,
            kind: raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) },
        },
    ];
    let mut program_roots = vec![raw_layout::NodeId(3)];
    if wrong_shared {
        types.push(raw_layout::TypeNode {
            id: raw_layout::NodeId(4),
            span: None,
            kind: raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(1) },
        });
        program_roots.push(raw_layout::NodeId(4));
    }
    let graph = raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 0,
        }],
        types,
        program_roots,
    };
    zryna_layout::verify(&graph, &sources, target)
        .expect("valid graph without a matching Shared type")
}

#[test]
fn upgrade_shape_requires_a_matching_shared_referent_not_just_any_shared_type() {
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        for wrong_shared in [false, true] {
            let layouts = unmatched_layouts(wrong_shared, target);
            let weak =
                layouts.types().find(|ty| ty.category() == TypeCategory::Weak).expect("Weak type");
            assert_eq!(
                layouts.types().filter(|ty| ty.category() == TypeCategory::Shared).count(),
                usize::from(wrong_shared)
            );
            if let Some(shared) = layouts.types().find(|ty| ty.category() == TypeCategory::Shared) {
                assert_ne!(shared.referenced_type(), weak.referenced_type());
            }
            assert_eq!(WeakUpgradeShape::derive(&layouts, weak.id()), None);
        }
    }
}

#[test]
fn upgrade_shape_prefixes_only_the_success_schema_without_validating_owner_flow() {
    let fixture = Fixture::new(Payload::String);
    let shape =
        WeakUpgradeShape::derive(&fixture.linear, fixture_type(&fixture.linear, fixture.weak))
            .expect("shape");
    let integer = fixture_type(&fixture.linear, raw::TypeId(1));
    let boolean = fixture_type(&fixture.linear, raw::TypeId(0));
    for ordinary in [vec![], vec![integer], vec![integer, boolean, shape.weak_type()]] {
        let success = std::iter::once(shape.success_parameter_type())
            .chain(ordinary.iter().copied())
            .collect::<Vec<_>>();
        let expired = ordinary.clone();
        assert_eq!(success.len(), ordinary.len() + 1);
        assert_eq!(success[0], fixture_type(&fixture.linear, fixture.shared));
        assert_eq!(success[1..], ordinary);
        assert_eq!(expired, ordinary);
    }
}

#[test]
fn upgrade_shape_composition_still_requires_full_ir_cleanup_and_edge_verification() {
    for payload in Payload::ALL {
        let fixture = Fixture::new(payload);
        let shape =
            WeakUpgradeShape::derive(&fixture.linear, fixture_type(&fixture.linear, fixture.weak))
                .expect("shape before building a CFG");
        // The raw fixture remains independent of the descriptor. Its parameter
        // claim is then composed from the planning shape and independently verified.
        let mut raw = fixture.program();
        let f = &mut raw.modules[0].functions[0];
        let synthetic = raw::TypeId(shape.success_parameter_type().index());
        f.blocks[1].parameters[0].ty = synthetic;
        f.places[5].ty = synthetic;
        let expected = sealed_trace(&fixture, raw.clone());
        for invalid_cleanup in [false, true] {
            let mut forged = raw.clone();
            let f = &mut forged.modules[0].functions[0];
            if invalid_cleanup {
                f.cleanup_plans[5].actions.clear();
            } else {
                let raw::Terminator::WeakUpgradeBranch { success, .. } =
                    &mut f.blocks[0].terminators[0].kind
                else {
                    panic!("upgrade")
                };
                success.arguments.push(raw::ValueId(1));
            }
            reject_and_replay(
                &fixture,
                forged,
                if invalid_cleanup { "ZRYNA-I3012" } else { "ZRYNA-I3007" },
            );
            assert_eq!(Some(shape), WeakUpgradeShape::derive(&fixture.linear, shape.weak_type()));
            assert_eq!(expected, sealed_trace(&fixture, raw.clone()));
        }
    }
}
