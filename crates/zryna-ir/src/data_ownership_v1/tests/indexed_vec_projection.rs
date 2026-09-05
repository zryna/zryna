use super::indexed_access::chained;
use super::indexed_access_resources::at_capacity;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn indexed_vec_projection_retains_bounds_region_and_exact_owned_replacement() {
    for container in [Container::Array, Container::Vec] {
        for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            let fixture = Fixture::new(container, Element::Vec);
            let mut program = chained(&fixture, access);
            let string = raw::TypeId(
                fixture
                    .linear
                    .types()
                    .find(|ty| ty.category() == TypeCategory::String)
                    .expect("String")
                    .id()
                    .index(),
            );
            let function = &mut program.modules[0].functions[0];
            function.parameters[4].ty = string;
            function.places[2].ty = string;
            if access == raw::BorrowAccess::Exclusive {
                function.blocks[0].instructions.insert(
                    2,
                    raw::Instruction {
                        result: None,
                        span: function.span,
                        kind: raw::InstructionKind::BorrowReplace {
                            borrow: raw::BorrowId(1),
                            value: raw::ValueId(4),
                        },
                    },
                );
                function.cleanup_plans[1].actions.remove(0);
            }
            let verified = fixture.verify(program);
            let function =
                verified.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let projection = block.instructions().nth(1).expect("projection");
            let view = projection.indexed_projection().expect("Vec child");
            assert_eq!(view.array_length(), None);
            assert_eq!(view.referent().index(), string.0);
            assert_eq!(view.container().index(), 0);
            assert_eq!(view.access(), access.into());
            assert_eq!(view.trap_identity(), super::super::VerifiedTrapIdentity::BoundsV1);
            assert_eq!(
                projection
                    .failure_ended_borrows()
                    .map(super::super::BorrowIdentity::index)
                    .collect::<Vec<_>>(),
                [0]
            );
            assert_eq!(
                projection
                    .derived_drop_actions()
                    .map(|drop| drop.root().index())
                    .collect::<Vec<_>>(),
                [2, 1, 0]
            );
            if access == raw::BorrowAccess::Exclusive {
                let commit = block.instructions().nth(2).expect("commit");
                assert_eq!(
                    commit.borrow_replacement().expect("replacement").referent().index(),
                    string.0
                );
                assert_eq!(commit.derived_drop_actions().count(), 0);
                assert_eq!(
                    block
                        .terminator()
                        .derived_drop_actions()
                        .map(|drop| drop.root().index())
                        .collect::<Vec<_>>(),
                    [1, 0]
                );
            }
        }
    }
}

#[test]
fn indexed_vec_projection_rejects_wrong_index_and_retired_parent_and_recovers() {
    let fixture = Fixture::new(Container::Vec, Element::Vec);
    let valid = chained(&fixture, raw::BorrowAccess::Shared);
    fixture.verify(valid.clone());
    let mut wrong = valid.clone();
    if let raw::InstructionKind::ProjectIndexedBorrow { index, .. } =
        &mut wrong.modules[0].functions[0].blocks[0].instructions[1].kind
    {
        *index = raw::ValueId(0);
    }
    fixture.rejects(wrong, "ZRYNA-I3005");
    let mut retired = valid.clone();
    let function = &mut retired.modules[0].functions[0];
    function.blocks[0].instructions.insert(1, end_borrow(0, function.span));
    fixture.rejects(retired, "ZRYNA-I3011");
    fixture.verify(valid);
}

#[test]
fn indexed_vec_projection_rejects_zero_stride_parent_element() {
    let (sources, _, _) = authorities();
    let file = sources.verify_file_id(0).expect("file");
    let kinds = [
        raw_layout::TypeKind::I32,
        raw_layout::TypeKind::String,
        raw_layout::TypeKind::FixedArray { element: raw_layout::NodeId(1), length: 0 },
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
        raw_layout::TypeKind::FixedArray { element: raw_layout::NodeId(3), length: 2 },
        raw_layout::TypeKind::Bool,
    ];
    let graph = raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 0,
        }],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(id, kind)| raw_layout::TypeNode {
                id: raw_layout::NodeId(u32::try_from(id).expect("small graph")),
                span: None,
                kind,
            })
            .collect(),
        program_roots: (0..6).map(raw_layout::NodeId).collect(),
    };
    // Such a parent cannot reach IR verification: the mandatory layout authority
    // rejects it on both targets before a projection can acquire sealed types.
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let errors = zryna_layout::verify(&graph, &sources, target).expect_err("Vec zero stride");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-L3003");
        assert_eq!(errors[0].message(), "Vec does not admit a zero-sized element type");
    }
}

#[test]
#[ignore = "full Vec projection active-authority exact/first-extra boundary"]
fn indexed_vec_projection_active_exact_first_extra_and_recovery() {
    let fixture = Fixture::new(Container::Vec, Element::Vec);
    fixture.verify(at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION));
    fixture.rejects(at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION + 1), "ZRYNA-I3201");
    fixture.verify(at_capacity(&fixture, 1));
}
