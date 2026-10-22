use super::*;
use std::fmt::Write as _;

#[test]
fn material_pin_binds_finite_complete_role_inventory_and_provisioning_identity() {
    for evaluate in [false, true] {
        let pin = material(evaluate);
        assert!(materials::validate_pin(&pin, evaluate, &pin.root_sha256).is_ok());
        for (files, bytes) in [
            (0, 1),
            (1, 0),
            (if evaluate { 513 } else { 65 }, 1),
            (1, if evaluate { 1_073_741_825 } else { 201_326_593 }),
        ] {
            let mut bad = pin.clone();
            bad.files = files;
            bad.bytes = bytes;
            assert!(materials::validate_pin(&bad, evaluate, &pin.root_sha256).is_err());
        }
        for name in [
            "host-secret",
            "zryna410-materials-../host",
            "zryna410-materials-X",
            "zryna410-materials-a?x=1",
        ] {
            let mut bad = pin.clone();
            bad.name = name.into();
            assert!(materials::validate_pin(&bad, evaluate, &pin.root_sha256).is_err());
        }
        assert!(materials::validate_pin(&pin, evaluate, &"0".repeat(64)).is_err());
    }
}

#[test]
fn operator_material_substitution_or_driver_options_fail_before_guest_admission() {
    let pin = material(false);
    let good = json!({"Name":pin.name,"CreatedAt":pin.created_at,"Driver":"local","Scope":"local",
        "Options":null,"Labels":{"zryna410.materials-nonce":pin.nonce,"zryna410.materials-root":pin.root_sha256}});
    assert!(materials::validate_volume(&good, &pin).is_ok());
    for (key, value) in [
        ("Name", json!("other")),
        ("CreatedAt", json!("2026-10-16T00:00:00Z")),
        ("Driver", json!("plugin")),
        ("Scope", json!("global")),
        ("Options", json!({"device":"/host","type":"none","o":"bind"})),
        (
            "Labels",
            json!({"zryna410.materials-nonce":"other","zryna410.materials-root":pin.root_sha256}),
        ),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(materials::validate_volume(&bad, &pin).is_err(), "{key}");
    }
    let mut bad = good.clone();
    bad["Labels"]["extra"] = json!("writable");
    assert!(materials::validate_volume(&bad, &pin).is_err());
}

#[test]
fn actual_complete_audit_receipt_rejects_partial_stale_duplicate_or_invented_counts() {
    let pin = material(false);
    let good =
        json!({"version":1,"rootSha256":pin.root_sha256,"files":pin.files,"bytes":pin.bytes});
    assert!(materials::audit_receipt(&serde_json::to_vec(&good).expect("receipt"), &pin).is_ok());
    for (key, value) in [
        ("version", json!(2)),
        ("rootSha256", json!("0".repeat(64))),
        ("files", json!(pin.files + 1)),
        ("bytes", json!(pin.bytes + 1)),
        ("isolated", json!(true)),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(
            materials::audit_receipt(&serde_json::to_vec(&bad).expect("mutant"), &pin).is_err(),
            "{key}"
        );
    }
    let duplicate = format!(
        "{{\"version\":1,\"version\":1,\"rootSha256\":\"{}\",\"files\":{},\"bytes\":{}}}",
        pin.root_sha256, pin.files, pin.bytes
    );
    assert!(materials::audit_receipt(duplicate.as_bytes(), &pin).is_err());
    let mut exact = serde_json::to_vec(&good).expect("receipt");
    exact.resize(1024, b' ');
    assert!(materials::audit_receipt(&exact, &pin).is_ok());
    exact.push(b' ');
    assert!(materials::audit_receipt(&exact, &pin).is_err());
}

#[test]
fn material_source_readonly_mount_and_audit_phase_cannot_be_skipped() {
    let approved = approved();
    let image = format!("sha256:{}", "a".repeat(64));
    let cid = "c".repeat(64);
    let request =
        config::container(Operation::Compile, false, &approved, &image, "owner", ["in", "out"]);
    let good = actual(&request, &cid);
    assert!(identity::validate(&good, &cid, "role", &image, &request).is_ok());
    let mut bad = good.clone();
    bad["Mounts"][2]["RW"] = json!(true);
    assert!(identity::validate(&bad, &cid, "role", &image, &request).is_err());
    let mut bad = good.clone();
    bad["Mounts"][2]["Name"] = json!("substitute");
    assert!(identity::validate(&bad, &cid, "role", &image, &request).is_err());
    let mut state = state::State { owner: "attempt".into(), phase: state::Phase::Member };
    assert!(state.advance(state::Phase::Member, state::Phase::InputSealed).is_err());
    state.advance(state::Phase::Member, state::Phase::Audited).expect("complete verified audit");
    assert!(state.advance(state::Phase::Audited, state::Phase::Dispatched).is_err());
}

#[test]
fn actual_kernel_security_credentials_cpu_and_duplicate_fields_are_required() {
    let fields = [
        ("NoNewPrivs", "1"),
        ("Threads", "1"),
        ("Seccomp", "2"),
        ("CapInh", "0000000000000000"),
        ("CapPrm", "0000000000000000"),
        ("CapEff", "0000000000000000"),
        ("CapBnd", "0000000000000000"),
        ("CapAmb", "0000000000000000"),
        ("Uid", "65534 65534 65534 65534"),
        ("Gid", "65534 65534 65534 65534"),
        ("Cpus_allowed_list", "0-1"),
    ];
    let good = fields.iter().fold(String::new(), |mut output, (key, value)| {
        writeln!(output, "{key}:\t{value}").expect("fixture string");
        output
    });
    assert!(kernel::security(&good, [0, 1]).is_ok());
    // A readiness marker never permits a threaded init, even if a later read could pass.
    assert!(kernel::security(&good.replace("Threads:\t1\n", "Threads:\t2\n"), [0, 1]).is_err());
    for (key, value) in fields {
        let line = format!("{key}:\t{value}\n");
        assert!(kernel::security(&good.replace(&line, ""), [0, 1]).is_err(), "{key}");
        assert!(kernel::security(&format!("{good}{line}"), [0, 1]).is_err(), "{key}");
    }
    assert!(kernel::security(&good.replace("0-1", "0-2"), [0, 1]).is_err());
    assert!(kernel::security(&good.replace("65534 65534 65534 65534", "0 0 0 0"), [0, 1]).is_err());
}
