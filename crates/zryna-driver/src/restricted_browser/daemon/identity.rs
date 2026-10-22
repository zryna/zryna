use super::{Result, approval::hash, reject};
use serde_json::{Value, json};

fn empty(value: &Value) -> bool {
    value.is_null()
        || value.as_array().is_some_and(Vec::is_empty)
        || value.as_object().is_some_and(serde_json::Map::is_empty)
        || value == ""
}

pub(super) fn validate(
    state: &Value,
    cid: &str,
    name: &str,
    image: &str,
    requested: &Value,
) -> Result<()> {
    let denied =
        || reject("PLAYGROUND-HOST-ENFORCEMENT", "actual owned role configuration differs");
    if !hash(cid)
        || state["Id"] != cid
        || state["Name"] != format!("/{name}")
        || state["Image"] != image
    {
        return Err(denied());
    }
    let config = state["Config"].as_object().ok_or_else(denied)?;
    let host = state["HostConfig"].as_object().ok_or_else(denied)?;
    for (key, expected) in requested.as_object().ok_or_else(denied)? {
        if key != "HostConfig" && config.get(key) != Some(expected) {
            return Err(denied());
        }
    }
    for (key, value) in config {
        if requested.get(key).is_some() {
            continue;
        }
        let allowed = match key.as_str() {
            "Hostname" => value == &cid[..12],
            "Domainname" | "MacAddress" | "Volumes" | "Healthcheck" | "OnBuild" | "Shell"
            | "StopSignal" | "ExposedPorts" => empty(value),
            "ArgsEscaped" | "NetworkDisabled" => value == false,
            "StopTimeout" => value.is_null(),
            _ => false,
        };
        if !allowed {
            return Err(denied());
        }
    }
    for (key, expected) in requested["HostConfig"].as_object().ok_or_else(denied)? {
        if key == "Mounts" {
            let actual = host[key].as_array().ok_or_else(denied)?;
            let expected = expected.as_array().ok_or_else(denied)?;
            if actual.len() != expected.len() {
                return Err(denied());
            }
            for (actual, expected) in actual.iter().zip(expected) {
                let mut fixed = expected.clone();
                if expected["ReadOnly"] == false && actual.get("ReadOnly").is_none() {
                    fixed.as_object_mut().ok_or_else(denied)?.remove("ReadOnly");
                }
                if actual != &fixed {
                    return Err(denied());
                }
            }
        } else if host.get(key) != Some(expected)
            && !(key == "OomKillDisable"
                && expected == false
                && host.get(key) == Some(&Value::Null))
        {
            return Err(denied());
        }
    }
    extra_host(&state["HostConfig"], &requested["HostConfig"])?;
    let mounts = state["Mounts"].as_array().ok_or_else(denied)?;
    let expected_mounts = requested["HostConfig"]["Mounts"].as_array().ok_or_else(denied)?;
    if mounts.len() != expected_mounts.len() {
        return Err(denied());
    }
    for expected in expected_mounts {
        let matches = mounts
            .iter()
            .filter(|mount| mount["Destination"] == expected["Target"])
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(denied());
        }
        let mount = matches[0];
        if mount.as_object().is_none_or(|map| {
            map.keys().any(|key| {
                !["Type", "Name", "Source", "Destination", "Driver", "Mode", "RW", "Propagation"]
                    .contains(&key.as_str())
            })
        }) || mount["Type"] != "volume"
            || mount["Name"] != expected["Source"]
            || mount["Driver"] != "local"
            || mount["RW"].as_bool() != Some(expected["ReadOnly"] == false)
            || mount["Mode"] != "z"
            || mount["Propagation"] != ""
        {
            return Err(denied());
        }
    }
    Ok(())
}

fn extra_host(host: &Value, requested: &Value) -> Result<()> {
    let blank = "Binds ContainerIDFile VolumeDriver VolumesFrom CapAdd Dns DnsOptions DnsSearch \
        ExtraHosts GroupAdd Cgroup Links Isolation CgroupParent CpusetMems Devices DeviceRequests \
        DeviceCgroupRules Sysctls StorageOpt Annotations BlkioWeightDevice BlkioDeviceReadBps \
        BlkioDeviceWriteBps BlkioDeviceReadIOps BlkioDeviceWriteIOps PortBindings";
    let zero = "OomScoreAdj CpuShares NanoCpus BlkioWeight CpuRealtimePeriod CpuRealtimeRuntime \
        MemoryReservation CpuCount CpuPercent IOMaximumIOps IOMaximumBandwidth";
    for (key, value) in host
        .as_object()
        .ok_or_else(|| reject("PLAYGROUND-HOST-ENFORCEMENT", "host settings absent"))?
    {
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
            return Err(reject("PLAYGROUND-HOST-ENFORCEMENT", "additional host authority refused"));
        }
    }
    Ok(())
}

pub(super) fn volume(state: &Value, name: &str, owner: &str, options: &Value) -> Result<()> {
    if state["Name"] != name
        || state["Driver"] != "local"
        || state["Options"] != *options
        || state["Labels"] != json!({"zryna410.owner":owner})
        || state["Scope"] != "local"
    {
        return Err(reject("PLAYGROUND-HOST-ENFORCEMENT", "owned bounded tmpfs volume differs"));
    }
    Ok(())
}
