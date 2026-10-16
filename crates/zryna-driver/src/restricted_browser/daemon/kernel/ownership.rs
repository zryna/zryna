//! Original process ownership is retained independently of execution admission.
use super::{Init, Result, read, reject, ticks};
use serde_json::Value;
use std::time::Instant;

const NEVER_STARTED: &str = "0001-01-01T00:00:00Z";

fn epoch<'a>(state: &'a Value, cid: &str) -> Result<(u32, &'a str)> {
    let pid = state["State"]["Pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "owned init PID unavailable"))?;
    let started = state["State"]["StartedAt"]
        .as_str()
        .filter(|value| !value.is_empty() && *value != NEVER_STARTED)
        .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "owned init start timestamp absent"))?;
    if state["Id"] != cid
        || state["State"]["Running"] != true
        || state["State"]["Paused"] != false
        || state["State"]["Restarting"] != false
        || state["RestartCount"] != 0
    {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "owned original init epoch unproved"));
    }
    Ok((pid, started))
}

pub(super) fn membership(text: &str, cid: &str) -> Result<String> {
    let mut lines = text.lines().filter(|line| line.starts_with("0::/"));
    let value = lines.next();
    if cid.len() != 64
        || !cid.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.is_none_or(|line| !line.ends_with(&format!("/docker/{cid}")))
        || lines.next().is_some()
    {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "owned actual cgroup membership unproved"));
    }
    Ok(value.unwrap_or_default().to_owned())
}

fn observed(state: &Value, cid: &str, first: String, group: &str, last: &str) -> Result<Init> {
    let (pid, started) = epoch(state, cid)?;
    if first.is_empty() || !first.bytes().all(|byte| byte.is_ascii_digit()) || first != last {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "owned init changed during observation"));
    }
    Ok(Init {
        pid,
        ticks: first,
        started: started.to_owned(),
        cid: cid.to_owned(),
        membership: membership(group, cid)?,
    })
}

pub(in super::super) fn capture_owned(state: &Value, cid: &str, deadline: Instant) -> Result<Init> {
    let (pid, _) = epoch(state, cid)?;
    let first = ticks(pid, deadline)?;
    let group = read(pid, "cgroup", deadline)?;
    observed(state, cid, first, &group, &ticks(pid, deadline)?)
}

fn same_original(original: &Init, current: &Init) -> Result<()> {
    if current != original {
        return Err(reject("PLAYGROUND-HOST-TEARDOWN", "original owned init identity changed"));
    }
    Ok(())
}

fn stopped(init: Option<&Init>, state: &Value, cid: &str) -> Result<()> {
    if state["Id"] != cid
        || state["State"]["Running"] != false
        || state["State"]["Pid"] != 0
        || state["State"]["Paused"] != false
        || state["State"]["Restarting"] != false
        || state["RestartCount"] != 0
    {
        return Err(reject("PLAYGROUND-HOST-TEARDOWN", "owned stopped epoch unproved"));
    }
    let same_epoch = init.map_or_else(
        || {
            state["State"]["Status"] == "created"
                && state["State"]["StartedAt"] == NEVER_STARTED
                && state["State"]["FinishedAt"] == NEVER_STARTED
        },
        |original| original.cid == cid && state["State"]["StartedAt"] == original.started,
    );
    if !same_epoch {
        return Err(reject("PLAYGROUND-HOST-TEARDOWN", "original stopped ownership unavailable"));
    }
    Ok(())
}

pub(in super::super) fn cleanup_owned(
    init: Option<&Init>,
    state: &Value,
    cid: &str,
    deadline: Instant,
) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(reject("PLAYGROUND-HOST-DEADLINE", "owned cleanup identity deadline exceeded"));
    }
    if state["State"]["Running"] == true {
        let original = init.ok_or_else(|| {
            reject("PLAYGROUND-HOST-TEARDOWN", "original running ownership unavailable")
        })?;
        // These reads authorize only destruction of the original owned CID, never execution.
        same_original(original, &capture_owned(state, cid, deadline)?)
    } else {
        stopped(init, state, cid)
    }
}

#[cfg(test)]
mod tests;
