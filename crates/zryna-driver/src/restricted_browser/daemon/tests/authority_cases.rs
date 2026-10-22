use super::*;

#[test]
fn production_source_profiles_fit_closed_rule_budgets_without_waiving_unsplit_extra_names() {
    let profiles = [
        (
            include_str!(
                "../../../../../../examples/playground/restricted/daemon/policies/compiler-amd64.json"
            ),
            12,
        ),
        (
            include_str!(
                "../../../../../../examples/playground/restricted/daemon/policies/evaluator-amd64.json"
            ),
            24,
        ),
    ];
    for (text, expected_rules) in profiles {
        let value =
            policy::seccomp(text).expect("exact tracked independently reviewed production profile");
        let mut bad = value.clone();
        let rules = bad["syscalls"].as_array_mut().expect("rules");
        assert_eq!(rules.len(), expected_rules);
        assert_eq!(rules[0]["names"].as_array().expect("first group").len(), 256);
        let rest = rules.remove(1)["names"].as_array().expect("second group").clone();
        assert_eq!(rest.len(), 97);
        let names = rules[0]["names"].as_array_mut().expect("names");
        names.extend(rest);
        assert_eq!(names.len(), 353);
        assert!(
            policy::seccomp(&bad.to_string()).is_err(),
            "first extra rule names must still reject"
        );
    }
}

#[test]
fn substituted_policy_pins_noncanonical_json_and_kernel_authority_refuse() {
    let good = record();
    assert!(approval::validate(good.clone()).is_ok());
    for mutate in [
        |value: &mut PlaygroundDaemonOperatorApproval| value.version = 1,
        |value: &mut PlaygroundDaemonOperatorApproval| {
            value.composition_kind = "published-toolkit-v1".into();
        },
        |value: &mut PlaygroundDaemonOperatorApproval| {
            value.composition_sha256 = "not-reviewed".into();
        },
        |value: &mut PlaygroundDaemonOperatorApproval| value.cpus = [0, 0],
        |value: &mut PlaygroundDaemonOperatorApproval| {
            value.compiler_image_sha256 = "latest".into();
        },
        |value: &mut PlaygroundDaemonOperatorApproval| value.compiler_root_sha256 = "A".repeat(64),
        |value: &mut PlaygroundDaemonOperatorApproval| value.policy_sha256 = "0".repeat(64),
    ] {
        let mut bad = good.clone();
        mutate(&mut bad);
        assert!(approval::validate(bad).is_err());
    }
    for text in [
        good.compiler_seccomp_json.replacen("SCMP_ACT_ERRNO", "SCMP_ACT_ALLOW", 1),
        format!(" {}", good.compiler_seccomp_json),
        good.compiler_seccomp_json.replace("\"read\"", "\"bpf\""),
        good.compiler_seccomp_json.replace("\"read\"", "\"process_vm_writev\""),
        good.compiler_seccomp_json
            .replace("\"defaultErrnoRet\":1", "\"defaultErrnoRet\":1,\"defaultErrnoRet\":1"),
    ] {
        assert!(policy::seccomp(&text).is_err());
    }
    assert_ne!(playground_daemon_policy_sha256(&good).expect("identity"), {
        let mut altered = good.clone();
        altered.cpus = [2, 3];
        playground_daemon_policy_sha256(&altered).expect("changed")
    });
}

#[test]
fn inherited_image_effects_and_unknown_image_authority_reject_before_resource_creation() {
    let image = format!("sha256:{}", "a".repeat(64));
    let good = json!({"Id":image,"Os":"linux","Architecture":"amd64","Config":{
        "User":"65534:65534","Entrypoint":["/supervisor"],"Cmd":["idle"],"WorkingDir":"/tmp","Env":config::ENV}});
    assert!(image::validate(&good, &image).is_ok());
    for (key, value) in [
        ("Volumes", json!({"/host":{}})),
        ("Healthcheck", json!({"Test":["CMD","evil"]})),
        ("OnBuild", json!(["RUN evil"])),
        ("Env", json!(["SECRET=ambient"])),
        ("User", json!("0")),
        ("Entrypoint", json!(["/bin/sh"])),
        ("Unknown", Value::Null),
    ] {
        let mut bad = good.clone();
        bad["Config"][key] = value;
        assert!(image::validate(&bad, &image).is_err(), "{key}");
    }
    assert!(image::validate(&good, &format!("sha256:{}", "b".repeat(64))).is_err());
}

