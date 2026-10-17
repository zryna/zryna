//! Complete visible-proc child observation, including zombies omitted by pids.current.
use super::{Init, Result, read, reject, ticks};
use std::{
    fs,
    io::{self, Read},
    path::Path,
    time::Instant,
};

const LIMIT: usize = 8192;

fn check(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(reject("PLAYGROUND-HOST-DEADLINE", "complete child observation timed out"));
    }
    Ok(())
}

fn visible_proc(mountinfo: &str) -> Result<()> {
    let mut found = false;
    for line in mountinfo.lines() {
        let (mount, filesystem) = line.split_once(" - ").ok_or_else(|| {
            reject("PLAYGROUND-HOST-KERNEL", "actual proc mount record malformed")
        })?;
        let fields = mount.split_whitespace().collect::<Vec<_>>();
        let fs = filesystem.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 || fs.len() < 3 {
            return Err(reject("PLAYGROUND-HOST-KERNEL", "actual proc mount record incomplete"));
        }
        if fields[4] != "/proc" {
            continue;
        }
        if found
            || fields[3] != "/"
            || fs[0] != "proc"
            || fields[5]
                .split(',')
                .chain(fs[2].split(','))
                .any(|option| option.starts_with("hidepid") && option != "hidepid=0")
        {
            return Err(reject("PLAYGROUND-HOST-KERNEL", "complete process visibility unproved"));
        }
        found = true;
    }
    if !found {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "actual full proc mount absent"));
    }
    Ok(())
}

fn process(text: &str, expected: u32) -> Result<(u32, char)> {
    let invalid = || reject("PLAYGROUND-HOST-KERNEL", "bounded actual process record malformed");
    let (prefix, suffix) = text.rsplit_once(')').ok_or_else(invalid)?;
    let (pid, _) = prefix.split_once('(').ok_or_else(invalid)?;
    if pid.trim().parse::<u32>().map_err(|_| invalid())? != expected {
        return Err(invalid());
    }
    let fields = suffix.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 20
        || fields[0].len() != 1
        || !fields[0].as_bytes()[0].is_ascii_alphabetic()
        || fields[19].is_empty()
        || !fields[19].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let parent = fields[1].parse::<u32>().map_err(|_| invalid())?;
    Ok((parent, char::from(fields[0].as_bytes()[0])))
}

fn stat(pid: u32, deadline: Instant) -> Result<Option<String>> {
    check(deadline)?;
    let file = match fs::File::open(format!("/proc/{pid}/stat")) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(reject("PLAYGROUND-HOST-KERNEL", "actual process read refused")),
    };
    let mut bytes = Vec::new();
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "actual process read interrupted"))?;
    check(deadline)?;
    if bytes.len() > 16_384 {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "actual process record exceeds finite bound"));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "actual process record encoding malformed"))
}

fn scan(parent: u32, deadline: Instant) -> Result<bool> {
    check(deadline)?;
    let entries = fs::read_dir("/proc")
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "complete process inventory unavailable"))?;
    let mut count = 0;
    for entry in entries {
        check(deadline)?;
        let entry = entry.map_err(|_| {
            reject("PLAYGROUND-HOST-KERNEL", "complete process inventory interrupted")
        })?;
        let name = entry.file_name();
        let Some(name) = name
            .to_str()
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        else {
            continue;
        };
        count += 1;
        if count > LIMIT {
            return Err(reject("PLAYGROUND-HOST-KERNEL", "process inventory exceeds finite bound"));
        }
        let pid = name
            .parse::<u32>()
            .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "actual process identity malformed"))?;
        // ENOENT only: an unreaped zombie cannot disappear. Never skip denied/incomplete reads.
        let Some(text) = stat(pid, deadline)? else {
            continue;
        };
        if process(&text, pid)?.0 == parent {
            return Ok(false);
        }
    }
    check(deadline)?;
    Ok(true)
}

pub(super) fn children_reaped(init: &Init, deadline: Instant) -> Result<bool> {
    check(deadline)?;
    let pid = std::process::id();
    let actual = fs::read_link("/proc/self")
        .map_err(|_| reject("PLAYGROUND-HOST-KERNEL", "actual proc namespace identity absent"))?;
    if actual != Path::new(&pid.to_string()) || ticks(init.pid, deadline)? != init.ticks {
        return Err(reject("PLAYGROUND-HOST-KERNEL", "process namespace or original init changed"));
    }
    visible_proc(&read(pid, "mountinfo", deadline)?)?;
    let reaped = scan(init.pid, deadline)?;
    if ticks(init.pid, deadline)? != init.ticks {
        return Err(reject(
            "PLAYGROUND-HOST-KERNEL",
            "init changed during complete child observation",
        ));
    }
    check(deadline)?;
    Ok(reaped)
}

#[cfg(test)]
mod tests;
