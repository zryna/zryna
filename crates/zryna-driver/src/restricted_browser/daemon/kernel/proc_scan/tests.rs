use super::*;
use std::{process::Command, thread, time::Duration};

fn future() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn visible_complete_proc_is_required_including_hidden_and_duplicate_mount_rejection() {
    let good = "1 0 0:1 / /proc ro,nosuid,nodev,noexec - proc proc rw\n";
    assert!(visible_proc(good).is_ok());
    assert!(visible_proc(&good.replace("proc rw", "proc rw,hidepid=0")).is_ok());
    for text in [
        good.replace("proc rw", "proc rw,hidepid=1"),
        good.replace("proc rw", "proc rw,hidepid=2"),
        good.replace(" ro,", " ro,hidepid=2,"),
        good.replace("- proc", "- tmpfs"),
        good.replace(" / /proc", " /subset /proc"),
        format!("{good}{good}"),
        String::new(),
        "malformed".to_owned(),
    ] {
        assert!(visible_proc(&text).is_err(), "{text}");
    }
}

#[test]
fn stat_parser_preserves_zombies_and_parent_when_command_contains_parentheses() {
    let fields = |state: &str| format!("42 (odd ) command ( ) {state} 7 {} 12345", "0 ".repeat(17));
    assert_eq!(process(&fields("Z"), 42).expect("zombie record"), (7, 'Z'));
    assert_eq!(process(&fields("S"), 42).expect("live record"), (7, 'S'));
    assert!(process(&fields("S"), 43).is_err());
    assert!(process(&fields("ZZ"), 42).is_err());
    assert!(process(&fields("S").replace("S 7", "S -1"), 42).is_err());
    assert!(process(&fields("S").replace("12345", "invalid"), 42).is_err());
    assert!(process("42 (short) Z 7", 42).is_err());
}

#[test]
fn actual_unreaped_child_remains_visible_until_wait_reaps_it() {
    // Other parallel tests own children too. Run the whole-process reap proof in
    // its own test process so a foreign test's child cannot invalidate the result.
    const ISOLATED: &str = "ZRYNA_REAP_SCAN_TEST_CHILD";
    if std::env::var_os(ISOLATED).is_none() {
        let status = zryna_process::status(
            Command::new(std::env::current_exe().expect("current test executable"))
                .args([
                    "--exact",
                    "restricted_browser::daemon::kernel::proc_scan::tests::actual_unreaped_child_remains_visible_until_wait_reaps_it",
                    "--nocapture",
                ])
                .env(ISOLATED, "1"),
        )
        .expect("isolated actual reap proof");
        assert!(status.success(), "isolated actual reap proof must pass");
        return;
    }
    // try_wait would itself reap the child, so observe its kernel state instead.
    let mut child = Command::new("/bin/true").spawn().expect("owned inert child");
    let result = (|| {
        let deadline = future();
        loop {
            check(deadline)?;
            let text = stat(child.id(), deadline)?
                .ok_or_else(|| reject("TEST", "owned child vanished before wait"))?;
            if process(&text, child.id())?.1 == 'Z' {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        scan(std::process::id(), deadline)
    })();
    child.wait().expect("reap only owned child even if assertion fails");
    assert!(!result.expect("actual zombie observation"), "actual zombie cannot pass reap proof");
    assert!(scan(std::process::id(), future()).expect("complete post-wait scan"));
}

#[test]
fn expired_observation_and_substituted_original_init_fail_closed() {
    let pid = std::process::id();
    let init = Init {
        pid,
        ticks: "0".to_owned(),
        started: "test".to_owned(),
        cid: "a".repeat(64),
        membership: String::new(),
    };
    assert!(children_reaped(&init, future()).is_err());
    assert!(scan(pid, Instant::now()).is_err());
}
