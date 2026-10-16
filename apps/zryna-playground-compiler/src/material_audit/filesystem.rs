//! Streaming, no-follow exact-tree audit retains only a bounded directory ancestry.

use super::{
    ERROR,
    manifest::{Manifest, Material, name, parent},
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptionsExt};
use same_file::Handle;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs::Metadata, io::Read, os::unix::fs::MetadataExt};

fn identity(directory: &Dir) -> Result<Handle, &'static str> {
    directory.try_clone().map(Dir::into_std_file).and_then(Handle::from_file).map_err(|_| ERROR)
}

fn state(metadata: &Metadata) -> (u64, u64, u64, u32, u32, u32, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mode(),
        metadata.uid(),
        metadata.gid(),
        metadata.nlink(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

pub(super) fn same_directory(left: &Dir, right: &Dir) -> Result<(), &'static str> {
    if identity(left)? != identity(right)? {
        return Err(ERROR);
    }
    Ok(())
}

pub(super) fn child(parent: &Dir, name: &str) -> Result<Dir, &'static str> {
    let directory = parent.open_dir_nofollow(name).map_err(|_| ERROR)?;
    let reopened = parent.open_dir_nofollow(name).map_err(|_| ERROR)?;
    same_directory(&directory, &reopened)?;
    Ok(directory)
}

fn regular(parent: &Dir, name: &str) -> Result<Handle, &'static str> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).custom_flags(0o4000); // Linux O_NONBLOCK.
    parent
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
        .and_then(Handle::from_file)
        .map_err(|_| ERROR)
}

pub(super) struct CapturedManifest {
    pub(super) bytes: Vec<u8>,
    original: Handle,
    before: Metadata,
    resources: Handle,
    resources_before: Metadata,
}

impl CapturedManifest {
    pub(super) fn revalidate(&self, resources: &Dir) -> Result<(), &'static str> {
        let current = regular(resources, "materials-manifest.json")?;
        if self.original != current
            || identity(resources)? != self.resources
            || state(&self.before) != state(&self.original.as_file().metadata().map_err(|_| ERROR)?)
            || state(&self.before) != state(&current.as_file().metadata().map_err(|_| ERROR)?)
            || state(&self.resources_before)
                != state(&self.resources.as_file().metadata().map_err(|_| ERROR)?)
        {
            return Err(ERROR);
        }
        Ok(())
    }
}

pub(super) fn read_manifest(resources: &Dir) -> Result<CapturedManifest, &'static str> {
    let mut file = regular(resources, "materials-manifest.json")?;
    let before = file.as_file().metadata().map_err(|_| ERROR)?;
    if !before.is_file()
        || before.uid() != 0
        || before.gid() != 0
        || before.mode() & 0o7777 != 0o444
        || before.nlink() != 1
        || before.len() == 0
        || before.len() > 262_144
    {
        return Err(ERROR);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(before.len()).map_err(|_| ERROR)?);
    file.as_file_mut().take(262_145).read_to_end(&mut bytes).map_err(|_| ERROR)?;
    let reopened = regular(resources, "materials-manifest.json")?;
    if file != reopened
        || state(&before) != state(&file.as_file().metadata().map_err(|_| ERROR)?)
        || state(&before) != state(&reopened.as_file().metadata().map_err(|_| ERROR)?)
        || bytes.len() > 262_144
    {
        return Err(ERROR);
    }
    let retained = identity(resources)?;
    let resources_before = retained.as_file().metadata().map_err(|_| ERROR)?;
    Ok(CapturedManifest { bytes, original: file, before, resources: retained, resources_before })
}

fn file(parent: &Dir, material: &Material, uid: u32, gid: u32) -> Result<(), &'static str> {
    let mut opened = regular(parent, name(&material.path))?;
    let before = opened.as_file().metadata().map_err(|_| ERROR)?;
    if !before.is_file()
        || before.uid() != uid
        || before.gid() != gid
        || before.nlink() != 1
        || before.mode() & 0o7777 != material.mode
        || before.len() != material.bytes
    {
        return Err(ERROR);
    }
    let mut reader = opened.as_file_mut().take(material.bytes + 1);
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 65_536];
    let mut total = 0_u64;
    loop {
        let count = reader.read(&mut buffer).map_err(|_| ERROR)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(u64::try_from(count).map_err(|_| ERROR)?).ok_or(ERROR)?;
        if total > material.bytes {
            return Err(ERROR);
        }
        hash.update(&buffer[..count]);
    }
    let reopened = regular(parent, name(&material.path))?;
    if total != material.bytes
        || format!("{:x}", hash.finalize()) != material.sha256
        || opened != reopened
        || state(&before) != state(&opened.as_file().metadata().map_err(|_| ERROR)?)
        || state(&before) != state(&reopened.as_file().metadata().map_err(|_| ERROR)?)
    {
        return Err(ERROR);
    }
    Ok(())
}

pub(super) fn walk(
    directory: &Dir,
    path: &str,
    manifest: &Manifest,
    uid: u32,
    gid: u32,
) -> Result<(), &'static str> {
    let retained = identity(directory)?;
    let before = retained.as_file().metadata().map_err(|_| ERROR)?;
    if !before.is_dir()
        || before.uid() != uid
        || before.gid() != gid
        || before.mode() & 0o7777 != 0o555
    {
        return Err(ERROR);
    }
    let expected = manifest
        .directories
        .iter()
        .filter(|item| parent(item) == path)
        .map(|item| name(item).to_owned())
        .chain(
            manifest
                .files
                .iter()
                .filter(|item| parent(&item.path) == path)
                .map(|item| name(&item.path).to_owned()),
        )
        .collect::<BTreeSet<_>>();
    let inventory = || {
        directory
            .entries()
            .map_err(|_| ERROR)?
            .map(|entry| entry.map_err(|_| ERROR)?.file_name().into_string().map_err(|_| ERROR))
            .collect::<Result<BTreeSet<_>, _>>()
    };
    if inventory()? != expected {
        return Err(ERROR);
    }
    for item in manifest.files.iter().filter(|item| parent(&item.path) == path) {
        file(directory, item, uid, gid)?;
    }
    for item in manifest.directories.iter().filter(|item| parent(item) == path) {
        let nested = child(directory, name(item))?;
        walk(&nested, item, manifest, uid, gid)?;
        same_directory(&nested, &child(directory, name(item))?)?;
    }
    if inventory()? != expected
        || state(&before) != state(&retained.as_file().metadata().map_err(|_| ERROR)?)
    {
        return Err(ERROR);
    }
    Ok(())
}
