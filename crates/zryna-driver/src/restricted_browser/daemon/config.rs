use super::{Operation, approval::Approved};
use serde_json::{Value, json};

pub(super) const ENV: [&str; 7] = [
    "PATH=/materials/runtime/node/bin",
    "LC_ALL=C.UTF-8",
    "LANG=C.UTF-8",
    "TMPDIR=/tmp",
    "HOME=/nonexistent",
    "TZ=UTC",
    "SOURCE_DATE_EPOCH=0",
];

pub(super) fn volume(output: bool) -> Value {
    let options = if output {
        "rw,nosuid,nodev,noexec,size=2097152,mode=0700,uid=65534,gid=65534"
    } else {
        "rw,nosuid,nodev,noexec,size=2097152,mode=0555,uid=0,gid=0"
    };
    json!({"type":"tmpfs","device":"tmpfs","o":options})
}

pub(super) fn container(
    operation: Operation,
    holder: bool,
    approval: &Approved,
    image: &str,
    owner: &str,
    volumes: [&str; 2],
) -> Value {
    let compile = operation == Operation::Compile;
    let memory = if holder {
        16_777_216
    } else if compile {
        536_870_912
    } else {
        1_073_741_824
    };
    let tasks = if holder {
        2
    } else if compile {
        32
    } else {
        256
    };
    let seccomp =
        if holder || compile { &approval.compiler_seccomp } else { &approval.evaluator_seccomp };
    let cpus = format!("{},{}", approval.record.cpus[0], approval.record.cpus[1]);
    let mut mounts = vec![
        json!({"Type":"volume","Source":volumes[0],"Target":"/input",
        "ReadOnly":!holder,"VolumeOptions":{"NoCopy":true}}),
        json!({"Type":"volume","Source":volumes[1],"Target":"/output",
        "ReadOnly":holder,"VolumeOptions":{"NoCopy":true}}),
    ];
    if !holder {
        let material = if compile {
            &approval.record.compiler_materials
        } else {
            &approval.record.evaluator_materials
        };
        mounts.push(json!({"Type":"volume","Source":material.name,"Target":"/materials",
            "ReadOnly":true,"VolumeOptions":{"NoCopy":true}}));
    }
    let host = json!({"ReadonlyRootfs":true,"NetworkMode":"none","CapDrop":["ALL"],
        "Privileged":false,"Init":false,"IpcMode":"none","CgroupnsMode":"private",
        "Runtime":"runc","PidMode":"","UTSMode":"","UsernsMode":"",
        "OomKillDisable":false,"ShmSize":67_108_864,"PidsLimit":tasks,
        "Memory":memory,"MemorySwap":memory,"CpuPeriod":100_000,
        "CpuQuota":if holder {10_000} else {200_000},"CpusetCpus":cpus,
        "Tmpfs":{
            "/tmp":if holder {"rw,nosuid,nodev,noexec,size=1048576,mode=1777"}
                else if compile {"rw,nosuid,nodev,noexec,size=67108864,mode=1777"}
                else {"rw,nosuid,nodev,noexec,size=134217728,mode=1777"},
            "/dev":"rw,nosuid,noexec,dev,size=1048576,mode=0755",
            "/dev/shm":if compile || holder {"rw,nosuid,nodev,noexec,size=1048576,mode=1777"}
                else {"rw,nosuid,nodev,noexec,size=67108864,mode=1777"}},
        "Ulimits":[{"Name":"core","Soft":0,"Hard":0},
            {"Name":"fsize","Soft":268_435_456,"Hard":268_435_456},
            {"Name":"nofile","Soft":512,"Hard":512}],
        "LogConfig":{"Type":"none","Config":{}},"AutoRemove":false,
        "RestartPolicy":{"Name":"no","MaximumRetryCount":0},
        "SecurityOpt":["no-new-privileges:true",format!("seccomp={seccomp}")],
        "MaskedPaths":["/proc/acpi","/proc/asound","/proc/interrupts","/proc/kcore",
            "/proc/keys","/proc/latency_stats","/proc/timer_list","/proc/timer_stats",
            "/proc/sched_debug","/proc/scsi","/sys/firmware","/sys/devices/virtual/powercap",
            "/etc/hostname","/etc/hosts","/etc/resolv.conf"],
        "ReadonlyPaths":if holder || compile {json!(["/proc"])} else {
            // Chromium maps only its own new user namespace; kernel administration stays read-only.
            json!(["/proc/bus","/proc/fs","/proc/irq","/proc/sys","/proc/sysrq-trigger"])
        },
        "Mounts":mounts});
    json!({"Image":image,"User":"65534:65534","Entrypoint":["/supervisor"],"Cmd":["idle"],
        "WorkingDir":"/tmp","Env":ENV,"Tty":false,"OpenStdin":false,
        "AttachStdin":false,"AttachStdout":false,"AttachStderr":false,"StdinOnce":false,
        "Labels":{"zryna410.owner":owner},"HostConfig":host})
}
