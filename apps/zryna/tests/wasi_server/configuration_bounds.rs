//! Real CLI capture/admission at the unchanged configuration and listener byte ceilings.

use super::support::{Case, common, exchange, frame, guard};
use std::fs;

use super::clock_fixture::{Granted, document};

const SOURCE: &str = "examples/wasi-server/status-200.zry";

#[test]
fn configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles() {
    let _guard = guard();
    let original = Case::new(1);
    let mut bytes = fs::read(&original.configuration.path).expect("valid configuration JSON");
    assert!(bytes.len() < 1024);
    bytes.resize(1024, b' ');
    for clock in [false, true] {
        check(Case::from_config(&bytes), clock, true);
    }
    bytes.push(b' ');
    for clock in [false, true] {
        check(Case::from_config(&bytes), clock, false);
    }
}

#[test]
fn listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles() {
    let _guard = guard();
    for size in [1024, 1025] {
        for clock in [false, true] {
            let mut case = Case::new(1);
            let mut bytes =
                fs::read(&case.approval.path).expect("valid digest-bound listener approval");
            assert!(bytes.len() < 1024);
            bytes.resize(size, b' ');
            case.approval = common::Input::from_bytes(&bytes);
            check(case, clock, size == 1024);
        }
    }
}

fn check(case: Case, clock: bool, accepted: bool) {
    if clock {
        let granted = Granted::for_case(case, document());
        if !accepted {
            super::source_tests::assert_selection_rejected(
                &granted.case,
                "server-clock-status-v1",
                granted.command(SOURCE),
            );
            return;
        }
        let mut running = granted.spawn(SOURCE);
        assert!(
            exchange(running.address, &frame(running.address, b"")).starts_with(b"HTTP/1.1 200 ")
        );
        let (exit, result) = running.finish();
        assert!(exit.success(), "{result}");
        let record = granted.manifest();
        assert_eq!(record["execution"]["clock_reads"], 1);
        assert_eq!(record["execution"]["teardown"]["confirmed"], true);
    } else {
        if !accepted {
            super::source_tests::assert_selection_rejected(
                &case,
                "server-status-v1",
                case.command(SOURCE),
            );
            return;
        }
        let mut running = case.spawn(SOURCE);
        assert!(
            exchange(running.address, &frame(running.address, b"")).starts_with(b"HTTP/1.1 200 ")
        );
        let (exit, result) = running.finish();
        assert!(exit.success(), "{result}");
        assert_eq!(case.manifest()["execution"]["teardown"]["confirmed"], true);
    }
}
