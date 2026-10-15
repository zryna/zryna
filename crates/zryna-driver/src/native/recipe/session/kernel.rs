//! Corroborate the daemon's private namespace init with the host kernel before use.
use super::{Result, fail};
use serde_json::Value;
use std::{fs::File, io::Read, time::Instant};

#[derive(Debug)]
pub(super) struct Init {
    pub pid: u32,
    pub ticks: String,
    pub started: String,
}
fn read(pid: u32, name: &str, deadline: Instant) -> Result<String> {
    if Instant::now() >= deadline {
        return Err(fail("namespace identity deadline exceeded"));
    }
    let mut bytes = Vec::new();
    File::open(format!("/proc/{pid}/{name}"))
        .map_err(|_| fail("kernel init identity unavailable"))?
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| fail("kernel init identity unreadable"))?;
    if bytes.len() > 16_384 || Instant::now() >= deadline {
        return Err(fail("kernel init identity outside bounds"));
    }
    String::from_utf8(bytes).map_err(|_| fail("kernel init identity invalid"))
}
pub(super) fn ticks(pid: u32, deadline: Instant) -> Result<String> {
    let stat = read(pid, "stat", deadline)?;
    stat.rsplit_once(')')
        .and_then(|(_, fields)| fields.split_whitespace().nth(19))
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or_else(|| fail("kernel init start identity unavailable"))
}
fn security_status(status: &str) -> Result<()> {
    for (key, expected) in [
        ("NoNewPrivs:", "1"),
        ("Seccomp:", "2"),
        ("CapInh:", "0000000000000000"),
        ("CapPrm:", "0000000000000000"),
        ("CapEff:", "0000000000000000"),
        ("CapBnd:", "0000000000000000"),
        ("CapAmb:", "0000000000000000"),
    ] {
        let mut values = status.lines().filter_map(|line| line.strip_prefix(key));
        if values.next().map(str::trim) != Some(expected) || values.next().is_some() {
            return Err(fail("actual NNP, seccomp or zero capabilities unproved"));
        }
    }
    Ok(())
}
pub(super) fn capture(state: &Value, cid: &str, deadline: Instant) -> Result<Init> {
    let pid = state["State"]["Pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| fail("private init PID unavailable"))?;
    if state["State"]["Running"] != true
        || state["State"]["Paused"] != false
        || state["State"]["Restarting"] != false
        || state["State"]["OOMKilled"] != false
        || state["RestartCount"] != 0
    {
        return Err(fail("private init is not continuously running"));
    }
    let first_ticks = ticks(pid, deadline)?;
    let status = read(pid, "status", deadline)?;
    security_status(&status)?;
    let namespace = status
        .lines()
        .find_map(|line| line.strip_prefix("NSpid:"))
        .ok_or_else(|| fail("private PID namespace unproved"))?;
    if namespace.split_whitespace().last() != Some("1") || namespace.split_whitespace().count() < 2
    {
        return Err(fail("recipe init is not private PID1"));
    }
    for key in ["Uid:", "Gid:"] {
        if status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .is_none_or(|ids| ids.split_whitespace().collect::<Vec<_>>() != ["65534"; 4])
        {
            return Err(fail("fixed nonroot kernel credentials unproved"));
        }
    }
    let cgroup = read(pid, "cgroup", deadline)?;
    if !cgroup
        .lines()
        .any(|line| line.starts_with("0::/") && line.ends_with(&format!("/docker/{cid}")))
    {
        return Err(fail("owned cgroup membership unproved"));
    }
    let started = state["State"]["StartedAt"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| fail("init start timestamp absent"))?
        .to_owned();
    if ticks(pid, deadline)? != first_ticks {
        return Err(fail("kernel init changed during capture"));
    }
    Ok(Init { pid, ticks: first_ticks, started })
}
pub(super) fn same(init: &Init, state: &Value, cid: &str, deadline: Instant) -> Result<()> {
    let current = capture(state, cid, deadline)?;
    if current.pid != init.pid || current.ticks != init.ticks || current.started != init.started {
        return Err(fail("owned init changed during execution"));
    }
    Ok(())
}
pub(super) fn gone(init: &Init, deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(fail("namespace teardown deadline exceeded"));
    }
    match File::open(format!("/proc/{}/stat", init.pid)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) if ticks(init.pid, deadline)? != init.ticks => Ok(()),
        _ => Err(fail("original namespace init teardown unproved")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn kernel_security_fields_reject_nonzero_missing_duplicate_and_unfiltered_status() {
        let fields = [
            ("NoNewPrivs", "1"),
            ("Seccomp", "2"),
            ("CapInh", "0000000000000000"),
            ("CapPrm", "0000000000000000"),
            ("CapEff", "0000000000000000"),
            ("CapBnd", "0000000000000000"),
            ("CapAmb", "0000000000000000"),
        ];
        let original = fields.iter().fold(String::new(), |mut output, (key, value)| {
            writeln!(output, "{key}:\t{value}").expect("fixture string");
            output
        });
        assert!(security_status(&original).is_ok());
        for (key, value) in fields {
            let line = format!("{key}:\t{value}\n");
            for invalid in ["", "true", "0000000000000001", "0", " 1 2"] {
                let changed = original.replace(&line, &format!("{key}:\t{invalid}\n"));
                assert!(security_status(&changed).is_err(), "{key}");
            }
            assert!(security_status(&original.replace(&line, "")).is_err());
            assert!(security_status(&format!("{original}{line}")).is_err());
        }
        assert!(security_status(&original).is_ok());
    }
}
