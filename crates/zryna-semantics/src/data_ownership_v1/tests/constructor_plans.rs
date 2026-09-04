use super::super::owned_constructor_plan::{
    ConstructorKind, ConstructorPlanError, ConstructorShape, PreparedConstructor,
};
use super::super::owner_state::{OwnerDelta, apply_owner_delta};
use super::super::type_model::Ty;
use super::*;
use zryna_layout::{self as layout, raw as graph};

impl ConstructorShape {
    fn prepare_definitions(
        self,
        definitions: &[raw::ValueDefinition],
        owners: &OwnerState,
    ) -> Result<PreparedConstructor, ConstructorPlanError> {
        let values = definitions.iter().map(|definition| definition.id).collect::<Vec<_>>();
        self.prepare(
            &values,
            |id| {
                definitions
                    .iter()
                    .find(|definition| definition.id == id)
                    .map(|definition| definition.ty)
            },
            owners,
        )
    }
}

fn fixture(sources: &SourceMap, target: layout::StorageTarget) -> layout::VerifiedLayouts {
    fixture_with_fields(sources, target, false)
}

fn fixture_with_fields(
    sources: &SourceMap,
    target: layout::StorageTarget,
    swapped: bool,
) -> layout::VerifiedLayouts {
    let file = sources.verify_file_id(0).expect("file");
    let at = sources.span(file, 0, 1).expect("nominal span");
    let node = graph::NodeId;
    let kinds =
        vec![
            graph::TypeKind::Bool,
            graph::TypeKind::I32,
            graph::TypeKind::String,
            graph::TypeKind::FixedArray { element: node(2), length: 0 },
            graph::TypeKind::FixedArray { element: node(2), length: 2 },
            graph::TypeKind::Vec { element: node(2) },
            graph::TypeKind::Shared { payload: node(2) },
            graph::TypeKind::Weak { payload: node(2) },
            graph::TypeKind::Enum {
                module: graph::ModuleId(0),
                declaration: 0,
                variants: std::iter::once(graph::Variant { ordinal: 0, payload: None })
                    .chain((0..8).map(|index| graph::Variant {
                        ordinal: index + 1,
                        payload: Some(node(index)),
                    }))
                    .collect(),
            },
            graph::TypeKind::Struct {
                module: graph::ModuleId(0),
                declaration: 1,
                fields: (0..9)
                    .map(|index| graph::Field {
                        ordinal: index,
                        ty: node(if swapped && index < 2 { 1 - index } else { index }),
                    })
                    .collect(),
            },
            graph::TypeKind::FixedArray { element: node(9), length: 2 },
            graph::TypeKind::Vec { element: node(9) },
            graph::TypeKind::Struct {
                module: graph::ModuleId(0),
                declaration: 2,
                fields: vec![graph::Field { ordinal: 0, ty: node(13) }],
            },
            graph::TypeKind::Vec { element: node(12) },
        ];
    let types = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| graph::TypeNode {
            id: node(u32::try_from(index).expect("small graph")),
            span: matches!(kind, graph::TypeKind::Struct { .. } | graph::TypeKind::Enum { .. })
                .then_some(at),
            kind,
        })
        .collect();
    layout::verify(
        &graph::Graph {
            modules: vec![graph::Module {
                id: graph::ModuleId(0),
                source_file: file,
                data_declarations: 3,
            }],
            types,
            program_roots: (0..14).map(node).collect(),
        },
        sources,
        target,
    )
    .expect("complete payload layout")
}

fn ty(record: layout::VerifiedType<'_>) -> Ty {
    Ty {
        layout: record.id(),
        ir: raw::TypeId(record.id().index()),
        category: record.category(),
        drop_kind: record.drop_kind(),
        runtime_kind: record.runtime_kind(),
        cloneable: true,
    }
}

fn shape(
    layouts: &layout::VerifiedLayouts,
    result: Ty,
    kind: ConstructorKind,
    count: usize,
) -> Result<ConstructorShape, ConstructorPlanError> {
    ConstructorShape::derive(layouts, result, kind, count, |id| layouts.type_by_id(id).map(ty))
}

