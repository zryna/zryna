use std::{
    collections::BTreeSet,
    io::{self, Write as _},
};

use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::Dir;
use same_file::Handle;

use super::PackageBuildError;

pub(super) fn record_path(entries: &mut BTreeSet<String>, relative: &str) {
    let mut prefix = String::new();
    for (index, component) in relative.split('/').enumerate() {
        if index > 0 {
            prefix.push('/');
        }
        prefix.push_str(component);
        entries.insert(prefix.clone());
    }
}

pub(super) fn write_file(
    root: &Dir,
    relative: &str,
    bytes: &[u8],
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    let mut directory =
        root.try_clone().map_err(|_| error("private stage capability cannot be cloned"))?;
    let mut components = relative.split('/').peekable();
    while let Some(component) = components.next() {
        if components.peek().is_some() {
            match directory.create_dir(component) {
                Ok(()) => {}
                Err(create_error) if create_error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(error("private stage directory cannot be created")),
            }
            directory = directory
                .open_dir_nofollow(component)
                .map_err(|_| error("private stage directory is unsafe"))?;
            continue;
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true).follow(FollowSymlinks::No);
        configure_create(&mut options);
        let mut file = directory
            .open_with(component, &options)
            .map_err(|_| error("private stage file cannot be created"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| error("private stage file cannot be synchronized"))?;
        return Ok(());
    }
    Err(error("private stage file path is empty"))
}

pub(super) fn sync_tree(
    directory: &Dir,
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    for entry in directory.entries().map_err(|_| error("private stage cannot be synchronized"))? {
        let entry = entry.map_err(|_| error("private stage cannot be synchronized"))?;
        let file_type =
            entry.file_type().map_err(|_| error("private stage entry cannot be inspected"))?;
        if file_type.is_dir() {
            let child = directory
                .open_dir_nofollow(entry.file_name())
                .map_err(|_| error("private stage directory changed"))?;
            sync_tree(&child, error)?;
        } else if !file_type.is_file() {
            return Err(error("private stage contains an unsafe path"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd as _;

        let retained_path = format!("/proc/{}/fd/{}", std::process::id(), directory.as_raw_fd());
        std::fs::File::open(retained_path)
            .and_then(|file| file.sync_all())
            .map_err(|_| error("private stage directory cannot be synchronized"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

pub(super) fn revalidate_name(
    parent: &Dir,
    stage_name: &str,
    stage: &Dir,
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    let current = parent
        .open_dir_nofollow(stage_name)
        .map_err(|_| error("private stage name cannot be revalidated"))?;
    if directory_identity(&current, error)? != directory_identity(stage, error)? {
        return Err(error("private stage name no longer selects its retained capability"));
    }
    Ok(())
}

pub(super) fn cleanup_known_stage(
    parent: &Dir,
    stage_name: &str,
    stage: &Dir,
    expected: &BTreeSet<String>,
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    cleanup_known_stage_with_hook(parent, stage_name, stage, expected, error, || {})
}

#[cfg(test)]
pub(super) fn cleanup_known_stage_with_substitution_hook(
    parent: &Dir,
    stage_name: &str,
    stage: &Dir,
    expected: &BTreeSet<String>,
    error: fn(&str) -> PackageBuildError,
    before_remove: impl FnOnce(),
) -> Result<(), PackageBuildError> {
    cleanup_known_stage_with_hook(parent, stage_name, stage, expected, error, before_remove)
}

fn cleanup_known_stage_with_hook(
    parent: &Dir,
    stage_name: &str,
    stage: &Dir,
    expected: &BTreeSet<String>,
    error: fn(&str) -> PackageBuildError,
    before_remove: impl FnOnce(),
) -> Result<(), PackageBuildError> {
    let mut actual = BTreeSet::new();
    collect(stage, "", &mut actual, error)?;
    if !actual.is_subset(expected) {
        return Err(error("private stage contains an unexpected cleanup path"));
    }
    before_remove();
    let mut paths = actual.into_iter().collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        right.matches('/').count().cmp(&left.matches('/').count()).then_with(|| right.cmp(left))
    });
    for path in paths {
        remove_entry(stage, &path, error)?;
    }
    parent.remove_dir(stage_name).map_err(|_| error("private stage directory could not be removed"))
}

fn collect(
    directory: &Dir,
    prefix: &str,
    entries: &mut BTreeSet<String>,
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    for entry in
        directory.entries().map_err(|_| error("private stage cannot be enumerated for cleanup"))?
    {
        let entry =
            entry.map_err(|_| error("private stage entry is unavailable during cleanup"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| error("private stage contains a non-portable cleanup path"))?;
        let relative = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
        let file_type = entry
            .file_type()
            .map_err(|_| error("private stage entry cannot be inspected during cleanup"))?;
        if file_type.is_dir() {
            let child = directory
                .open_dir_nofollow(entry.file_name())
                .map_err(|_| error("private stage directory changed during cleanup"))?;
            entries.insert(relative.clone());
            collect(&child, &relative, entries, error)?;
        } else if file_type.is_file() {
            entries.insert(relative);
        } else {
            return Err(error("private stage contains an unsafe cleanup path"));
        }
    }
    Ok(())
}

fn remove_entry(
    root: &Dir,
    relative: &str,
    error: fn(&str) -> PackageBuildError,
) -> Result<(), PackageBuildError> {
    let mut directory = root
        .try_clone()
        .map_err(|_| error("private stage capability cannot be cloned for cleanup"))?;
    let mut components = relative.split('/').peekable();
    while let Some(component) = components.next() {
        if components.peek().is_some() {
            directory = directory
                .open_dir_nofollow(component)
                .map_err(|_| error("private stage directory changed during cleanup"))?;
            continue;
        }
        return directory
            .remove_file(component)
            .or_else(|_| directory.remove_dir(component))
            .map_err(|_| error("private stage entry could not be removed"));
    }
    Err(error("private stage cleanup path is empty"))
}

fn directory_identity(
    directory: &Dir,
    error: fn(&str) -> PackageBuildError,
) -> Result<Handle, PackageBuildError> {
    directory
        .try_clone()
        .map(Dir::into_std_file)
        .and_then(Handle::from_file)
        .map_err(|_| error("private stage identity cannot be retained"))
}

#[cfg(unix)]
fn configure_create(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn configure_create(_options: &mut cap_std::fs::OpenOptions) {}
