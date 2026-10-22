//! Exact owned Engine identities. Successful inspection alone is not kernel enforcement.
use super::config::{self, Role};
use serde_json::{Value, json};

fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
fn empty(value: &Value) -> bool {
    value.is_null()
        || value.as_array().is_some_and(Vec::is_empty)
        || value.as_object().is_some_and(serde_json::Map::is_empty)
        || value.as_str() == Some("")
}
fn mounts(actual: &Value, expected: &Value) -> Result<(), &'static str> {
    let actual = actual.as_array().ok_or("mount array absent")?;
    let expected = expected.as_array().ok_or("fixed mount array absent")?;
    if actual.len() != expected.len() {
        return Err("mount count differs");
    }
    for (actual, expected) in actual.iter().zip(expected) {
        let mut fixed = expected.clone();
        // Docker Go omitempty elides false only. Required true must remain explicit.
        if expected["ReadOnly"] == false && actual.get("ReadOnly").is_none() {
            fixed.as_object_mut().ok_or("fixed mount shape differs")?.remove("ReadOnly");
        }
        if actual != &fixed {
            return Err("exact mount role differs");
        }
    }
    Ok(())
}
fn additional_host_fields(host: &Value, requested: &Value) -> Result<(), &'static str> {
    let blank = "Binds ContainerIDFile VolumeDriver VolumesFrom CapAdd Dns DnsOptions DnsSearch \
        ExtraHosts GroupAdd Cgroup Links PidMode UTSMode UsernsMode Isolation CgroupParent \
        CpusetCpus CpusetMems Devices DeviceRequests DeviceCgroupRules Sysctls Ulimits StorageOpt \
        Annotations BlkioWeightDevice BlkioDeviceReadBps BlkioDeviceWriteBps BlkioDeviceReadIOps \
        BlkioDeviceWriteIOps PortBindings";
    let zero = "OomScoreAdj CpuShares NanoCpus BlkioWeight CpuRealtimePeriod CpuRealtimeRuntime \
        MemoryReservation CpuCount CpuPercent IOMaximumIOps IOMaximumBandwidth";
    for (key, value) in host.as_object().ok_or("host config absent")? {
        if requested.get(key).is_some() {
            continue;
        }
        let permitted = if blank.split_ascii_whitespace().any(|field| field == key) {
            empty(value)
        } else if zero.split_ascii_whitespace().any(|field| field == key) {
            value == &json!(0)
        } else {
            match key.as_str() {
                "ConsoleSize" => value == &json!([0, 0]),
                "PublishAllPorts" => value == false,
                "MemorySwappiness" => value.is_null(),
                _ => false,
            }
        };
        if !permitted {
            return Err("unknown or additional host authority");
        }
    }
    Ok(())
}
fn fixed_request(requested: &Value, image: &str, owner: &str) -> Result<(), &'static str> {
    let role = match requested["HostConfig"]["Memory"].as_u64() {
        Some(16_777_216) => Role::Holder,
        Some(100_663_296) => Role::Recipe,
        _ => return Err("role budget differs"),
    };
    let entries = requested["HostConfig"]["Mounts"].as_array().ok_or("mount request absent")?;
    if entries.len() != 3 {
        return Err("exactly three named volumes required");
    }
    let a = entries[0]["Source"].as_str().ok_or("input volume absent")?;
    let b = entries[1]["Source"].as_str().ok_or("output volume absent")?;
    let c = entries[2]["Source"].as_str().ok_or("control volume absent")?;
    if ![a, b, c].iter().all(|s| name(s)) || a == b || a == c || b == c {
        return Err("distinct fixed named volumes required");
    }
    if requested != &config::config(role, image, owner, [a, b, c]) {
        return Err("request is not the fixed role policy");
    }
    Ok(())
}
fn mounted_volumes(state: &Value, requested: &Value) -> Result<(), &'static str> {
    let actual = state["Mounts"].as_array().ok_or("effective mounts absent")?;
    let expected = requested["HostConfig"]["Mounts"].as_array().ok_or("fixed mounts absent")?;
    if actual.len() != 3 {
        return Err("effective mount count differs");
    }
    for fixed in expected {
        let observed =
            actual.iter().filter(|m| m["Destination"] == fixed["Target"]).collect::<Vec<_>>();
        if observed.len() != 1 {
            return Err("effective mount destination differs");
        }
        let observed = observed[0];
        if observed.as_object().ok_or("effective mount shape differs")?.keys().any(|key| {
            !["Type", "Name", "Source", "Destination", "Driver", "Mode", "RW", "Propagation"]
                .contains(&key.as_str())
        }) {
            return Err("unknown effective mount authority");
        }
        if observed["Type"] != "volume"
            || observed["Name"] != fixed["Source"]
            || observed["Driver"] != "local"
            || observed["RW"].as_bool() != Some(fixed["ReadOnly"] == false)
            // Docker's shared SELinux label marker; RW is checked separately.
            || observed["Mode"] != "z"
            || observed["Propagation"] != ""
        {
            return Err("effective volume authority differs");
        }
    }
    Ok(())
}
pub(super) fn validate(
    state: &Value,
    cid: &str,
    requested_name: &str,
    owner: &str,
    image: &str,
    requested: &Value,
) -> Result<(), &'static str> {
    if !hex(cid, 64)
        || !hex(owner, 32)
        || !name(requested_name)
        || !image.strip_prefix("sha256:").is_some_and(|s| hex(s, 64))
    {
        return Err("full immutable owned identity required");
    }
    fixed_request(requested, image, owner)?;
    if state["Id"] != cid
        || state["Name"] != format!("/{requested_name}")
        || state["Image"] != image
    {
        return Err("owned container identity differs");
    }
    let cfg = state.get("Config").ok_or("container config absent")?;
    let host = state.get("HostConfig").ok_or("host config absent")?;
    for (key, expected) in requested.as_object().ok_or("fixed request absent")? {
        if key != "HostConfig" && cfg.get(key) != Some(expected) {
            return Err("fixed container config differs");
        }
    }
    for (key, value) in cfg.as_object().ok_or("container config shape differs")? {
        if key != "HostConfig" && requested.get(key).is_some() {
            continue;
        }
        let permitted = match key.as_str() {
            "Hostname" => value.as_str() == Some(&cid[..12]),
            "Domainname" | "MacAddress" | "Volumes" | "Healthcheck" | "OnBuild" | "Shell"
            | "StopSignal" => empty(value),
            "ArgsEscaped" | "NetworkDisabled" => value == false,
            "StopTimeout" => value.is_null(),
            _ => false,
        };
        if !permitted {
            return Err("unknown or inherited container authority");
        }
    }
    for (key, expected) in requested["HostConfig"].as_object().ok_or("fixed host config absent")? {
        if key == "Mounts" {
            mounts(&host[key], expected)?;
        } else if host.get(key) != Some(expected)
            && !(key == "OomKillDisable"
                && expected == false
                && host.get(key) == Some(&Value::Null))
        {
            return Err("fixed host setting differs");
        }
    }
    additional_host_fields(host, &requested["HostConfig"])?;
    mounted_volumes(state, requested)?;
    Ok(())
}
pub(super) fn validate_volume(
    state: &Value,
    requested_name: &str,
    owner: &str,
    options: &Value,
) -> Result<(), &'static str> {
    if !name(requested_name)
        || !hex(owner, 32)
        || state["Name"] != requested_name
        || state["Driver"] != "local"
        || state["Options"] != *options
        || state["Labels"] != json!({"zryna405.owner":owner})
        || state["Scope"] != "local"
    {
        return Err("exact owned local volume differs");
    }
    if ![config::Volume::Input, config::Volume::Output, config::Volume::Control]
        .iter()
        .any(|v| config::volumeopts(*v) == *options)
    {
        return Err("unknown volume policy");
    }
    for (key, value) in state.as_object().ok_or("volume shape differs")? {
        if !"Name Driver Options Labels Scope CreatedAt Mountpoint Status UsageData"
            .split_ascii_whitespace()
            .any(|field| field == key)
        {
            return Err("unknown volume authority");
        }
        if key == "Status" && !empty(value) {
            return Err("volume plugin status differs");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: &str = "11111111111111111111111111111111";
    fn sample(role: Role) -> (Value, Value, String, String) {
        let cid = "a".repeat(64);
        let image = format!("sha256:{}", "b".repeat(64));
        let request = config::config(role, &image, OWNER, ["input", "output", "control"]);
        let mut cfg = request.clone();
        cfg.as_object_mut().expect("fixture").remove("HostConfig");
        let state = json!({"Id":cid,"Name":"/owned","Image":image,
            "Config":cfg,"HostConfig":request["HostConfig"],"Mounts":
            request["HostConfig"]["Mounts"].as_array().expect("fixture").iter().map(|m|
                json!({"Type":"volume","Name":m["Source"],"Driver":"local",
                    "Destination":m["Target"],"RW":m["ReadOnly"]==false,
                    "Mode":"z","Propagation":""})).collect::<Vec<_>>()});
        (state, request, cid, image)
    }
    #[test]
    fn readonly_false_is_the_only_mount_omitempty_exception() {
        let (mut state, request, cid, image) = sample(Role::Holder);
        assert_eq!(validate(&state, &cid, "owned", OWNER, &image, &request), Ok(()));
        state["HostConfig"]["Mounts"][0].as_object_mut().expect("fixture").remove("ReadOnly");
        assert_eq!(validate(&state, &cid, "owned", OWNER, &image, &request), Ok(()));
        for (index, value) in [(1, Value::Null), (2, json!(false)), (0, json!(0))] {
            let mut mutant = state.clone();
            mutant["HostConfig"]["Mounts"][index]["ReadOnly"] = value;
            assert!(validate(&mutant, &cid, "owned", OWNER, &image, &request).is_err());
        }
        state["HostConfig"]["Mounts"][1].as_object_mut().expect("fixture").remove("ReadOnly");
        assert!(validate(&state, &cid, "owned", OWNER, &image, &request).is_err());
    }
    #[test]
    fn authority_substitution_and_forged_requested_policy_are_rejected() {
        let (mut state, request, cid, image) = sample(Role::Recipe);
        for (field, value) in [
            ("Privileged", json!(true)),
            ("CapAdd", json!(["SYS_ADMIN"])),
            ("Memory", json!(201_326_592)),
            ("UnknownAuthority", json!(false)),
            ("OomKillDisable", json!(true)),
        ] {
            let mut mutant = state.clone();
            mutant["HostConfig"][field] = value;
            assert!(validate(&mutant, &cid, "owned", OWNER, &image, &request).is_err());
        }
        let mut forged = request.clone();
        forged["HostConfig"]["Privileged"] = json!(true);
        assert!(validate(&state, &cid, "owned", OWNER, &image, &forged).is_err());
        let mut mutant = state.clone();
        mutant["Config"]["Env"] = json!(["TOKEN=secret"]);
        assert!(validate(&mutant, &cid, "owned", OWNER, &image, &request).is_err());
        assert!(validate(&state, &"c".repeat(64), "owned", OWNER, &image, &request).is_err());
        state["HostConfig"]["OomKillDisable"] = Value::Null;
        assert_eq!(validate(&state, &cid, "owned", OWNER, &image, &request), Ok(()));
    }
    #[test]
    fn effective_mount_and_owned_tmpfs_substitution_are_rejected() {
        let (mut state, request, cid, image) = sample(Role::Recipe);
        for mode in ["", "Z", "rw", "z,rw"] {
            let mut mutant = state.clone();
            mutant["Mounts"][0]["Mode"] = json!(mode);
            assert!(validate(&mutant, &cid, "owned", OWNER, &image, &request).is_err());
        }
        state["Mounts"][0]["RW"] = json!(true);
        assert!(validate(&state, &cid, "owned", OWNER, &image, &request).is_err());
        let options = config::volumeopts(config::Volume::Control);
        let volume = json!({"Name":"control","Driver":"local","Options":options,
            "Labels":{"zryna405.owner":OWNER},"Scope":"local"});
        assert_eq!(validate_volume(&volume, "control", OWNER, &options), Ok(()));
        for (field, value) in [
            ("Driver", json!("plugin")),
            ("Labels", json!({})),
            ("Options", json!({"type":"tmpfs","device":"tmpfs","o":"rw,exec"})),
        ] {
            let mut mutant = volume.clone();
            mutant[field] = value;
            assert!(validate_volume(&mutant, "control", OWNER, &options).is_err());
        }
        assert_eq!(validate_volume(&volume, "control", OWNER, &options), Ok(()));
    }
}