#[test]
fn constructor_shapes_cover_complete_payloads_and_commit_each_owner_once() {
    let sources = sources_for("x");
    let at = sources
        .span(sources.verify_file_id(0).expect("verified constructor fixture"), 0, 1)
        .expect("verified constructor fixture");
    for target in [layout::StorageTarget::Linear32V1, layout::StorageTarget::LinuxX8664V1] {
        let layouts = fixture(&sources, target);
        for record in layouts.types() {
            let cases = match record.category() {
                layout::TypeCategory::Struct => vec![(
                    ConstructorKind::Struct,
                    record.fields().iter().map(|field| field.ty()).collect::<Vec<_>>(),
                )],
                layout::TypeCategory::Enum => record
                    .variants()
                    .iter()
                    .map(|variant| {
                        (
                            ConstructorKind::Enum { variant: variant.ordinal() },
                            variant.payload().into_iter().collect(),
                        )
                    })
                    .collect(),
                layout::TypeCategory::FixedArray => vec![(
                    ConstructorKind::FixedArray,
                    vec![
                        record.referenced_type().expect("verified constructor fixture");
                        usize::try_from(record.array_length().expect("array length"))
                            .expect("bounded fixture array")
                    ],
                )],
                layout::TypeCategory::Vec => [0, 2]
                    .into_iter()
                    .map(|length| {
                        (
                            ConstructorKind::Vec,
                            vec![
                                record.referenced_type().expect("verified constructor fixture");
                                length
                            ],
                        )
                    })
                    .collect(),
                _ => continue,
            };
            for (kind, children) in cases {
                let mut owners = OwnerState::default();
                let definitions = children
                    .iter()
                    .enumerate()
                    .map(|(index, child)| {
                        let id = raw::ValueId(
                            u32::try_from(index).expect("verified constructor fixture"),
                        );
                        if !ty(layouts.type_by_id(*child).expect("verified constructor fixture"))
                            .is_copy()
                        {
                            assert!(owners.register(id, raw::PlaceId(id.0)).is_some());
                        }
                        raw::ValueDefinition { id, ty: raw::TypeId(child.index()), span: at }
                    })
                    .collect::<Vec<_>>();
                let expected = owners.pending().to_vec();
                let prepared = shape(&layouts, ty(record), kind, children.len())
                    .expect("verified constructor fixture")
                    .prepare_definitions(&definitions, &owners)
                    .expect("typed batch");
                let cleanup = (kind == ConstructorKind::Vec).then_some(raw::CleanupPlanId(0));
                assert!(prepared.instruction(cleanup).is_ok());
                assert!(
                    prepared
                        .instruction(if cleanup.is_some() {
                            None
                        } else {
                            Some(raw::CleanupPlanId(0))
                        })
                        .is_err()
                );
                let result = raw::PlaceId(100);
                assert!(owners.register(raw::ValueId(100), result).is_some());
                assert_eq!(
                    prepared.commit(&mut owners),
                    expected
                        .into_iter()
                        .map(|owner| OwnerDelta::Transferred { owner })
                        .collect::<Vec<_>>()
                );
                assert_eq!(owners.pending(), &[result]);
            }
        }
    }
}

