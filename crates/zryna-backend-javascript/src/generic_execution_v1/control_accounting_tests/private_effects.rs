//! Real ledger/control wrappers and genuine retained Clone/drop glue; seeded counts are not source ownership.
use super::*;
use crate::generic_execution_v1::{Context, Writer, accounting_tests, runtime::accounting, shapes};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
#[test]
fn fixed_control_debt_rows_bind_actual_wrappers_and_retained_helpers() {
    let oracle = include_str!("private_effects/HELD_COUNTER_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "608f189fd8e21d44f567ee41ee3aedec6f65ccd1c8ddb35542bb8a8bbf20e9ce"
    );
    for provider in source::PROVIDERS {
        source::with_program("control-live-count-call-loop", provider, |program, _| {
            let mut context = Context::new(program).expect("actual control type owner");
            context.accounting = accounting::Plan::derive(&context).expect("prepaid effect census");
            assert!(
                context.accounting.as_ref().is_some_and(|p| p.bound(&context)),
                "actual control activation"
            );
            let frontier = |kind| {
                program
                    .operations()
                    .filter_map(
                        zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier,
                    )
                    .find(|c| c.frontier().kind() == kind)
                    .expect("actual source Clone role")
                    .ty()
            };
            let shared = frontier(CloneNodeKind::SharedCount);
            let weak = frontier(CloneNodeKind::WeakCount);
            let payload = context
                .ty(shared)
                .expect("actual Shared type")
                .referenced_type()
                .expect("actual referent")
                .index();
            let layout = context.control(payload).expect("same-program Linear32 control metadata");
            let mut counter = Writer::counter(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
            shapes::declarations(&context, &mut counter).expect("actual retained glue count");
            let mut writer =
                Writer::output(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES, counter.bytes())
                    .expect("bounded real glue");
            shapes::declarations(&context, &mut writer)
                .expect("actual retained Clone and cleanup bodies");
            let script = format!(
                "{}\nconst $heldSource={};\nconst $controlLayout=Object.freeze({{size:{},alignment:{}}});\n{}\n{}\nconst $sharedClone=$clone{shared},$retainedWeakClone=$clone{weak},$sharedDrop=$drop{shared},$weakDrop=$drop{weak};\n{}",
                crate::prelude::JAVASCRIPT_PRELUDE,
                serde_json::to_string(oracle).expect("unchanged oracle literal"),
                layout.size(),
                layout.alignment(),
                accounting::LEDGER,
                writer.finish(),
                include_str!("private_effects/cases.mjs")
            );
            let receipt =
                accounting_tests::private_boundary(&format!("control-held-{provider}"), &script);
            assert_eq!(receipt["fixedDebtWrapperRows"], 19);
            assert_eq!(receipt["genericOverflowModelRows"], 1);
            assert_eq!(receipt["actualRetainedCloneDrop"], true);
            assert_eq!(receipt["seededPrivateEvidence"], true);
        });
    }
}
