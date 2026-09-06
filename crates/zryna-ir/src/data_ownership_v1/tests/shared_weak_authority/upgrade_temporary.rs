use super::*;

fn temporary_program(fixture: &Fixture) -> raw::Program {
    let mut program = fixture.program();
    let function = &mut program.modules[0].functions[0];
    for index in [1, 2] {
        function.blocks[index].instructions.push(raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(4) },
        });
    }
    function.cleanup_plans[5] = fixtures::cleanup(5, vec![5, 3, 2, 1], function.span);
    function.cleanup_plans[6] = fixtures::cleanup(6, vec![3, 2, 1], function.span);
    program
}

#[test]
fn weak_upgrade_temporary_edges_reject_forgery_and_cleanup_corruption_then_recover() {
    let fixture = Fixture::new(Payload::String);
    let pristine = temporary_program(&fixture);
    let valid = fixture.verify(pristine.clone()).expect("independent complete temporary baseline");
    let function = valid.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(
        blocks[0]
            .instructions()
            .last()
            .expect("Weak clone")
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [3, 2, 1]
    );
    assert_eq!(
        blocks[0]
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [4, 3, 2, 1]
    );
    for block in &blocks[1..] {
        let drop = block.instructions().next().expect("temporary cleanup first");
        assert_eq!(drop.kind(), VerifiedInstructionKind::DropPlace);
        assert_eq!(
            drop.place_operands()
                .map(super::super::super::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            [4]
        );
        assert!(
            !block.terminator().derived_drop_actions().any(|action| action.root().index() == 4)
        );
    }
    for (case, code) in
        [(0, "ZRYNA-I3014"), (1, "ZRYNA-I3012"), (2, "ZRYNA-I3012"), (3, "ZRYNA-I3012")]
    {
        let mut hostile = pristine.clone();
        let function = &mut hostile.modules[0].functions[0];
        match case {
            0 => {
                function.blocks[1].parameters[0].ty = fixture.weak;
                function.places[5].ty = fixture.weak;
            }
            1 => {
                function.cleanup_plans[4].actions.remove(0);
            }
            2 => function.cleanup_plans[4].actions.swap(0, 1),
            3 => function.blocks[2].instructions.clear(),
            _ => unreachable!("four isolated corruptions"),
        }
        let first = fixture.verify(hostile.clone()).expect_err("hostile upgrade");
        assert!(first.iter().any(|diagnostic| diagnostic.code() == code), "case {case}: {first:?}");
        assert_eq!(first, fixture.verify(hostile).expect_err("deterministic complete diagnostics"));
        let recovered = fixture.verify(pristine.clone()).expect("pristine recovery");
        assert_eq!(format!("{valid:?}"), format!("{recovered:?}"));
    }
}
