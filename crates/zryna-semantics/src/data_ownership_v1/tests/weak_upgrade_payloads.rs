use super::generic_vec_fixture::weak_upgrade_fixture::payload_fixture::Payload;
use super::generic_vec_fixture::weak_upgrade_fixture::{Case, payload_fixture};
use super::*;

#[test]
fn weak_upgrade_payloads_authenticate_exact_success_type_and_cleanup_for_every_shape() {
    for payload in Payload::ALL {
        for temporary in [false, true] {
            let (source, raw) = payload_fixture(
                if temporary { Case::Temporary } else { Case::Addressable },
                payload,
            );
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources)
                .unwrap_or_else(|errors| panic!("{payload:?}: {source}\n{errors:?}"));
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{payload:?}: {source}\n{errors:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let blocks = function.blocks().collect::<Vec<_>>();
            let upgrade = blocks[0].terminator();
            let operand = upgrade.place_operands().next().expect("exact Weak place");
            let (success, expired) = upgrade.weak_upgrade_edges().expect("upgrade edges");
            let yes =
                blocks.iter().find(|block| block.id() == success.target()).expect("success block");
            let no =
                blocks.iter().find(|block| block.id() == expired.target()).expect("expired block");
            let parameters = yes.parameters().collect::<Vec<_>>();
            assert_eq!(parameters.len(), 1);
            assert_eq!(
                parameters[0].ty(),
                function.result_type(),
                "exact Shared<T> success payload"
            );
            assert_eq!(no.parameters().count(), 0);
            let shared = blocks[0]
                .instructions()
                .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakDowngrade)
                .expect("downgrade")
                .place_operands()
                .next()
                .expect("retained Shared root");
            let weak = if temporary {
                blocks[0]
                    .instructions()
                    .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
                    .expect("explicit temporary clone")
                    .place_operands()
                    .next()
                    .expect("retained Weak root")
            } else {
                operand
            };
            let expected = if temporary { vec![operand, weak, shared] } else { vec![weak, shared] };
            assert_eq!(
                upgrade.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                yes.terminator()
                    .derived_drop_actions()
                    .map(|action| action.root())
                    .collect::<Vec<_>>(),
                [weak, shared]
            );
            assert_eq!(
                no.terminator()
                    .derived_drop_actions()
                    .map(|action| action.root())
                    .collect::<Vec<_>>(),
                [weak]
            );
            let construct = blocks[0]
                .instructions()
                .find(|instruction| instruction.kind() == VerifiedInstructionKind::SharedConstruct)
                .expect("payload publication");
            assert_eq!(construct.result_type(), Some(function.result_type()));
            assert_eq!(
                format!("{program:?}"),
                format!(
                    "{:?}",
                    lower(pair_input(&syntax, &sources)).expect("same typed program replay")
                )
            );
        }
    }
}
