use super::{Result, reject};
use serde_json::Value;
use std::{collections::BTreeSet, fs::File, io::Read, time::Instant};

mod ownership;
mod proc_scan;

pub(super) use ownership::{capture_owned, cleanup_owned};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Init {
    pub pid: u32,
    pub ticks: String,
    started: String,
    cid: String,
    membership: String,
}

fn read(pid: u32, name: &str, deadline: Instant) -> Result<String> {
    if Instant::now() >= deadline {
        return Err(reject("PLAYGROUND-HOST-DEADLINE", "kernel observation deadline exceeded"));
    }
    let mut bytes = Vec::new();
    File::open(format!("/proc/{pid}/{name}"))
        .and_then(|file| file.take(16_385).read_to_end(&mut bytes))
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "actual kernel identity unavailable"))?;
    if bytes.len() > 16_384 || Instant::now() >= deadline {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "kernel observation outside finite budget"));
    }
    String::from_utf8(bytes)
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "invalid kernel observation"))
}

fn ticks(pid: u32, deadline: Instant) -> Result<String> {
    read(pid, "stat", deadline)?
        .rsplit_once(')')
        .and_then(|(_, fields)| fields.split_whitespace().nth(19))
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "kernel init start identity unavailable"))
}

pub(super) fn security(status: &str, cpus: [u16; 2]) -> Result<()> {
    for (key, expected) in [
        ("NoNewPrivs:", "1"),
        ("Seccomp:", "2"),
        ("Threads:", "1"),
        ("CapInh:", "0000000000000000"),
        ("CapPrm:", "0000000000000000"),
        ("CapEff:", "0000000000000000"),
        ("CapBnd:", "0000000000000000"),
        ("CapAmb:", "0000000000000000"),
    ] {
        let mut values = status.lines().filter_map(|line| line.strip_prefix(key));
        if values.next().map(str::trim) != Some(expected) || values.next().is_some() {
            return Err(reject(
                "PLAYGROUND-HOST-KERNEL",
                "actual NNP, seccomp or zero capabilities unproved",
            ));
        }
    }
    for key in ["Uid:", "Gid:"] {
        let mut values = status.lines().filter_map(|line| line.strip_prefix(key));
        if values.next().map(|value| value.split_whitespace().collect::<Vec<_>>())
            != Some(vec!["65534"; 4])
            || values.next().is_some()
        {
            return Err(reject(
                "PLAYGROUND-HOST-KERNEL",
                "actual fixed nonroot credentials unproved",
            ));
        }
    }
    let mut values = status.lines().filter_map(|line| line.strip_prefix("Cpus_allowed_list:"));
    let actual = values
        .next()
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "effective CPU set absent"))?;
    if values.next().is_some() || cpu_set(actual.trim())? != BTreeSet::from(cpus) {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "effective two-CPU set differs"));
    }
    Ok(())
}

fn cpu_set(text: &str) -> Result<BTreeSet<u16>> {
    let mut output = BTreeSet::new();
    for part in text.split(',') {
        let parse = |value: &str| {
            value
                .parse::<u16>()
                .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "invalid effective CPU set"))
        };
        let (first, last) = if let Some((first, last)) = part.split_once('-') {
            (parse(first)?, parse(last)?)
        } else {
            let cpu = parse(part)?;
            (cpu, cpu)
        };
        if first > last || u32::from(last) - u32::from(first) > 1 {
            return Err(reject(
                "PLAYGROUND-HOST-KERNEL",
                "effective CPU count exceeds fixed policy",
            ));
        }
        output.extend(first..=last);
        if output.len() > 2 {
            return Err(reject(
                "PLAYGROUND-HOST-KERNEL",
                "effective CPU count exceeds fixed policy",
            ));
        }
    }
    Ok(output)
}

pub(super) fn capture(state: &Value, cid: &str, cpus: [u16; 2], deadline: Instant) -> Result<Init> {
    let pid = state["State"]["Pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "private init PID unavailable"))?;
    if state["State"]["Running"] != true
        || state["State"]["Paused"] != false
        || state["State"]["Restarting"] != false
        || state["State"]["OOMKilled"] != false
        || state["RestartCount"] != 0
    {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "private init not continuously running"));
    }
    let first = ticks(pid, deadline)?;
    let status = read(pid, "status", deadline)?;
    security(&status, cpus)?;
    let namespace = status
        .lines()
        .find_map(|line| line.strip_prefix("NSpid:"))
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "private PID namespace absent"))?;
    if namespace.split_whitespace().last() != Some("1") || namespace.split_whitespace().count() < 2
    {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "actual private PID1 unproved"));
    }
    let membership = ownership::membership(&read(pid, "cgroup", deadline)?, cid)?;
    let started = state["State"]["StartedAt"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "init start timestamp absent"))?
        .to_owned();
    if ticks(pid, deadline)? != first {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "init changed during observation"));
    }
    Ok(Init { pid, ticks: first, started, cid: cid.to_owned(), membership })
}

pub(super) fn same(
    init: &Init,
    state: &Value,
    cid: &str,
    cpus: [u16; 2],
    deadline: Instant,
) -> Result<()> {
    let current = capture(state, cid, cpus, deadline)?;
    if current != *init {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "original namespace init changed"));
    }
    Ok(())
}

pub(super) fn gone(init: &Init, deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(reject("PLAYGROUND-HOST-DEADLINE", "init removal deadline exceeded"));
    }
    match File::open(format!("/proc/{}/stat", init.pid)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) if ticks(init.pid, deadline)? != init.ticks => Ok(()),
        _ => Err(reject("PLAYGROUND-HOST-TEARDOWN", "original private init removal unproved")),
    }
}

pub(super) fn children_reaped(init: &Init, deadline: Instant) -> Result<bool> {
    // Some supported kernels omit task/*/children. A visible proc scan also observes zombies.
    // The caller brackets this proof with pids.current == 1 and the same single-thread PID1.
    proc_scan::children_reaped(init, deadline)
}
