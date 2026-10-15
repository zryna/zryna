//! Fixed two-role local Docker policy. These values are requests, not enforcement proof.
use crate::package_build::ExecutionPolicyIdentity;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Holder,
    Recipe,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Volume {
    Input,
    Output,
    Control,
}
pub(super) const ENV: [&str; 8] = [
    "PATH=/tool/bin:/tool/libexec",
    "LC_ALL=C",
    "LANG=C",
    "LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/lib64",
    "TMPDIR=/tmp",
    "HOME=/nonexistent",
    "TZ=UTC",
    "SOURCE_DATE_EPOCH=0",
];
pub(super) fn volumeopts(volume: Volume) -> Value {
    let options = match volume {
        Volume::Input => "rw,nosuid,nodev,noexec,size=2097152,mode=0555,uid=0,gid=0",
        Volume::Output => "rw,nosuid,nodev,noexec,size=8388608,mode=0700,uid=65534,gid=65534",
        Volume::Control => "rw,nosuid,nodev,noexec,size=4096,mode=0555,uid=0,gid=0",
    };
    json!({"type":"tmpfs","device":"tmpfs","o":options})
}
fn seccomp() -> Value {
    serde_json::from_str(include_str!("seccomp.json")).expect("embedded fixed seccomp JSON")
}
pub(super) fn config(role: Role, image: &str, owner: &str, volumes: [&str; 3]) -> Value {
    let holder = role == Role::Holder;
    let mounts = [
        json!({"Type":"volume","Source":volumes[0],"Target":"/input",
            "ReadOnly":!holder,"VolumeOptions":{"NoCopy":true}}),
        json!({"Type":"volume","Source":volumes[1],"Target":"/output",
            "ReadOnly":holder,"VolumeOptions":{"NoCopy":true}}),
        json!({"Type":"volume","Source":volumes[2],"Target":"/control",
            "ReadOnly":true,"VolumeOptions":{"NoCopy":true}}),
    ];
    let tmpfs = json!({
        "/tmp":"rw,nosuid,nodev,noexec,size=16777216,mode=1777,uid=0,gid=0",
        "/dev":"rw,nosuid,noexec,dev,size=1048576,mode=0755,uid=0,gid=0",
        "/dev/shm":"rw,nosuid,nodev,noexec,size=1048576,mode=1777,uid=0,gid=0"});
    let host = json!({
            "ReadonlyRootfs":true,"NetworkMode":"none","CapDrop":["ALL"],
            "Privileged":false,"Init":false,"IpcMode":"none","CgroupnsMode":"private",
            "Runtime":"runc","PidMode":"","UTSMode":"","UsernsMode":"",
            "OomKillDisable":false,"ShmSize":67_108_864,
            "PidsLimit":if holder {2} else {16},
            "Memory":if holder {16_777_216} else {100_663_296},
            "MemorySwap":if holder {16_777_216} else {100_663_296},
            "CpuPeriod":100_000,"CpuQuota":if holder {10_000} else {50_000},
            "Tmpfs":tmpfs,
            "LogConfig":{"Type":"none","Config":{}},"AutoRemove":false,
            "RestartPolicy":{"Name":"no","MaximumRetryCount":0},
            "SecurityOpt":["no-new-privileges:true",format!("seccomp={}",seccomp())],
            "MaskedPaths":["/proc/acpi","/proc/asound","/proc/interrupts","/proc/kcore",
                "/proc/keys","/proc/latency_stats","/proc/timer_list","/proc/timer_stats",
                "/proc/sched_debug","/proc/scsi","/sys/firmware","/sys/devices/virtual/powercap",
                "/etc/hostname","/etc/hosts","/etc/resolv.conf"],
            "ReadonlyPaths":["/proc"],"Mounts":mounts});
    json!({"Image":image,"User":"65534:65534","Entrypoint":["/supervisor"],"Cmd":["idle"],
        "WorkingDir":"/tmp","Env":ENV,"Tty":false,"OpenStdin":false,
        "AttachStdin":false,"AttachStdout":false,"AttachStderr":false,"StdinOnce":false,
        "Labels":{"zryna405.owner":owner},"HostConfig":host})
}
/// Reproducible policy identity excludes run-specific image, owner and volume names.
pub(super) fn policy_identity() -> ExecutionPolicyIdentity {
    let mut policy = json!({"id":"zryna.native-local-engine-two-role","version":"0",
        "holder":config(Role::Holder,"","",["input","output","control"]),
        "recipe":config(Role::Recipe,"","",["input","output","control"]),
        "volumes":[volumeopts(Volume::Input),volumeopts(Volume::Output),volumeopts(Volume::Control)],
        "seccomp":seccomp(),"jobSeconds":30,"cleanupSeconds":5,
        "aggregate":{"memoryBytes":117_440_512,"pids":18,"cpuQuota":60_000,"cpuPeriod":100_000}});
    policy.sort_all_objects();
    let mut canonical = serde_json::to_vec(&policy).expect("fixed JSON policy serialization");
    canonical.push(b'\n');
    ExecutionPolicyIdentity {
        id: "zryna.native-local-engine-two-role".into(),
        version: "0".into(),
        configuration_sha256: format!("{:x}", Sha256::digest(canonical)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_separate_write_authority_and_account_both_budgets() {
        let holder = config(Role::Holder, "image", "owner", ["in", "out", "ctl"]);
        let recipe = config(Role::Recipe, "image", "owner", ["in", "out", "ctl"]);
        assert_eq!(holder["Env"], json!(ENV));
        assert_eq!(holder["HostConfig"]["Mounts"][0]["ReadOnly"], false);
        assert_eq!(recipe["HostConfig"]["Mounts"][0]["ReadOnly"], true);
        assert_eq!(holder["HostConfig"]["Mounts"][1]["ReadOnly"], true);
        assert_eq!(recipe["HostConfig"]["Mounts"][1]["ReadOnly"], false);
        for state in [&holder, &recipe] {
            assert_eq!(state["HostConfig"]["Mounts"][2]["ReadOnly"], true);
            assert_eq!(state["HostConfig"]["Mounts"][2]["VolumeOptions"]["NoCopy"], true);
            assert_eq!(
                &state["HostConfig"]["MaskedPaths"].as_array().expect("mask array")[12..],
                &[json!("/etc/hostname"), json!("/etc/hosts"), json!("/etc/resolv.conf")]
            );
        }
        assert_eq!(
            holder["HostConfig"]["Memory"].as_u64().expect("fixture")
                + recipe["HostConfig"]["Memory"].as_u64().expect("fixture"),
            117_440_512
        );
        assert_eq!(
            holder["HostConfig"]["PidsLimit"].as_u64().expect("fixture")
                + recipe["HostConfig"]["PidsLimit"].as_u64().expect("fixture"),
            18
        );
        assert_eq!(
            policy_identity().configuration_sha256,
            "b6d7e06c41f7b3620a7083ffdbc01a85c5b3820a31017fc86466ba089e40c467"
        );
    }

    #[test]
    fn seccomp_is_default_deny_with_only_fixed_clone_and_memory_rules() {
        let policy = seccomp();
        assert_eq!(policy["defaultAction"], "SCMP_ACT_ERRNO");
        assert_eq!(policy["defaultErrnoRet"], 1);
        let rules = policy["syscalls"].as_array().expect("fixture");
        for name in ["execveat", "memfd_create", "socket", "setns", "unshare", "mount", "ptrace"] {
            assert!(!rules.iter().any(|r| r["action"] == "SCMP_ACT_ALLOW"
                && r["names"].as_array().expect("fixture").contains(&json!(name))));
        }
        assert!(rules.iter().any(|r| r["action"] == "SCMP_ACT_ALLOW"
            && r["names"].as_array().expect("fixture").contains(&json!("execve"))));
        let clones = rules
            .iter()
            .filter(|r| r["names"] == json!(["clone"]))
            .map(|r| r["args"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            clones,
            vec![
                json!([{"index":0,"op":"SCMP_CMP_EQ","value":17}]),
                json!([{"index":0,"op":"SCMP_CMP_EQ","value":16_657}])
            ]
        );
    }
}
