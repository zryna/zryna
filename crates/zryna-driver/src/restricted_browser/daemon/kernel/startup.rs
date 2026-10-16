//! Scheduling barrier for the fixed image entrypoint, never execution admission.
use super::{Init, Result, read, reject};
use serde_json::Value;
use std::time::Instant;

pub(in super::super) fn startup_epoch(
    original: &Init,
    current: &Init,
    state: &Value,
) -> Result<()> {
    if original != current || state["State"]["OOMKilled"] != false {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "original startup init epoch changed"));
    }
    Ok(())
}

fn marker(text: &str, init: &Init) -> Result<bool> {
    let invalid = || reject("PLAYGROUND-HOST-KERNEL", "original init startup record invalid");
    let (pid, rest) = text.split_once(" (").ok_or_else(invalid)?;
    if pid != init.pid.to_string() {
        return Err(invalid());
    }
    let (comm, fields) = rest.rsplit_once(") ").ok_or_else(invalid)?;
    if comm.is_empty() || comm.len() > 15 || comm.contains(['\0', '\n', '\r']) {
        return Err(invalid());
    }
    let fields = fields.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 20
        || !matches!(fields[0], "R" | "S" | "D" | "I")
        || fields[19] != init.ticks
        || !fields[19].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    // comm is mutable readiness metadata. The unchanged security capture must still pass.
    Ok(comm == "supervisor")
}

pub(in super::super) fn supervisor_ready(init: &Init, deadline: Instant) -> Result<bool> {
    marker(&read(init.pid, "stat", deadline)?, init)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn init() -> Init {
        Init {
            pid: 42,
            ticks: "123".into(),
            started: "epoch".into(),
            cid: "a".repeat(64),
            membership: "group".into(),
        }
    }
    fn record(comm: &str, state: &str, ticks: &str) -> String {
        let mut fields = vec!["0"; 52];
        fields[0] = state;
        fields[19] = ticks;
        format!("42 ({comm}) {}\n", fields.join(" "))
    }

    #[test]
    fn only_exact_original_supervisor_marker_is_ready() {
        for comm in ["runtime-init", "runc:[2:INIT]", "supervisor-x", "a)b"] {
            assert!(!marker(&record(comm, "S", "123"), &init()).unwrap());
        }
        assert!(marker(&record("supervisor", "R", "123"), &init()).unwrap());
    }

    #[test]
    fn malformed_reused_and_dead_records_are_errors_never_pending() {
        for text in [
            "".into(),
            "42 supervisor S".into(),
            "42 () S".into(),
            record("supervisor", "S", "124"),
            record("supervisor", "S", "+123"),
            record("supervisor", "S", "123").replacen("42 (", "43 (", 1),
            record("supervisor\n", "S", "123"),
            record("abcdefghijklmnop", "S", "123"),
        ] {
            assert!(marker(&text, &init()).is_err(), "{text:?}");
        }
        for state in ["Z", "X", "x", "T", "t", "?", "SS"] {
            assert!(marker(&record("supervisor", state, "123"), &init()).is_err());
        }
    }

    #[test]
    fn startup_cannot_adopt_another_epoch_or_oom_state() {
        let good = json!({"State":{"OOMKilled":false}});
        for field in ["pid", "ticks", "started", "cid", "membership"] {
            let mut changed = init();
            match field {
                "pid" => changed.pid += 1,
                "ticks" => changed.ticks = "124".into(),
                "started" => changed.started = "other".into(),
                "cid" => changed.cid = "b".repeat(64),
                "membership" => changed.membership = "other".into(),
                _ => unreachable!(),
            }
            assert!(startup_epoch(&init(), &changed, &good).is_err(), "{field}");
        }
        assert!(startup_epoch(&init(), &init(), &good).is_ok());
        for bad in [json!({}), json!({"State":{"OOMKilled":true}})] {
            assert!(startup_epoch(&init(), &init(), &bad).is_err());
        }
    }

    #[test]
    fn expired_original_clock_refuses_before_reading_a_startup_marker() {
        let expired = Instant::now().checked_sub(std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(
            supervisor_ready(&init(), expired).unwrap_err().reason(),
            "PLAYGROUND-HOST-DEADLINE"
        );
    }
}
