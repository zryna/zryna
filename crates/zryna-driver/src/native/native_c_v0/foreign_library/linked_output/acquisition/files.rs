use super::*;
use nix::{fcntl::OFlag, sys::stat::Mode};
use std::{
    ffi::CString,
    fs::File,
    io::Read as _,
    os::unix::{ffi::OsStrExt as _, fs::MetadataExt as _},
    path::{Component, Path},
};

#[derive(Clone, Debug, PartialEq)]
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
        let value = file.metadata().map_err(|_| rejected())?;
        Ok(Self {
            device: value.dev(),
            inode: value.ino(),
            mode: value.mode(),
            length: value.len(),
            modified: (value.mtime(), value.mtime_nsec()),
            changed: (value.ctime(), value.ctime_nsec()),
        })
    }
    fn directory(&self) -> bool {
        self.mode & libc::S_IFMT == libc::S_IFDIR
    }
    fn same_directory(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }
}

struct Directory {
    file: File,
    identity: Identity,
    name: Option<CString>,
}
pub(super) struct Root {
    chain: Vec<Directory>,
}

fn open(parent: &File, name: &CString, directory: bool) -> Result<File, Diagnostic> {
    let flags = OFlag::O_RDONLY
        | OFlag::O_CLOEXEC
        | OFlag::O_NOFOLLOW
        | OFlag::O_NONBLOCK
        | if directory { OFlag::O_DIRECTORY } else { OFlag::empty() };
    let descriptor = nix::fcntl::openat(parent, name.as_c_str(), flags, Mode::empty())
        .map_err(|_| rejected())?;
    Ok(File::from(descriptor))
}

fn revalidate(chain: &[Directory]) -> Result<(), Diagnostic> {
    for (index, directory) in chain.iter().enumerate() {
        if !directory.identity.same_directory(&Identity::of(&directory.file)?) {
            return Err(rejected());
        }
        if let Some(name) = &directory.name {
            let current = open(&chain[index - 1].file, name, true)?;
            if !directory.identity.same_directory(&Identity::of(&current)?) {
                return Err(rejected());
            }
        }
    }
    Ok(())
}

impl Root {
    pub(super) fn open(path: &Path) -> Result<Self, Diagnostic> {
        use std::os::unix::fs::OpenOptionsExt as _;
        if !path.is_absolute()
            || path.components().collect::<PathBuf>().as_os_str() != path.as_os_str()
        {
            return Err(rejected());
        }
        let base = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")
            .map_err(|_| rejected())?;
        let mut chain = vec![Directory { identity: Identity::of(&base)?, file: base, name: None }];
        for component in path.components() {
            let name = match component {
                Component::RootDir => continue,
                Component::Normal(value) => {
                    CString::new(value.as_bytes()).map_err(|_| rejected())?
                }
                _ => return Err(rejected()),
            };
            let file = open(&chain.last().ok_or_else(rejected)?.file, &name, true)?;
            let identity = Identity::of(&file)?;
            if !identity.directory() {
                return Err(rejected());
            }
            chain.push(Directory { file, identity, name: Some(name) });
        }
        revalidate(&chain)?;
        Ok(Self { chain })
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        revalidate(&self.chain)
    }

    pub(super) fn read(&self, path: &str, limit: usize) -> Result<Vec<u8>, Diagnostic> {
        self.with_file(path, limit, |file| {
            let mut bytes = Vec::new();
            file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|_| rejected())?;
            let length = bytes.len();
            Ok((bytes, length))
        })
    }

    pub(super) fn compare(&self, path: &str, expected: &[u8]) -> Result<(), Diagnostic> {
        self.with_file(path, expected.len(), |mut file| {
            let mut position = 0_usize;
            let mut buffer = [0; 16384];
            loop {
                let length = file.read(&mut buffer).map_err(|_| rejected())?;
                if length == 0 {
                    break;
                }
                let end = position.checked_add(length).ok_or_else(rejected)?;
                if expected.get(position..end) != Some(&buffer[..length]) {
                    return Err(rejected());
                }
                position = end;
            }
            if position != expected.len() {
                return Err(rejected());
            }
            Ok(((), position))
        })
    }

    fn with_file<T>(
        &self,
        path: &str,
        limit: usize,
        read: impl FnOnce(&File) -> Result<(T, usize), Diagnostic>,
    ) -> Result<T, Diagnostic> {
        self.revalidate()?;
        let components = path.split('/').collect::<Vec<_>>();
        if components.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
            return Err(rejected());
        }
        let mut descendants = Vec::new();
        let root = &self.chain.last().ok_or_else(rejected)?.file;
        for part in &components[..components.len() - 1] {
            let parent = descendants.last().map_or(root, |value: &Directory| &value.file);
            let name = CString::new(*part).map_err(|_| rejected())?;
            let file = open(parent, &name, true)?;
            let identity = Identity::of(&file)?;
            if !identity.directory() {
                return Err(rejected());
            }
            descendants.push(Directory { file, identity, name: Some(name) });
        }
        let parent = descendants.last().map_or(root, |value| &value.file);
        let name =
            CString::new(*components.last().ok_or_else(rejected)?).map_err(|_| rejected())?;
        // O_PATH acquires the named identity without opening a FIFO/device for IO.
        let descriptor = nix::fcntl::openat(
            parent,
            name.as_c_str(),
            OFlag::O_PATH | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| rejected())?;
        let metadata = File::from(descriptor);
        let before = Identity::of(&metadata)?;
        if before.mode & libc::S_IFMT != libc::S_IFREG
            || before.length == 0
            || before.length > limit as u64
        {
            return Err(rejected());
        }
        let file = open(parent, &name, false)?;
        if before != Identity::of(&file)? {
            return Err(rejected());
        }
        let (value, length) = read(&file)?;
        let after = Identity::of(&file)?;
        let named = open(parent, &name, false)?;
        if before != after || before != Identity::of(&named)? || length as u64 != before.length {
            return Err(rejected());
        }
        self.revalidate()?;
        let mut prior = root;
        for item in &descendants {
            let named = open(prior, item.name.as_ref().ok_or_else(rejected)?, true)?;
            if !item.identity.same_directory(&Identity::of(&named)?) {
                return Err(rejected());
            }
            prior = &item.file;
        }
        Ok(value)
    }
}
