//! Closed ordinary USTAR projection for private recipe inputs and outputs.
mod header;
mod parse;
mod root;
mod write;

use std::collections::BTreeMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    Deadline,
    Limit,
    Format,
    Inventory,
    Identity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Root {
    Input,
    Output,
}

impl Root {
    fn name(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
}

/// Exact numeric ownership and permission expectations; not an approval by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    /// Numeric owner UID.
    pub uid: u32,
    /// Numeric owner GID.
    pub gid: u32,
    /// Permission bits without the filesystem type bits.
    pub mode: u32,
}

#[derive(Clone, Debug)]
pub(super) struct FileIdentity {
    pub(super) metadata: Metadata,
    pub(super) bytes: usize,
    pub(super) sha256: [u8; 32],
}

#[derive(Clone, Debug)]
pub(super) struct Expected {
    pub(super) root: Root,
    pub(super) directory: Metadata,
    pub(super) files: BTreeMap<String, FileIdentity>,
}

/// An independently expected root-tree entry, carrying no execution authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RootKind {
    /// A zero-payload directory.
    Directory,
    /// One regular file with exact body identity.
    File {
        /// Exact byte length.
        bytes: usize,
        /// Exact SHA-256 digest.
        sha256: [u8; 32],
    },
    /// One explicitly expected symbolic link.
    Symlink {
        /// Exact absolute canonical target path.
        target: String,
    },
}

/// Metadata and kind of one independently approved root-tree member.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootEntry {
    /// Exact numeric metadata.
    pub metadata: Metadata,
    /// Exact member kind and body or link identity.
    pub kind: RootKind,
}

pub(super) fn extract_root(
    deadline: Instant,
    bytes: &[u8],
    expected: &BTreeMap<String, RootEntry>,
    archive_limit: usize,
) -> Result<(), Error> {
    root::extract(deadline, bytes, expected, archive_limit)
}

pub(super) fn write_input(
    deadline: Instant,
    files: &BTreeMap<String, Vec<u8>>,
    archive_limit: usize,
) -> Result<Vec<u8>, Error> {
    write::input(deadline, files, archive_limit)
}

pub(super) fn extract(
    deadline: Instant,
    bytes: &[u8],
    expected: &Expected,
    archive_limit: usize,
) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    parse::extract(deadline, bytes, expected, archive_limit)
}

fn check(deadline: Instant) -> Result<(), Error> {
    if Instant::now() >= deadline { Err(Error::Deadline) } else { Ok(()) }
}

fn basename(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 80
        && name != "."
        && name != ".."
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn padded(size: usize) -> Result<usize, Error> {
    size.checked_add(511).map(|size| size / 512 * 512).ok_or(Error::Limit)
}

#[cfg(test)]
mod tests;
