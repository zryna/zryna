use super::*;

#[test]
fn structured_string_reads_retain_exact_places_and_temporary_owners_across_match() {
    for mode in 0..3 {
        let (text, raw) = structured_owned_fixture::string_match_fixture(mode);
        let sources = sources_for(&text);
        let syntax =
            verify_snapshot(raw, &sources).expect("authenticated String read with nested match");
        let program = lower(pair_input(&syntax, &sources))
            .expect("String continuation preserves read lifetimes");
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function");
        let blocks = function.blocks().collect::<Vec<_>>();
        let tail = blocks[3].instructions().next().expect("String read after match");
        assert_eq!(
            tail.kind(),
            if mode == 0 {
                VerifiedInstructionKind::StringClone
            } else {
                VerifiedInstructionKind::StringConcat
            }
        );
        assert_eq!(tail.derived_drop_actions().len(), if mode == 0 { 1 } else { 2 });
        assert_eq!(
            blocks[3].terminator().derived_drop_actions().len(),
            if mode == 0 { 1 } else { 2 }
        );
        if mode == 2 {
            let named = function
                .places()
                .find(|place| matches!(place.kind(), VerifiedPlaceKind::Parameter(1)))
                .expect("retained named parameter")
                .id();
            assert_eq!(tail.place_operands().next(), Some(named));
            assert!(
                blocks[3].instructions().skip(1).any(|instruction| instruction.kind()
                    == VerifiedInstructionKind::MoveFromPlace
                    && instruction.place_operands().next() == Some(named)),
                "retained read exclusion releases after String completion"
            );
            assert_eq!(blocks[0].instructions().len(), 1);
            assert_eq!(
                blocks[0].instructions().next().expect("scrutinee evaluation only").kind(),
                VerifiedInstructionKind::MoveFromPlace
            );
        }
        let replay = lower(pair_input(&syntax, &sources)).expect("String graph replay");
        assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
    }
}

#[test]
fn structured_string_read_allows_clone_but_excludes_move_with_exact_replay_diagnostic() {
    for cloned in [false, true] {
        let (mut text, mut raw) = if cloned {
            structured_owned_fixture::string_match_fixture(2)
        } else {
            structured_owned_fixture::string_move_match_fixture()
        };
        let expression = raw.files[0].functions[0].body.expressions.iter_mut().find(|expression| matches!(&expression.kind, zryna_syntax::v4::RawExpressionKind::Reference { name } if name.text == "first")).expect("arm source reference");
        let at = expression.span;
        text.replace_range(at.start as usize..at.end as usize, "saved");
        let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expression.kind else {
            unreachable!("arm reference")
        };
        name.text = "saved".into();
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated retained source move");
        if cloned {
            assert!(
                lower(pair_input(&syntax, &sources)).is_ok(),
                "retained read permits read-only overlapping clone"
            );
            continue;
        }
        let first = lower(pair_input(&syntax, &sources)).expect_err("retained source cannot move");
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources))
                .expect_err("deterministic retained-read rejection")
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), "ZRYNA-M3014");
        assert_eq!(first[0].primary_span(), Some(span(&sources, at)));
        assert_eq!(first[0].message(), "owned access conflicts with a retained String operand");
        assert_eq!(
            first[0].guidance(),
            "finish the String operation before consuming or mutating its retained operand"
        );
    }
}
