//! Held-component input/output retention. Serialized evidence grants no execution authority.
use super::*;
use nix::{
    fcntl::{OFlag, openat},
    sys::stat::Mode,
};
use std::{
    ffi::CString,
    fs::File,
    io::{Read as _, Write as _},
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::{Component, Path, PathBuf},
};
#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity {
    device: u64,
    inode: u64,
    mode: u32,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Identity {
    fn of(file: &File) -> Result<Self, Diagnostic> {
        let m = file.metadata().map_err(|_| rejected())?;
        Ok(Self {
            device: m.dev(),
            inode: m.ino(),
            mode: m.mode(),
            length: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        })
    }
    fn same_directory(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }
}
fn open(parent: &File, name: &str, flags: OFlag) -> Result<File, Diagnostic> {
    let name = CString::new(name).map_err(|_| rejected())?;
    Ok(File::from(
        openat(
            parent,
            name.as_c_str(),
            flags | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| rejected())?,
    ))
}
pub(super) struct Directory {
    chain: Vec<(File, Identity, String)>,
}
impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self, Diagnostic> {
        if !path.is_absolute() || path.components().collect::<PathBuf>() != path {
            return Err(rejected());
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")
            .map_err(|_| rejected())?;
        let mut chain =
            vec![(file.try_clone().map_err(|_| rejected())?, Identity::of(&file)?, String::new())];
        for part in path.components() {
            let Component::Normal(part) = part else {
                if part == Component::RootDir {
                    continue;
                }
                return Err(rejected());
            };
            let name = part.to_str().ok_or_else(rejected)?.to_owned();
            let f = open(
                &chain.last().ok_or_else(rejected)?.0,
                &name,
                OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NONBLOCK,
            )?;
            chain.push((f.try_clone().map_err(|_| rejected())?, Identity::of(&f)?, name));
        }
        let value = Self { chain };
        value.revalidate()?;
        Ok(value)
    }
    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        for (index, (file, id, name)) in self.chain.iter().enumerate() {
            if !id.same_directory(&Identity::of(file)?) {
                return Err(rejected());
            }
            if index > 0
                && !id.same_directory(&Identity::of(&open(
                    &self.chain[index - 1].0,
                    name,
                    OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NONBLOCK,
                )?)?)
            {
                return Err(rejected());
            }
        }
        Ok(())
    }
    pub(super) fn anchored(&self, name: &str) -> Result<PathBuf, Diagnostic> {
        use std::os::fd::AsRawFd as _;
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            return Err(rejected());
        }
        self.revalidate()?;
        Ok(PathBuf::from(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            self.chain.last().ok_or_else(rejected)?.0.as_raw_fd()
        ))
        .join(name))
    }
    fn with_file<T>(
        &self,
        name: &str,
        limit: usize,
        read: impl FnOnce(&mut File) -> Result<(T, usize), Diagnostic>,
    ) -> Result<T, Diagnostic> {
        self.revalidate()?;
        let _path = self.anchored(name)?;
        let parent = &self.chain.last().ok_or_else(rejected)?.0;
        let before = Identity::of(&open(parent, name, OFlag::O_PATH)?)?;
        if before.mode & libc::S_IFMT != libc::S_IFREG
            || before.length > u64::try_from(limit).map_err(|_| rejected())?
        {
            return Err(rejected());
        }
        let mut file = open(parent, name, OFlag::O_RDONLY | OFlag::O_NONBLOCK)?;
        if Identity::of(&file)? != before {
            return Err(rejected());
        }
        let (value, length) = read(&mut file)?;
        if length > limit
            || u64::try_from(length).map_err(|_| rejected())? != before.length
            || Identity::of(&file)? != before
            || Identity::of(&open(parent, name, OFlag::O_PATH)?)? != before
        {
            return Err(rejected());
        }
        self.revalidate()?;
        Ok(value)
    }
    pub(super) fn read(&self, name: &str, limit: usize) -> Result<Vec<u8>, Diagnostic> {
        self.with_file(name, limit, |file| {
            let mut bytes = Vec::new();
            file.take(u64::try_from(limit).map_err(|_| rejected())? + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| rejected())?;
            let length = bytes.len();
            Ok((bytes, length))
        })
    }
    pub(super) fn hash(&self, name: &str, limit: usize) -> Result<String, Diagnostic> {
        self.with_file(name, limit, |file| {
            let mut h = Sha256::new();
            let mut buffer = [0; 16_384];
            let mut length = 0_usize;
            loop {
                let n = file.read(&mut buffer).map_err(|_| rejected())?;
                if n == 0 {
                    break;
                }
                length = length.checked_add(n).ok_or_else(rejected)?;
                if length > limit {
                    return Err(rejected());
                }
                h.update(&buffer[..n]);
            }
            Ok((format!("{:x}", h.finalize()), length))
        })
    }
    pub(super) fn write(&self, name: &str, bytes: &[u8]) -> Result<(), Diagnostic> {
        let path = self.anchored(name)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| rejected())?;
        file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|_| rejected())?;
        let held = Identity::of(&file)?;
        let named =
            Identity::of(&open(&self.chain.last().ok_or_else(rejected)?.0, name, OFlag::O_PATH)?)?;
        if held != named || held.length != u64::try_from(bytes.len()).map_err(|_| rejected())? {
            return Err(rejected());
        }
        self.revalidate()
    }
}
pub(super) fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Diagnostic> {
    Directory::open(path.parent().ok_or_else(rejected)?)?
        .read(path.file_name().and_then(|s| s.to_str()).ok_or_else(rejected)?, limit)
}
pub(super) fn create(parent: &Path, name: &str) -> Result<(PathBuf, Directory), Diagnostic> {
    use std::os::unix::fs::DirBuilderExt as _;
    let held = Directory::open(parent)?;
    std::fs::DirBuilder::new().mode(0o700).create(held.anchored(name)?).map_err(|_| rejected())?;
    held.revalidate()?;
    let path = parent.join(name);
    let child = Directory::open(&path)?;
    child.revalidate()?;
    Ok((path, child))
}
pub(super) fn record(value: &process::Record) -> serde_json::Value {
    serde_json::json!({"name":value.name,"program":value.program,"arguments_bytes":value.arguments.iter().map(|a|a.as_encoded_bytes()).collect::<Vec<_>>(),"elapsed_ms":value.elapsed_ms,"directory":value.directory,"exit_status":value.output.as_ref().map(|o|o.status.to_string()),"success":value.output.as_ref().map(|o|o.status.success()),"diagnostic":value.diagnostic.as_ref().map(Diagnostic::code),"error_raw_fields_available":value.output.is_some(),"child_env_cleared":true,"child_env":{"LANG":"C","LC_ALL":"C","TZ":"UTC","SOURCE_DATE_EPOCH":"0","PATH":"/usr/bin:/bin","TMPDIR":value.directory},"network_denial_proven":false})
}
pub(super) fn finish(
    session: &Session<'_>,
    parent: &Path,
    name: &str,
) -> Result<PathBuf, Diagnostic> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let name = format!(
        "ordinary-rust-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let mut artifacts = session
        .artifacts
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.as_slice()))
        .collect::<Vec<_>>();
    for value in &session.records {
        if let Some(output) = &value.output {
            artifacts.push((format!("{}.stdout.log", value.name), &output.stdout));
            artifacts.push((format!("{}.stderr.log", value.name), &output.stderr));
        }
    }
    let mut names = std::collections::BTreeSet::new();
    let mut total = 0_usize;
    for (name, bytes) in &artifacts {
        if name.is_empty()
            || !name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b".-".contains(&c))
            || !names.insert(name.to_ascii_lowercase())
        {
            return Err(rejected());
        }
        total = total.checked_add(bytes.len()).ok_or_else(rejected)?;
    }
    if artifacts.len() > 64 || total > 96 * 1024 * 1024 {
        return Err(rejected());
    }
    let inventory = artifacts.iter().map(|(name,bytes)|
        serde_json::json!({"path":name,"bytes":bytes.len(),"sha256":digest(bytes)}))
        .collect::<Vec<_>>();
    let prerequisite=session.measurement.map(|m|serde_json::json!({"retained_evidence":m.evidence,"source_sha256":digest(&m.source),"object_sha256":digest(&m.object),"records":m.records.iter().map(record).collect::<Vec<_>>(),"rust_tool_observations":m.rust_observations,"original_receipt_sha256":digest(&m.receipt),"origin":"fresh ordinary trusted Rust producer completed before this C observer; no public/compiler admission capability"}));
    let report = serde_json::json!({"kind":"ordinary-trusted-distinct-rust-fixture-observation","records":session.records.iter().map(record).collect::<Vec<_>>(),"artifacts":inventory,"diagnostics":session.diagnostics.iter().map(Diagnostic::code).collect::<Vec<_>>(),"rust_targets_checked":session.targets_checked,"rust_compile_in_this_observation":session.rust_tools.is_some(),"rust_prerequisite":prerequisite,"ordinary_tool_observations":session.tools.report(),"rust_tool_observations":session.rust_tools.map(material::RustTools::report),"comparator_stage":session.phase,"GCC_origin_for_Rust":false,"production_admission":false,"private_recipe_qualified":false,"complete_input_use":false,"complete_provider_inventory":false,"global_leak_absence_proven":false,"retention_contract_proven_by_counter":false});
    let bytes = serde_json::to_vec_pretty(&report).map_err(|_| rejected())?;
    if bytes.len() > 1024 * 1024
        || names.contains("receipt.json")
        || !receipt_fits(total, artifacts.len(), bytes.len())
    {
        return Err(rejected());
    }
    let (path, held) = create(parent, &name)?;
    for (name, payload) in artifacts {
        held.write(&name, payload)?;
    }
    held.write("receipt.json", &bytes)?;
    Ok(path)
}

// Receipt is a member of the unchanged closed per-observation inventory.
pub(super) fn receipt_fits(payload: usize, entries: usize, receipt: usize) -> bool {
    entries < 64 && payload.checked_add(receipt).is_some_and(|n| n <= 96 * 1024 * 1024)
}