#[test]
fn constructor_shapes_reject_wrong_layout_types_and_arity() {
    let sources = sources_for("x");
    let layouts = fixture(&sources, layout::StorageTarget::Linear32V1);
    let aggregate = ty(layouts
        .types()
        .find(|record| record.nominal_identity() == Some((0, 1)))
        .expect("verified constructor fixture"));
    assert!(shape(&layouts, aggregate, ConstructorKind::Struct, 8).is_err());
    assert!(shape(&layouts, aggregate, ConstructorKind::Vec, 9).is_err());
    let mut forged = aggregate;
    forged.drop_kind = 0;
    assert!(shape(&layouts, forged, ConstructorKind::Struct, 9).is_err());
    forged = aggregate;
    forged.ir = raw::TypeId(999);
    assert!(shape(&layouts, forged, ConstructorKind::Struct, 9).is_err());
    let other = fixture_with_fields(&sources, layout::StorageTarget::Linear32V1, true);
    let foreign = ty(other
        .types()
        .find(|record| record.nominal_identity() == Some((0, 1)))
        .expect("verified constructor fixture"));
    assert_eq!(foreign.layout.index(), aggregate.layout.index());
    // Swapped nominal field types change the universe without changing canonical indices.
    assert_ne!(foreign.layout, aggregate.layout);
    assert!(shape(&layouts, foreign, ConstructorKind::Struct, 9).is_err());
    let enumeration = ty(layouts
        .types()
        .find(|record| record.category() == layout::TypeCategory::Enum)
        .expect("verified constructor fixture"));
    assert!(shape(&layouts, enumeration, ConstructorKind::Enum { variant: 99 }, 0).is_err());
    assert!(
        ConstructorShape::derive(&layouts, aggregate, ConstructorKind::Struct, 9, |_| Some(
            aggregate
        ))
        .is_err()
    );
}

#[test]
fn constructor_preparation_rejects_missing_stale_duplicate_and_wrong_typed_operands() {
    let sources = sources_for("x");
    let layouts = fixture(&sources, layout::StorageTarget::Linear32V1);
    let array = ty(layouts
        .types()
        .find(|record| {
            record.category() == layout::TypeCategory::FixedArray
                && record.array_length() == Some(2)
                && layouts
                    .type_by_id(record.referenced_type().expect("verified constructor fixture"))
                    .expect("verified constructor fixture")
                    .category()
                    == layout::TypeCategory::String
        })
        .expect("verified constructor fixture"));
    let at = sources
        .span(sources.verify_file_id(0).expect("verified constructor fixture"), 0, 1)
        .expect("verified constructor fixture");
    let string = layouts
        .type_by_id(array.layout)
        .expect("verified constructor fixture")
        .referenced_type()
        .expect("verified constructor fixture");
    let valid = [0, 1].map(|index| raw::ValueDefinition {
        id: raw::ValueId(index),
        ty: raw::TypeId(string.index()),
        span: at,
    });
    for mode in 0..5 {
        let mut owners = OwnerState::default();
        assert!(owners.register(valid[0].id, raw::PlaceId(0)).is_some());
        assert!(owners.register(valid[1].id, raw::PlaceId(1)).is_some());
        let mut definitions = valid;
        let expected = match mode {
            0 => {
                owners.value_owners.remove(&valid[1].id);
                ConstructorPlanError::MissingOwner
            }
            1 => {
                owners.pending.pop();
                ConstructorPlanError::UnavailableOwner
            }
            2 => {
                definitions[1] = definitions[0];
                ConstructorPlanError::DuplicateOwner
            }
            3 => {
                definitions[1].ty = raw::TypeId(999);
                ConstructorPlanError::WrongType
            }
            _ => {
                definitions[1].id = raw::ValueId(99);
                ConstructorPlanError::MissingOwner
            }
        };
        let before = owners.clone();
        assert_eq!(
            shape(&layouts, array, ConstructorKind::FixedArray, 2)
                .expect("verified constructor fixture")
                .prepare_definitions(&definitions, &owners)
                .err(),
            Some(expected)
        );
        assert_eq!(owners, before);
        let replay = OwnerState {
            pending: vec![raw::PlaceId(0), raw::PlaceId(1)],
            value_owners: valid
                .iter()
                .map(|definition| (definition.id, raw::PlaceId(definition.id.0)))
                .collect(),
        };
        assert!(
            shape(&layouts, array, ConstructorKind::FixedArray, 2)
                .expect("verified constructor fixture")
                .prepare_definitions(&valid, &replay)
                .is_ok()
        );
    }
}

