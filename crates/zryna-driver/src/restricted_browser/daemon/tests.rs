use super::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod archive_cases;
mod authority_cases;
mod cleanup_cases;
mod materials_cases;

fn future() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

fn record() -> PlaygroundDaemonOperatorApproval {
    let seccomp = json!({"architectures":["SCMP_ARCH_X86_64"],"defaultAction":"SCMP_ACT_ERRNO",
        "defaultErrnoRet":1,"syscalls":[{"action":"SCMP_ACT_ALLOW","names":["read","write"]}]})
    .to_string();
    let mut record = PlaygroundDaemonOperatorApproval {
        version: 2,
        composition_kind: "reviewed-source-v1".into(),
        composition_sha256: "4".repeat(64),
        engine_uid: 65534,
        compiler_image_sha256: "a".repeat(64),
        evaluator_image_sha256: "b".repeat(64),
        compiler_root_sha256: "c".repeat(64),
        evaluator_root_sha256: "d".repeat(64),
        compiler_materials: material(false),
        evaluator_materials: material(true),
        cpus: [0, 1],
        compiler_executable_sha256: "1".repeat(64),
        binding_template_sha256: "2".repeat(64),
        wit_closure_sha256: "3".repeat(64),
        compiler_seccomp_json: seccomp.clone(),
        evaluator_seccomp_json: seccomp,
        policy_sha256: "e".repeat(64),
    };
    record.policy_sha256 =
        playground_daemon_policy_sha256(&record).expect("fixture policy identity");
    record
}

fn approved() -> approval::Approved {
    approval::validate(record()).expect("explicit fixture approval")
}

#[test]
fn source_exact_first_extra_and_closed_carriers_precede_all_host_effects() {
    let request = |source: &str| {
        serde_json::to_vec(&json!({"version":1,"revision":1,"source":source})).expect("request")
    };
    assert!(owner::source_request(&request(&"a".repeat(4096))).is_ok());
    assert!(owner::source_request(&request(&"a".repeat(4097))).is_err());
    assert!(owner::source_request(&request(&"é".repeat(2048))).is_ok());
    assert!(owner::source_request(&request(&"é".repeat(2049))).is_err());
    let mut exact = request("");
    exact.resize(32768, b' ');
    assert!(owner::source_request(&exact).is_ok());
    exact.push(b' ');
    assert!(owner::source_request(&exact).is_err());
    for input in [
        br#"{"version":1,"revision":0,"source":""}"#.as_slice(),
        br#"{"version":1,"revision":9007199254740992,"source":""}"#,
        br#"{"version":1,"revision":1,"revision":2,"source":""}"#,
        br#"{"version":1,"revision":1,"source":"","path":"/host/secret"}"#,
        br#"{"version":1,"revision":1.0,"source":""}"#,
        br#"{"version":1,"revision":1e0,"source":""}"#,
        br#"{"version":1,"revision":1,"source":"\ud800"}"#,
        b"\xff",
    ] {
        assert!(owner::source_request(input).is_err(), "{input:?}");
    }
}

#[test]
fn fixed_role_budgets_and_write_authority_cannot_be_selected_by_source() {
    let approved = approved();
    for (op, memory, pids) in
        [(Operation::Compile, 536_870_912, 32), (Operation::Evaluate, 1_073_741_824, 256)]
    {
        let value = config::container(op, false, &approved, "image", "owner", ["in", "out"]);
        let host = &value["HostConfig"];
        assert_eq!(host["Memory"], memory);
        assert_eq!(host["MemorySwap"], memory);
        assert_eq!(host["PidsLimit"], pids);
        assert_eq!(host["CpuQuota"], 200_000);
        assert_eq!(host["CpusetCpus"], "0,1");
        assert_eq!(host["NetworkMode"], "none");
        assert_eq!(host["Mounts"][0]["ReadOnly"], true);
        assert_eq!(host["Mounts"][1]["ReadOnly"], false);
        assert_eq!(host["Mounts"][2]["ReadOnly"], true);
        assert_eq!(host["Mounts"][2]["Target"], "/materials");
        assert_eq!(host["Mounts"].as_array().expect("mounts").len(), 3);
        assert_eq!(value["Cmd"], json!(["idle"]));
        assert_eq!(value["Env"], json!(config::ENV));
    }
    let holder =
        config::container(Operation::Compile, true, &approved, "image", "owner", ["in", "out"]);
    assert_eq!(holder["HostConfig"]["Memory"], 16_777_216);
    assert_eq!(holder["HostConfig"]["PidsLimit"], 2);
    assert_eq!(holder["HostConfig"]["Mounts"][0]["ReadOnly"], false);
    assert_eq!(holder["HostConfig"]["Mounts"][1]["ReadOnly"], true);
    assert_eq!(holder["HostConfig"]["Mounts"].as_array().expect("mounts").len(), 2);
}

