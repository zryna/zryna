use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::indexed_handle_source::elements;
use super::*;
use zryna_diagnostics::Diagnostic;

#[test]
fn indexed_handle_regions_static_siblings_and_shared_roots_keep_lexical_rules() {
    for element in elements() {
        for action in [Action::Conflict, Action::RootShared] {
            let (source, raw) = fixture(Container::Array(2), &element, false, action, Some(0));
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated static aliases");
            let program =
                lower(pair_input(&syntax, &sources)).expect("disjoint siblings or shared root");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let begins = instructions
                .iter()
                .filter(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                .collect::<Vec<_>>();
            assert_eq!(begins.len(), 2);
            let places = begins
                .iter()
                .map(|i| i.place_operands().next().expect("static place"))
                .collect::<Vec<_>>();
            assert_ne!(places[0], places[1]);
            assert_eq!(
                instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .map(|i| i.borrow())
                    .collect::<Vec<_>>(),
                [begins[1].borrow(), begins[0].borrow()]
            );
        }
        for container in [Container::Array(2), Container::Vec] {
            let (source, raw) = fixture(container, &element, false, Action::RootShared, None);
            let sources = sources_for(&source);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated dynamic and root aliases");
            let program = lower(pair_input(&syntax, &sources))
                .expect("compatible whole-container shared access");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let indexed =
                instructions.iter().find_map(|i| i.indexed_borrow()).expect("dynamic begin");
            let root = instructions
                .iter()
                .find(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                .expect("root begin");
            assert_eq!(root.place_operands().collect::<Vec<_>>(), [indexed.container()]);
        }
    }
}

#[test]
fn indexed_handle_regions_vec_growth_rejects_before_move_and_recovers_after_end() {
    for element in elements() {
        let (source, raw) = fixture(Container::Vec, &element, false, Action::GrowthInside, None);
        let body = &raw.files[0].functions[0].body;
        let RawStatementKind::ExpressionStatement { expression, .. } =
            body.statements[body.blocks[1].statements[1] as usize].kind
        else {
            panic!("push");
        };
        let at = body.expressions[expression as usize].span;
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated forbidden growth");
        let expected = vec![Diagnostic::error_at(
            "ZRYNA-M3014",
            span(&sources, at),
            "Vec operation conflicts with an active whole-container access",
            "finish the indexed operation before accessing or consuming its container",
        )];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("borrow excludes growth"),
                expected
            );
        }
        let (source, raw) = fixture(Container::Vec, &element, false, Action::GrowthAfter, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated restored growth");
        let program =
            lower(pair_input(&syntax, &sources)).expect("valid recovery after lexical end");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let end = instructions
            .iter()
            .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("end");
        let push = instructions
            .iter()
            .position(|i| i.kind() == VerifiedInstructionKind::VecPush)
            .expect("push");
        assert!(end < push);
        let begin = instructions.iter().find_map(|i| i.indexed_borrow()).expect("begin");
        assert_eq!(instructions[push].failure_ended_borrows().len(), 0);
        let drops = instructions[push].derived_drop_actions().collect::<Vec<_>>();
        assert!(drops.iter().any(|a| a.root() == begin.container()));
        assert_eq!(
            drops.len(),
            2,
            "prepared element and original Vec both survive reserve failure"
        );
        assert!(drops.iter().all(|a| a.moved_projections().len() == 0));
    }
}
