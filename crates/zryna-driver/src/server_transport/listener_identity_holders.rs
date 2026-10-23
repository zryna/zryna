//! Failure-only, best-effort kernel holder observations; absence grants no closure authority.
use std::{
    fs::{self, File},
    io::Read,
    time::Instant,
};

const TASKS: usize = 128;
const CHILDREN: usize = 128;
const CHILD_BYTES: u64 = 4096;
const PROCESS_FDS: usize = 1024;
const TOTAL_FDS: usize = 4096;
const HOLDERS: usize = 64;
const NOTES: usize = 16;

struct Scan {
    deadline: Instant,
    inode: u64,
    holders: Vec<(u32, u32)>,
    notes: Vec<String>,
    omitted_notes: usize,
    tasks: usize,
    children: usize,
    fds: usize,
    stopped: bool,
}
impl Scan {
    fn note(&mut self, message: String) {
        if self.notes.len() < NOTES {
            self.notes.push(message);
        } else {
            self.omitted_notes += 1;
        }
    }
    fn active(&mut self) -> bool {
        if !self.stopped && Instant::now() >= self.deadline {
            self.note("original probe deadline exhausted during holder observation".into());
            self.stopped = true;
        }
        !self.stopped
    }
    fn error(&mut self, stage: &str, pid: u32, error: &std::io::Error) {
        // ErrorKind only: never print arbitrary path, command, environment or link contents.
        self.note(format!("{stage}: pid={pid} kind={:?}; observation incomplete", error.kind()));
    }
    fn fds(&mut self, pid: u32) {
        if !self.active() {
            return;
        }
        let directory = match fs::read_dir(format!("/proc/{pid}/fd")) {
            Ok(directory) => directory,
            Err(error) => {
                self.error("fd-directory", pid, &error);
                return;
            }
        };
        for (count, entry) in directory.enumerate() {
            if !self.active() {
                return;
            }
            if count == PROCESS_FDS || self.fds == TOTAL_FDS {
                self.note(format!("fd entry limit: pid={pid}; observation truncated"));
                self.stopped |= self.fds == TOTAL_FDS;
                return;
            }
            self.fds += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    self.error("fd-entry", pid, &error);
                    continue;
                }
            };
            let Some(fd) = number(&entry.file_name()) else {
                self.note(format!("nonnumeric fd entry: pid={pid}; observation incomplete"));
                continue;
            };
            match fs::read_link(entry.path()) {
                Ok(target) => {
                    if !self.active() {
                        return;
                    }
                    let matched = target.to_str().and_then(|text| {
                        text.strip_prefix("socket:[")?.strip_suffix(']')?.parse::<u64>().ok()
                    }) == Some(self.inode);
                    if matched {
                        if self.holders.len() == HOLDERS {
                            self.note("holder limit; observation truncated".into());
                            self.stopped = true;
                            return;
                        }
                        self.holders.push((pid, fd));
                    }
                }
                Err(error) => self.error("fd-read-link", pid, &error),
            }
        }
    }
    fn children(&mut self, pid: u32) -> Vec<u32> {
        let mut children = Vec::new();
        if !self.active() {
            return children;
        }
        let directory = match fs::read_dir(format!("/proc/{pid}/task")) {
            Ok(directory) => directory,
            Err(error) => {
                self.error("task-directory", pid, &error);
                return children;
            }
        };
        for entry in directory {
            if !self.active() {
                break;
            }
            if self.tasks == TASKS {
                self.note("task entry limit; observation truncated".into());
                break;
            }
            self.tasks += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    self.error("task-entry", pid, &error);
                    continue;
                }
            };
            let Some(task) = number(&entry.file_name()) else {
                self.note("nonnumeric task entry; observation incomplete".into());
                continue;
            };
            let path = format!("/proc/{pid}/task/{task}/children");
            let mut bytes = Vec::new();
            let result = File::open(path)
                .and_then(|file| file.take(CHILD_BYTES + 1).read_to_end(&mut bytes));
            if !self.active() {
                break;
            }
            if let Err(error) = result {
                self.error("task-children", pid, &error);
                continue;
            }
            if bytes.len() > usize::try_from(CHILD_BYTES).expect("fixed child bound fits usize") {
                self.note(format!("children byte limit: task={task}; observation truncated"));
                continue;
            }
            for token in bytes.split(u8::is_ascii_whitespace).filter(|token| !token.is_empty()) {
                let child = std::str::from_utf8(token)
                    .ok()
                    .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
                    .and_then(|text| text.parse::<u32>().ok());
                let Some(child) = child.filter(|child| *child != 0) else {
                    self.note(format!("invalid child record: task={task}; observation incomplete"));
                    continue;
                };
                if !children.contains(&child) {
                    if children.len() == CHILDREN {
                        self.note("child count limit; observation truncated".into());
                        return children;
                    }
                    children.push(child);
                }
            }
        }
        children.sort_unstable();
        children
    }
}
fn number(name: &std::ffi::OsStr) -> Option<u32> {
    let text = name.to_str()?;
    if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

pub(super) fn collect(pid: u32, inode: u64, deadline: Instant) -> String {
    let mut scan = Scan {
        deadline,
        inode,
        holders: Vec::new(),
        notes: Vec::new(),
        omitted_notes: 0,
        tasks: 0,
        children: 0,
        fds: 0,
        stopped: false,
    };
    scan.fds(pid);
    for child in scan.children(pid) {
        if !scan.active() {
            break;
        }
        scan.children += 1;
        scan.fds(child);
    }
    format!(
        "observation-only original_inode={inode} holders_pid_fd={:?} tasks={} direct_children={} fd_entries={} stopped={} notes={:?} omitted_notes={}; absence is not closure proof; child/task/fd inventories can race",
        scan.holders,
        scan.tasks,
        scan.children,
        scan.fds,
        scan.stopped,
        scan.notes,
        scan.omitted_notes,
    )
}