#[test]
fn effective_mount_or_security_substitution_cannot_satisfy_owned_role_inspection() {
    let approved = approved();
    let cid = "c".repeat(64);
    let image = format!("sha256:{}", "a".repeat(64));
    let request =
        config::container(Operation::Compile, false, &approved, &image, "owner", ["in", "out"]);
    let good = actual(&request, &cid);
    assert!(identity::validate(&good, &cid, "role", &image, &request).is_ok());
    for (key, value) in [
        ("Privileged", json!(true)),
        ("NetworkMode", json!("host")),
        ("ReadonlyRootfs", json!(false)),
        ("CapAdd", json!(["SYS_ADMIN"])),
        ("Binds", json!(["/:/host"])),
        ("PidsLimit", json!(0)),
        ("MemorySwap", json!(-1)),
        ("CpusetCpus", json!("0-7")),
        ("SecurityOpt", json!(["seccomp=unconfined"])),
        ("FutureAuthority", Value::Null),
    ] {
        let mut bad = good.clone();
        bad["HostConfig"][key] = value;
        assert!(identity::validate(&bad, &cid, "role", &image, &request).is_err(), "{key}");
    }
    let mut bad = good.clone();
    bad["Mounts"][0]["RW"] = json!(true);
    assert!(identity::validate(&bad, &cid, "role", &image, &request).is_err());
    let mut bad = good.clone();
    bad["Config"]["Labels"] = json!({"zryna410.owner":"other"});
    assert!(identity::validate(&bad, &cid, "role", &image, &request).is_err());
}

#[test]
fn actual_resource_readbacks_and_fixed_exec_identity_fail_closed() {
    let host = json!({"Memory":536_870_912,"PidsLimit":32});
    let good = json!({"memory_stats":{"limit":536_870_912},"pids_stats":{"limit":32,"current":1}});
    assert!(resources::stats_match(&good, &host).is_ok());
    for current in [0, 33] {
        let mut bad = good.clone();
        bad["pids_stats"]["current"] = json!(current);
        assert!(resources::stats_match(&bad, &host).is_err());
    }
    let mut bad = good.clone();
    bad["memory_stats"]["limit"] = json!(536_870_913);
    assert!(resources::stats_match(&bad, &host).is_err());
    let exec = json!({"ID":"exec","ContainerID":"cid","OpenStdin":false,"OpenStdout":false,"OpenStderr":false,"Running":false,
        "ProcessConfig":{"entrypoint":"/supervisor","arguments":["compile"],"privileged":false,"tty":false,"user":"65534:65534"}});
    assert!(execute::inspect(&exec, "exec", "cid", Operation::Compile, false).is_ok());
    for (key, value) in
        [("ContainerID", json!("other")), ("OpenStdin", json!(true)), ("Running", json!("false"))]
    {
        let mut bad = exec.clone();
        bad[key] = value;
        assert!(execute::inspect(&bad, "exec", "cid", Operation::Compile, false).is_err());
    }
    assert!(execute::inspect(&exec, "exec", "cid", Operation::Evaluate, false).is_err());
}

#[test]
fn evaluator_private_user_maps_keep_fixed_kernel_paths_protected() {
    let approved = approved();
    let cid = "c".repeat(64);
    let image = format!("sha256:{}", "a".repeat(64));
    let evaluator =
        config::container(Operation::Evaluate, false, &approved, &image, "owner", ["in", "out"]);
    let paths = json!(["/proc/bus", "/proc/fs", "/proc/irq", "/proc/sys", "/proc/sysrq-trigger"]);
    assert_eq!(evaluator["HostConfig"]["ReadonlyPaths"], paths);
    for (role, holder) in [(Operation::Compile, false), (Operation::Evaluate, true)] {
        let request = config::container(role, holder, &approved, &image, "owner", ["in", "out"]);
        assert_eq!(request["HostConfig"]["ReadonlyPaths"], json!(["/proc"]));
    }
    let good = actual(&evaluator, &cid);
    assert!(identity::validate(&good, &cid, "role", &image, &evaluator).is_ok());
    for paths in [json!([]), json!(["/proc"]), json!(["/proc/sys"])] {
        let mut bad = good.clone();
        bad["HostConfig"]["ReadonlyPaths"] = paths;
        assert!(identity::validate(&bad, &cid, "role", &image, &evaluator).is_err());
    }
}
