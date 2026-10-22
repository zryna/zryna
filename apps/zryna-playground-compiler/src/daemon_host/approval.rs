//! Caller-supplied hash authenticates captured bytes; adjacent metadata has no authority.

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::OpenOptionsExt as _;
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;
use std::{
    io::Read,
    path::{Component, Path},
};

const ERROR: &str = "PLAYGROUND-HOST-APPROVAL";
const MAX_APPROVAL_BYTES: usize = 262_144;

fn identity(directory: &Dir) -> Result<Handle, &'static str> {
    directory.try_clone().map(Dir::into_std_file).and_then(Handle::from_file).map_err(|_| ERROR)
}

fn ordinary(parent: &Dir, name: &std::ffi::OsStr) -> Result<Handle, &'static str> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    // Prevent a FIFO substituted at a trusted-looking path from blocking before metadata checks.
    options.custom_flags(0o4000); // Linux O_NONBLOCK; this entire host mode is Linux x86-64 only.
    parent
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
        .and_then(Handle::from_file)
        .map_err(|_| ERROR)
}

pub(super) fn capture(path: &Path, expected: &str) -> Result<Vec<u8>, &'static str> {
    if !path.is_absolute() || !super::evaluation::hash(expected) {
        return Err(ERROR);
    }
    let components = path.components().collect::<Vec<_>>();
    if components.len() < 2
        || components.len() > 128
        || !matches!(components[0], Component::RootDir)
        || components[1..].iter().any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ERROR);
    }
    let mut directory = Dir::open_ambient_dir("/", ambient_authority()).map_err(|_| ERROR)?;
    let mut ancestors = Vec::new();
    for component in &components[1..components.len() - 1] {
        let Component::Normal(name) = component else {
            return Err(ERROR);
        };
        let child = directory.open_dir_nofollow(name).map_err(|_| ERROR)?;
        let reopened = directory.open_dir_nofollow(name).map_err(|_| ERROR)?;
        if identity(&child)? != identity(&reopened)? {
            return Err(ERROR);
        }
        ancestors.push((directory, *name, identity(&child)?));
        directory = child;
    }
    let Component::Normal(name) = components[components.len() - 1] else {
        return Err(ERROR);
    };
    let mut original = ordinary(&directory, name)?;
    let before = original.as_file().metadata().map_err(|_| ERROR)?;
    if !before.is_file() || before.len() == 0 || before.len() > MAX_APPROVAL_BYTES as u64 {
        return Err(ERROR);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(before.len()).map_err(|_| ERROR)?);
    original
        .as_file_mut()
        .take((MAX_APPROVAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ERROR)?;
    let reopened = ordinary(&directory, name)?;
    let after = original.as_file().metadata().map_err(|_| ERROR)?;
    let current = reopened.as_file().metadata().map_err(|_| ERROR)?;
    let state = |value: &std::fs::Metadata| {
        (
            value.dev(),
            value.ino(),
            value.len(),
            value.mode(),
            value.uid(),
            value.gid(),
            value.mtime(),
            value.mtime_nsec(),
            value.ctime(),
            value.ctime_nsec(),
        )
    };
    if bytes.len() > MAX_APPROVAL_BYTES
        || original != reopened
        || state(&before) != state(&after)
        || state(&before) != state(&current)
        || format!("{:x}", Sha256::digest(&bytes)) != expected
    {
        return Err(ERROR);
    }
    for (parent, name, retained) in ancestors {
        if identity(&parent.open_dir_nofollow(name).map_err(|_| ERROR)?)? != retained {
            return Err(ERROR);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_pin_and_no_follow_paths_are_required() {
        let root = std::env::temp_dir().join(format!("playground-approval-{}", std::process::id()));
        std::fs::create_dir(&root).expect("owned fixture root");
        let path = root.join("approval.json");
        std::fs::write(&path, b"authenticated operator record").expect("fixture bytes");
        let pin = format!("{:x}", Sha256::digest(b"authenticated operator record"));
        assert_eq!(capture(&path, &pin).expect("external pin"), b"authenticated operator record");
        assert!(capture(&path, &"0".repeat(64)).is_err());
        let linked = root.join("linked.json");
        std::os::unix::fs::symlink(&path, &linked).expect("leaf link");
        assert!(capture(&linked, &pin).is_err());
        let linked_root = root.with_extension("linked");
        std::os::unix::fs::symlink(&root, &linked_root).expect("ancestor link");
        assert!(capture(&linked_root.join("approval.json"), &pin).is_err());
        std::fs::remove_file(linked_root).expect("owned link cleanup");
        std::fs::remove_dir_all(root).expect("owned fixture cleanup");
    }
}
