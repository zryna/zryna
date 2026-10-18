use super::super::{cleanup::inspect_for_removal, resources::Container};
use super::*;

fn pending() -> Container {
    let approved = approved();
    Container {
        name: "role".into(),
        cid: None,
        requested: config::container(
            Operation::Compile,
            false,
            &approved,
            &format!("sha256:{}", "a".repeat(64)),
            "owner",
            ["in", "out"],
        ),
        init: None,
        removed: false,
    }
}

#[test]
fn cancelled_create_can_appear_after_the_first_cleanup_lookup() {
    let item = pending();
    let created = actual(&item.requested, &"c".repeat(64));
    let mut lookups = 0;
    let observed = inspect_for_removal(&item, future(), || {
        lookups += 1;
        // Docker commits the already-issued create after cleanup's first 404.
        Ok((lookups > 1).then(|| created.clone()))
    })
    .expect("reconcile the attempted create before authorizing removal");
    assert_eq!(observed, created);
    assert_eq!(lookups, 2);
}

#[test]
fn confirmed_container_absence_is_not_retried() {
    let mut item = pending();
    item.cid = Some("c".repeat(64));
    let mut lookups = 0;
    let error = inspect_for_removal(&item, future(), || {
        lookups += 1;
        Ok(None)
    })
    .expect_err("a confirmed container cannot be replaced by a later create");
    assert_eq!(error.reason(), "PLAYGROUND-HOST-TEARDOWN");
    assert_eq!(lookups, 1);
}

#[test]
fn uncertain_create_never_extends_the_existing_cleanup_deadline() {
    let item = pending();
    let deadline = Instant::now() + Duration::from_millis(20);
    let error = inspect_for_removal(&item, deadline, || Ok(None))
        .expect_err("unconfirmed creation must retain unknown cleanup");
    assert_eq!(error.reason(), "PLAYGROUND-HOST-DEADLINE");
    assert!(Instant::now() >= deadline);
}

#[test]
fn expired_cleanup_does_not_issue_another_inspection() {
    let error = inspect_for_removal(&pending(), Instant::now(), || {
        panic!("expired cleanup cannot perform another request")
    })
    .expect_err("the caller's deadline is binding");
    assert_eq!(error.reason(), "PLAYGROUND-HOST-DEADLINE");
}

#[test]
fn confirmed_container_id_cannot_be_substituted() {
    let mut item = pending();
    item.cid = Some("c".repeat(64));
    let replacement = actual(&item.requested, &"d".repeat(64));
    let error = inspect_for_removal(&item, future(), || Ok(Some(replacement.clone())))
        .expect_err("matching name and config cannot replace the original CID");
    assert_eq!(error.reason(), "PLAYGROUND-HOST-TEARDOWN");
}

#[test]
fn cleanup_transport_errors_are_not_retried() {
    let mut lookups = 0;
    let error = inspect_for_removal(&pending(), future(), || {
        lookups += 1;
        Err(reject("PLAYGROUND-HOST-TRANSPORT", "actual lookup failure"))
    })
    .expect_err("only an uncertain create's 404 may be observed again");
    assert_eq!(error.reason(), "PLAYGROUND-HOST-TRANSPORT");
    assert_eq!(lookups, 1);
}

#[test]
fn late_container_still_requires_the_complete_original_identity() {
    let item = pending();
    for (field, value) in [
        ("Name", json!("/foreign")),
        ("Image", json!(format!("sha256:{}", "b".repeat(64)))),
        ("Id", json!("short")),
    ] {
        let mut created = actual(&item.requested, &"c".repeat(64));
        created[field] = value;
        let mut lookups = 0;
        assert!(
            inspect_for_removal(&item, future(), || {
                lookups += 1;
                Ok((lookups > 1).then(|| created.clone()))
            })
            .is_err(),
            "{field}"
        );
        assert_eq!(lookups, 2, "{field}");
    }
    let mut created = actual(&item.requested, &"c".repeat(64));
    created["Config"]["Labels"] = json!({"zryna410.owner":"foreign"});
    assert!(inspect_for_removal(&item, future(), || Ok(Some(created.clone()))).is_err());
}