#[test]
fn constructor_owner_batch_failure_is_atomic_and_deltas_clear_only_consumed_metadata() {
    let mut owners = OwnerState::default();
    for index in 0..3 {
        assert!(owners.register(raw::ValueId(index), raw::PlaceId(index)).is_some());
    }
    let before = owners.clone();
    for values in [vec![raw::ValueId(0), raw::ValueId(99)], vec![raw::ValueId(0), raw::ValueId(0)]]
    {
        assert_eq!(owners.transfer_batch(&values), None);
        assert_eq!(owners, before);
    }
    let mut bytes = (0..3)
        .map(|index| (raw::PlaceId(index), u64::from(index)))
        .collect::<std::collections::BTreeMap<_, _>>();
    for delta in owners
        .transfer_batch(&[raw::ValueId(2), raw::ValueId(0)])
        .expect("verified constructor fixture")
    {
        apply_owner_delta(&mut bytes, delta);
    }
    assert_eq!(owners.pending(), &[raw::PlaceId(1)]);
    assert_eq!(bytes.into_iter().collect::<Vec<_>>(), vec![(raw::PlaceId(1), 1)]);
    let after = owners.clone();
    assert_eq!(owners.transfer_batch(&[raw::ValueId(2)]), None);
    assert_eq!(owners, after);
    assert_eq!(owners.transfer_batch(&[]), Some(vec![]));
}

#[test]
fn constructor_commit_rejects_stale_owner_identity_before_any_transfer() {
    let sources = sources_for("x");
    let layouts = fixture(&sources, layout::StorageTarget::Linear32V1);
    let enumeration = ty(layouts
        .types()
        .find(|record| record.category() == layout::TypeCategory::Enum)
        .expect("verified constructor fixture"));
    let string = layouts
        .types()
        .find(|record| record.category() == layout::TypeCategory::String)
        .expect("verified constructor fixture");
    let at = sources
        .span(sources.verify_file_id(0).expect("verified constructor fixture"), 0, 1)
        .expect("verified constructor fixture");
    let definition = raw::ValueDefinition {
        id: raw::ValueId(0),
        ty: raw::TypeId(string.id().index()),
        span: at,
    };
    let mut owners = OwnerState::default();
    assert!(owners.register(definition.id, raw::PlaceId(0)).is_some());
    let prepared = shape(&layouts, enumeration, ConstructorKind::Enum { variant: 3 }, 1)
        .expect("verified constructor fixture")
        .prepare_definitions(&[definition], &owners)
        .expect("verified constructor fixture");
    assert!(owners.transfer(definition.id).is_some());
    assert!(owners.register(definition.id, raw::PlaceId(9)).is_some());
    let before = owners.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| prepared.commit(&mut owners)))
            .is_err()
    );
    assert_eq!(owners, before);
}

#[test]
fn constructor_copy_value_reuse_does_not_issue_or_consume_an_owner() {
    let sources = sources_for("x");
    let layouts = fixture(&sources, layout::StorageTarget::Linear32V1);
    let enumeration = ty(layouts
        .types()
        .find(|record| record.category() == layout::TypeCategory::Enum)
        .expect("verified constructor fixture"));
    let boolean = layouts
        .types()
        .find(|record| record.category() == layout::TypeCategory::Bool)
        .expect("verified constructor fixture");
    let at = sources
        .span(sources.verify_file_id(0).expect("verified constructor fixture"), 0, 1)
        .expect("verified constructor fixture");
    let definition = raw::ValueDefinition {
        id: raw::ValueId(0),
        ty: raw::TypeId(boolean.id().index()),
        span: at,
    };
    let mut owners = OwnerState::default();
    for _ in 0..2 {
        let prepared = shape(&layouts, enumeration, ConstructorKind::Enum { variant: 1 }, 1)
            .expect("verified constructor fixture")
            .prepare_definitions(&[definition], &owners)
            .expect("verified constructor fixture");
        assert!(prepared.commit(&mut owners).is_empty());
        assert!(owners.pending().is_empty());
    }
    assert!(owners.register(definition.id, raw::PlaceId(0)).is_some());
    assert_eq!(
        shape(&layouts, enumeration, ConstructorKind::Enum { variant: 1 }, 1)
            .expect("verified constructor fixture")
            .prepare_definitions(&[definition], &owners)
            .err(),
        Some(ConstructorPlanError::WrongType)
    );
}
