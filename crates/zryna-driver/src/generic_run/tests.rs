//! Genuine original-source receipts; completion is obtained only through actual typed hosts.
use super::*;
use zryna_abi::{ScalarOutcome, ScalarValue};
mod host;
mod source;
#[test]
fn missing_node_and_unadmitted_components_never_prepare_a_guest() {
    for targets in [
        TargetSelection::JavaScript,
        TargetSelection::WebAssembly,
        TargetSelection::All,
        TargetSelection::Component,
        TargetSelection::BrowserComponent,
    ] {
        assert!(GenericRunHosts::prepare(targets, None).is_err());
    }
}
#[test]
fn exact_source_and_scalar_request_validation_precedes_host_work() {
    let prior = source::with(|_, receipt, _| receipt.clone());
    source::with(|program, receipt, _| {
        validate(program, receipt, "entry".into(), vec![]).expect("same world scalar request");
        let absent =
            validate(program, receipt, "absent".into(), vec![]).expect_err("unknown source export");
        assert_eq!(absent.kind(), CommandFailureKind::Request);
        assert_eq!(absent.diagnostics()[0].code(), "ZRYNA-B2101");
        assert_eq!(
            validate(program, receipt, "entry".into(), vec![ScalarValue::I32(1)])
                .expect_err("arity")
                .diagnostics()[0]
                .code(),
            "ZRYNA-B2102"
        );
        assert_eq!(prior.execution_sha256(), receipt.execution_sha256());
        assert_eq!(
            validate(program, &prior, "entry".into(), vec![])
                .expect_err("same bytes foreign world")
                .diagnostics()[0]
                .code(),
            "ZRYNA-C3413"
        );
    });
}
#[test]
fn native_error_vectors_keep_cleanup_priority_in_both_phases() {
    for phase in [CommandFailureKind::Preparation, CommandFailureKind::Execution] {
        for code in ["ZRYNA-N4009", "ZRYNA-N4016"] {
            let errors =
                vec![zryna_diagnostics::Diagnostic::error(code, None, "cleanup failed", "retain")];
            assert_eq!(failure::native(errors, phase).kind(), CommandFailureKind::Cleanup);
        }
        let errors = vec![zryna_diagnostics::Diagnostic::error(
            "fixture-host",
            None,
            "host failed",
            "retain",
        )];
        assert_eq!(failure::native(errors, phase).kind(), phase);
    }
}
#[test]
fn actual_node_targets_issue_typed_completion_and_reject_foreign_projection() {
    let root = host::Root::new();
    let node = host::node(&root);
    let output = root.output();
    let foreign = source::with(|_, receipt, _| receipt.clone());
    for targets in [TargetSelection::JavaScript, TargetSelection::WebAssembly] {
        let hosts = GenericRunHosts::prepare(targets, Some(&node)).expect("retained admitted Node");
        source::with(|program, receipt, checkpoint| {
            let completed = execute(
                program,
                receipt,
                targets,
                &hosts,
                &output,
                "entry".into(),
                vec![],
                checkpoint,
            )
            .expect("actual typed host outcome");
            let (record, native) =
                completed.into_records(receipt).expect("matching owned source receipt");
            assert!(native.is_none());
            assert_eq!(record.export, "entry");
            assert!(record.arguments.is_empty());
            assert_eq!(record.outcomes.len(), 1);
            assert_eq!(
                record.outcomes[0].outcome,
                ScalarOutcome::Returned { value: ScalarValue::I32(14) }
            );
            assert_eq!(
                record.outcomes[0].target,
                if targets.javascript() {
                    GenericTarget::JavaScript
                } else {
                    GenericTarget::WebAssembly
                }
            );
            let completed = execute(
                program,
                receipt,
                targets,
                &hosts,
                &output,
                "entry".into(),
                vec![],
                checkpoint,
            )
            .expect("actual second host completion");
            assert!(
                completed.into_records(&foreign).is_err(),
                "foreign world cannot publish real completion as its own"
            );
        });
    }
}
#[test]
fn failed_postguest_checkpoint_returns_no_completion_record() {
    let root = host::Root::new();
    let node = host::node(&root);
    let output = root.output();
    let hosts =
        GenericRunHosts::prepare(TargetSelection::JavaScript, Some(&node)).expect("Node host");
    source::with(|program, receipt, checkpoint| {
        let count = std::cell::Cell::new(0);
        let check = || {
            checkpoint()?;
            count.set(count.get() + 1);
            if count.get() == 4 {
                Err(failure::boundary("retained source changed after guest"))
            } else {
                Ok(())
            }
        };
        assert!(
            execute(
                program,
                receipt,
                TargetSelection::JavaScript,
                &hosts,
                &output,
                "entry".into(),
                vec![],
                &check
            )
            .is_err()
        );
        assert_eq!(count.get(), 4, "normal typed host completion reached its postguest checkpoint");
        assert!(std::fs::read_dir(output.path()).expect("output inventory").next().is_none());
    });
}
