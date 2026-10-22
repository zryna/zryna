use super::*;
use serde_json::json;
use std::time::Duration;

fn cid() -> String {
    "a".repeat(64)
}

fn running() -> Value {
    json!({"Id":cid(),"RestartCount":0,"State":{"Pid":42,"Running":true,
        "Paused":false,"Restarting":false,"OOMKilled":false,"StartedAt":"original-start"}})
}

fn group() -> String {
    format!("0::/../docker/{}\n", cid())
}

fn original() -> Init {
    observed(&running(), &cid(), "12345".to_owned(), &group(), "12345")
        .expect("fixed original ownership")
}

#[test]
fn ownership_survives_security_refusal_without_authorizing_admission() {
    let anchor = Some(original());
    let refused = super::super::security("NoNewPrivs:\t0\n", [0, 1]);
    assert_eq!(refused.expect_err("admission stays refused").reason(), "PLAYGROUND-HOST-KERNEL");
    assert!(same_original(anchor.as_ref().expect("retained original"), &original()).is_ok());
    // OOM health is admission policy, not permission to erase original ownership.
    let mut unhealthy = running();
    unhealthy["State"]["OOMKilled"] = json!(true);
    assert!(observed(&unhealthy, &cid(), "12345".into(), &group(), "12345").is_ok());
}

#[test]
fn every_original_identity_component_must_match_before_cleanup() {
    for field in ["pid", "ticks", "started", "cid", "membership"] {
        let mut changed = original();
        match field {
            "pid" => changed.pid = 43,
            "ticks" => changed.ticks = "12346".into(),
            "started" => changed.started = "replacement-start".into(),
            "cid" => changed.cid = "b".repeat(64),
            "membership" => changed.membership = format!("0::/elsewhere/docker/{}", cid()),
            _ => unreachable!(),
        }
        assert!(same_original(&original(), &changed).is_err(), "{field}");
    }
}

#[test]
fn ownership_observation_rejects_ambiguous_epoch_cgroup_and_start_ticks() {
    for (key, value) in [
        ("Pid", json!(0)),
        ("Pid", json!(1)),
        ("Pid", json!(u64::MAX)),
        ("Running", json!(false)),
        ("Paused", json!(true)),
        ("Restarting", json!(true)),
        ("StartedAt", json!(NEVER_STARTED)),
    ] {
        let mut changed = running();
        changed["State"][key] = value;
        assert!(observed(&changed, &cid(), "12345".into(), &group(), "12345").is_err(), "{key}");
    }
    let mut changed = running();
    changed["Id"] = json!("b".repeat(64));
    assert!(observed(&changed, &cid(), "12345".into(), &group(), "12345").is_err());
    changed = running();
    changed["RestartCount"] = json!(1);
    assert!(observed(&changed, &cid(), "12345".into(), &group(), "12345").is_err());
    for text in [String::new(), "0::/foreign\n".into(), format!("{}{}", group(), group())] {
        assert!(observed(&running(), &cid(), "12345".into(), &text, "12345").is_err());
    }
    for (first, last) in [("12345", "12346"), ("", ""), ("invalid", "invalid")] {
        assert!(observed(&running(), &cid(), first.into(), &group(), last).is_err());
    }
}

#[test]
fn missing_anchor_cannot_be_replaced_by_a_later_running_or_exited_init() {
    let future = Instant::now() + Duration::from_secs(2);
    assert!(cleanup_owned(None, &running(), &cid(), future).is_err());
    let mut ended = running();
    ended["State"]["Running"] = json!(false);
    ended["State"]["Pid"] = json!(0);
    ended["State"]["Status"] = json!("exited");
    assert!(cleanup_owned(None, &ended, &cid(), future).is_err());
    assert!(cleanup_owned(Some(&original()), &ended, &cid(), future).is_ok());
    ended["State"]["StartedAt"] = json!("replacement-start");
    assert!(cleanup_owned(Some(&original()), &ended, &cid(), future).is_err());
}

#[test]
fn only_proved_never_started_epoch_can_be_removed_without_an_anchor() {
    let future = Instant::now() + Duration::from_secs(2);
    let created = json!({"Id":cid(),"RestartCount":0,"State":{"Pid":0,"Running":false,
        "Paused":false,"Restarting":false,"Status":"created",
        "StartedAt":NEVER_STARTED,"FinishedAt":NEVER_STARTED}});
    assert!(cleanup_owned(None, &created, &cid(), future).is_ok());
    for (key, value) in [
        ("Pid", json!(42)),
        ("Status", json!("exited")),
        ("StartedAt", json!("original-start")),
        ("FinishedAt", json!("finished")),
    ] {
        let mut changed = created.clone();
        changed["State"][key] = value;
        assert!(cleanup_owned(None, &changed, &cid(), future).is_err(), "{key}");
    }
    assert!(cleanup_owned(None, &created, &cid(), Instant::now()).is_err());
}