#[test]
fn staged_or_cancelled_attempt_cannot_skip_quiescence_collection_or_owned_removal() {
    use state::{Phase, State};
    let mut state = State { owner: "fresh-owner".into(), phase: Phase::Preparing };
    let cancel = AtomicBool::new(false);
    for phase in [
        Phase::Member,
        Phase::Audited,
        Phase::InputSealed,
        Phase::Dispatched,
        Phase::Quiescent,
        Phase::Collected,
    ] {
        assert!(state.success(&cancel).is_err());
        assert!(state.advance(state.phase, Phase::Removed).is_err());
        state.advance(state.phase, phase).expect("ordered ownership");
    }
    assert!(state.success(&cancel).is_err());
    state.advance(Phase::Collected, Phase::Removed).expect("owned cleanup");
    assert!(state.success(&cancel).is_ok());
    let primary = reject("PRIMARY", "retained actual failure");
    let cleanup = reject("CLEANUP", "retained cleanup failure");
    let error = state.cleanup_failure(Some(primary.clone()), cleanup.clone());
    assert_eq!(error.primary(), Some(&primary));
    assert_eq!(error.cleanup(), Some(&cleanup));
    assert_eq!(error.owner(), Some("fresh-owner"));
    assert!(state.success(&cancel).is_err());
}

#[test]
fn cancellation_during_teardown_prevents_old_success_and_cannot_mask_failed_cleanup() {
    let mut state = state::State { owner: "attempt".into(), phase: state::Phase::Collected };
    let cancel = AtomicBool::new(false);
    assert_eq!(
        state.success(&cancel).expect_err("teardown pending").reason(),
        "PLAYGROUND-HOST-TEARDOWN"
    );
    cancel.store(true, Ordering::Release);
    state
        .advance(state::Phase::Collected, state::Phase::Removed)
        .expect("actual owned cleanup settled");
    assert_eq!(
        state.success(&cancel).expect_err("late cancellation wins before publication").reason(),
        "PLAYGROUND-CANCELLED"
    );
    let cleanup = state.cleanup_failure(None, reject("CLEANUP", "actual cleanup refusal"));
    assert_eq!(cleanup.reason(), "PLAYGROUND-HOST-TEARDOWN");
    assert_eq!(
        state.success(&cancel).expect_err("cleanup failure retains precedence").reason(),
        "PLAYGROUND-HOST-TEARDOWN"
    );
}

fn actual(request: &Value, cid: &str) -> Value {
    let mut config = request.clone();
    config.as_object_mut().expect("config").remove("HostConfig");
    config["Hostname"] = json!(&cid[..12]);
    let mut value = json!({"Id":cid,"Name":"/role","Image":request["Image"],"Config":config,
        "HostConfig":request["HostConfig"],"Mounts":[
            {"Type":"volume","Name":"in","Source":"/daemon/in","Destination":"/input","Driver":"local","Mode":"z","RW":false,"Propagation":""},
            {"Type":"volume","Name":"out","Source":"/daemon/out","Destination":"/output","Driver":"local","Mode":"z","RW":true,"Propagation":""}]});
    value["Mounts"].as_array_mut().expect("mounts").push(json!({"Type":"volume",
        "Name":request["HostConfig"]["Mounts"][2]["Source"],"Source":"/daemon/materials",
        "Destination":"/materials","Driver":"local","Mode":"z","RW":false,"Propagation":""}));
    value
}

fn material(evaluate: bool) -> PlaygroundDaemonMaterialPin {
    PlaygroundDaemonMaterialPin {
        name: format!("zryna410-materials-{}", if evaluate { "evaluator" } else { "compiler" }),
        created_at: "2026-10-15T00:00:00Z".into(),
        nonce: if evaluate { "6" } else { "5" }.repeat(32),
        root_sha256: if evaluate { "d" } else { "c" }.repeat(64),
        files: 2,
        bytes: 1024,
    }
}
