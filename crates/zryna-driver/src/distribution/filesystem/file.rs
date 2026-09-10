use std::{
    fs,
    io::{Read as _, Seek as _, SeekFrom},
};

use cap_fs_ext::{FollowSymlinks, MetadataExt as _, OpenOptionsFollowExt as _};
use cap_std::fs::Dir;
use same_file::Handle;
use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;

use super::{changed, platform};

pub(super) struct RetainedFile {
    pub(super) identity: Handle,
    pub(super) metadata: fs::Metadata,
    pub(super) sha256: String,
    pub(super) bytes: Option<Vec<u8>>,
}

impl RetainedFile {
    pub(super) fn open(
        parent: &Dir,
        name: &str,
        limit: u64,
        retain: bool,
    ) -> Result<Self, Diagnostic> {
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        platform::configure_read(&mut options);
        let file = parent.open_with(name, &options).map_err(|_| changed())?;
        // This metadata originates from the open handle, including Windows link count.
        if file.metadata().map_err(|_| changed())?.nlink() != 1 {
            return Err(changed());
        }
        let identity = Handle::from_file(file.into_std()).map_err(|_| changed())?;
        let metadata = identity.as_file().metadata().map_err(|_| changed())?;
        if !metadata.is_file() || platform::link_or_reparse(&metadata) || metadata.len() > limit {
            return Err(changed());
        }
        let (sha256, bytes) = read(&identity, limit, retain)?;
        let after = identity.as_file().metadata().map_err(|_| changed())?;
        if !platform::same_state(&metadata, &after) {
            return Err(changed());
        }
        Ok(Self { identity, metadata, sha256, bytes })
    }

    pub(super) fn mode_matches(&self, mode: u32) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            self.metadata.mode() & 0o7777 == mode
        }
        #[cfg(windows)]
        {
            [0o600, 0o644, 0o700, 0o755].contains(&mode)
        }
    }
}

fn read(
    identity: &Handle,
    limit: u64,
    retain: bool,
) -> Result<(String, Option<Vec<u8>>), Diagnostic> {
    let mut file = identity.as_file().try_clone().map_err(|_| changed())?;
    file.seek(SeekFrom::Start(0)).map_err(|_| changed())?;
    let mut reader = file.take(limit + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = retain.then(Vec::new);
    let mut total = 0_u64;
    loop {
        let count = reader.read(&mut buffer).map_err(|_| changed())?;
        if count == 0 {
            break;
        }
        total += u64::try_from(count).map_err(|_| changed())?;
        if total > limit {
            return Err(changed());
        }
        digest.update(&buffer[..count]);
        if let Some(bytes) = &mut bytes {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    Ok((format!("{:x}", digest.finalize()), bytes))
}
