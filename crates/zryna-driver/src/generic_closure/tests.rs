use super::test_source::{CurrentSyntax, MemoryRoot};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
fn entry() -> NormalizedSourcePath {
    NormalizedSourcePath::new("main.zry").expect("entry")
}
fn root() -> MemoryRoot {
    MemoryRoot::new([("main.zry".into(), "export function entry(): i32 { return 7; }".into())])
}
#[test]
fn retained_source_session_postvalidates_prepare_and_preserves_outer_phase() {
    let root = root();
    let frontend = CurrentSyntax::ordinary();
    let failed = with_closure(&root, entry(), &frontend, &|| Ok(()), |_, _| {
        assert!(root.active.load(Ordering::SeqCst));
        root.drift.store(true, Ordering::SeqCst);
        Ok(7)
    })
    .expect_err("source drift after preparation must fail");
    assert_eq!(failed.diagnostics()[0].code(), "ZRYNA-S1001");
    assert!(!root.active.load(Ordering::SeqCst));
    let root = self::root();
    let failed = with_closure(
        &root,
        entry(),
        &frontend,
        &|| {
            Err(CommandFailure {
                kind: crate::CommandFailureKind::Preparation,
                diagnostics: vec![zryna_diagnostics::Diagnostic::error(
                    "fixture-outer",
                    None,
                    "drift",
                    "retain",
                )],
            })
        },
        |_, _| Ok(()),
    )
    .expect_err("outer checkpoint");
    assert_eq!(failed.kind(), crate::CommandFailureKind::Preparation);
    assert_eq!(failed.diagnostics()[0].code(), "fixture-outer");
}
#[test]
fn terminal_scope_blocks_finish_on_drift_and_never_fails_after_finish_success() {
    let root = root();
    let frontend = CurrentSyntax::ordinary();
    let finished = AtomicBool::new(false);
    let failed = with_closure_terminal(
        &root,
        entry(),
        &frontend,
        &|| Ok(()),
        |_, _| {
            root.drift.store(true, Ordering::SeqCst);
            Ok(7)
        },
        |_, _| {
            finished.store(true, Ordering::SeqCst);
            Ok(())
        },
    )
    .expect_err("drift blocks finalizer");
    assert_eq!(failed.diagnostics()[0].code(), "ZRYNA-S1001");
    assert!(!finished.load(Ordering::SeqCst));
    let root = self::root();
    let value = with_closure_terminal(
        &root,
        entry(),
        &frontend,
        &|| Ok(()),
        |_, _| Ok(7),
        |owned, checkpoint| {
            assert!(root.active.load(Ordering::SeqCst));
            checkpoint()?;
            root.drift.store(true, Ordering::SeqCst);
            Ok(owned)
        },
    )
    .expect("successful commit boundary must not get a fallible post-check");
    assert_eq!(value, 7);
    assert!(!root.active.load(Ordering::SeqCst));
}
#[test]
fn foreign_provider_source_drift_and_error_advisory_never_reach_callback() {
    for mode in 0..3 {
        let root = root();
        let frontend = CurrentSyntax {
            foreign: mode == 0,
            drift: (mode == 1).then(|| root.drift.clone()),
            advisory_error: mode == 2,
            ..CurrentSyntax::ordinary()
        };
        let called = AtomicBool::new(false);
        let failure = with_closure(&root, entry(), &frontend, &|| Ok(()), |_, _| {
            called.store(true, Ordering::SeqCst);
            Ok(())
        })
        .expect_err("hostile source/provider boundary");
        assert_eq!(
            failure.diagnostics()[0].code(),
            ["ZRYNA-D3102", "ZRYNA-S1001", "ZRYNA-D3101"][mode]
        );
        assert!(!called.load(Ordering::SeqCst));
        assert!(!root.active.load(Ordering::SeqCst));
    }
}
#[test]
fn source_cycles_invalid_paths_and_insufficient_physical_credit_reject_before_issuance() {
    let cyclic = MemoryRoot::new([(
        "main.zry".into(),
        "import { value } from './main.zry'; export function entry(): i32 { return 7; }".into(),
    )]);
    let failure =
        with_closure(&cyclic, entry(), &CurrentSyntax::ordinary(), &|| Ok(()), |_, _| Ok(()))
            .expect_err("cycle");
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-D3007");
    for units in [0, crate::MAX_MODULE_PROVIDER_CALLS] {
        let root = root();
        let frontend = CurrentSyntax { units, ..CurrentSyntax::ordinary() };
        let failure = with_closure(&root, entry(), &frontend, &|| Ok(()), |_, _| Ok(()))
            .expect_err("provider reserve");
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-D3201");
        assert_eq!(frontend.calls.load(Ordering::SeqCst), 0);
    }
    let bad = MemoryRoot::new([(
        "main.zry".into(),
        "import { value } from '../escape.zry'; export function entry(): i32 { return 7; }".into(),
    )]);
    let failure =
        with_closure(&bad, entry(), &CurrentSyntax::ordinary(), &|| Ok(()), |_, _| Ok(()))
            .expect_err("explicit relative path boundary");
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-D3001");
}
#[test]
fn provider_cleanup_reserve_and_mid_discovery_outer_failure_precede_analysis() {
    let root = root();
    let frontend =
        CurrentSyntax { units: 2, minimum: Duration::from_secs(2), ..CurrentSyntax::ordinary() };
    let start = Instant::now();
    let failure = with_clock(
        &root,
        entry(),
        &frontend,
        &|| Ok(()),
        || start,
        Duration::from_secs(1),
        |_, _| Ok(()),
    )
    .expect_err("two-worker cleanup reserve");
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-D3201");
    assert_eq!(frontend.calls.load(Ordering::SeqCst), 0);
    let root = self::root();
    let frontend = CurrentSyntax::ordinary();
    let checkpoints = std::sync::atomic::AtomicUsize::new(0);
    let outer = || {
        if checkpoints.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(CommandFailure {
                kind: crate::CommandFailureKind::Preparation,
                diagnostics: vec![zryna_diagnostics::Diagnostic::error(
                    "fixture-mid-outer",
                    None,
                    "provider changed",
                    "retain",
                )],
            });
        }
        Ok(())
    };
    let failure = with_closure(&root, entry(), &frontend, &outer, |_, _| Ok(()))
        .expect_err("mid-discovery outer checkpoint");
    assert_eq!(failure.kind(), crate::CommandFailureKind::Preparation);
    assert_eq!(failure.diagnostics()[0].code(), "fixture-mid-outer");
    assert_eq!(frontend.calls.load(Ordering::SeqCst), 0);
    assert!(!root.active.load(Ordering::SeqCst));
}
#[test]
fn clock_boundaries_reject_before_provider_process_work() {
    for backwards in [false, true] {
        let root = root();
        let frontend = CurrentSyntax::ordinary();
        let start = Instant::now();
        let mut tick = 0;
        let clock = || {
            tick += 1;
            if tick == 1 {
                start
            } else if backwards {
                start.checked_sub(Duration::from_secs(1)).expect("earlier monotonic fixture")
            } else {
                start + MAX_MODULE_DISCOVERY_WALL_TIME + Duration::from_secs(1)
            }
        };
        let failure = with_clock(
            &root,
            entry(),
            &frontend,
            &|| Ok(()),
            clock,
            MAX_MODULE_DISCOVERY_WALL_TIME,
            |_, _| Ok(()),
        )
        .expect_err("clock");
        assert_eq!(
            failure.diagnostics()[0].code(),
            if backwards { "ZRYNA-D3102" } else { "ZRYNA-D3201" }
        );
        assert_eq!(frontend.calls.load(Ordering::SeqCst), 0);
    }
}
