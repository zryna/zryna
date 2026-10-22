//! Independently approved complete immutable root, including explicit daemon metadata entries.
use super::super::archive::{RootEntry, RootKind};
use super::client::digest_id;
use super::{Result, fail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, time::Instant};

/// Approval data supplied by the trusted operator channel, never read from a package or cache.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeToolRootApproval {
    /// Independently approved owner UID of the protected fixed local Engine socket and peer.
    pub engine_uid: u32,
    /// Raw lowercase immutable Docker image digest, with no tag or endpoint.
    pub image_sha256: String,
    /// Complete created-holder root archive inventory, including fixed empty Docker entries.
    pub entries: BTreeMap<String, RootEntry>,
    /// Complete executable regular files: absolute path to SHA-256; never a host lookup path.
    pub tools: BTreeMap<String, String>,
}
/// Computes the approval's identity without authenticating its origin or granting execution.
/// # Errors
/// Rejects open, unsafe, oversized, or inconsistent material inventories.
pub fn native_tool_root_identity(
    root: &NativeToolRootApproval,
) -> std::result::Result<String, super::super::plan::NativeRecipeError> {
    identity(root).map_err(super::Failure::into_error)
}
pub(super) fn identity(root: &NativeToolRootApproval) -> Result<String> {
    if !digest_id(&root.image_sha256)
        || root.entries.is_empty()
        || root.entries.len() > 128
        || root.tools.len() != 5
    {
        return Err(fail("complete bounded tool root approval required"));
    }
    let required = [
        "/lib64/ld-linux-x86-64.so.2",
        "/supervisor",
        "/tool/bin/as",
        "/tool/bin/gcc",
        "/tool/libexec/cc1",
    ];
    if root.tools.keys().map(String::as_str).ne(required) {
        return Err(fail("fixed complete compiler tool inventory required"));
    }
    validate_entries(root)?;
    for (path, hash) in &root.tools {
        if !digest_id(hash)
            || !matches!(root.entries.get(path).map(|entry| &entry.kind), Some(RootKind::File{sha256,..}) if hex(sha256)==*hash)
        {
            return Err(fail("tool body identity differs"));
        }
    }
    let mut value =
        serde_json::to_value(root).map_err(|_| fail("tool root identity encoding failed"))?;
    value.sort_all_objects();
    let mut bytes = b"ZRYNA-NATIVE-ROOT-APPROVAL-V0\0".to_vec();
    bytes.extend(
        serde_json::to_vec(&value).map_err(|_| fail("tool root identity encoding failed"))?,
    );
    Ok(hex(&Sha256::digest(bytes)))
}
fn validate_entries(root: &NativeToolRootApproval) -> Result<()> {
    let (mut regular, mut directories, mut total, mut data) = (0usize, 0usize, 0usize, 0usize);
    for (path, entry) in &root.entries {
        if !path.is_empty()
            && (path.len() > 96
                || !path.starts_with('/')
                || path[1..].split('/').any(|part| {
                    part.is_empty()
                        || part == "."
                        || part == ".."
                        || !part
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
                }))
        {
            return Err(fail("closed canonical root inventory paths required"));
        }
        if entry.metadata.mode & !0o777 != 0
            || (entry.metadata.uid != 0 && path != "/output")
            || (entry.metadata.gid != 0 && path != "/output")
        {
            return Err(fail("tool root contains additional ownership or special mode authority"));
        }
        match &entry.kind {
            RootKind::Directory => {
                let (uid, gid, mode) = match path.as_str() {
                    "/input" | "/control" => (0, 0, 0o555),
                    "/output" => (65534, 65534, 0o700),
                    _ => (0, 0, 0o755),
                };
                if entry.metadata.uid != uid
                    || entry.metadata.gid != gid
                    || entry.metadata.mode != mode
                {
                    return Err(fail("fixed root directory ownership/mode differs"));
                }
                directories += 1;
                if directories > 64 {
                    return Err(fail("directory inventory exceeds bound"));
                }
            }
            RootKind::File { bytes, sha256 } => {
                regular += 1;
                total = total.checked_add(*bytes).ok_or_else(|| fail("tool root size overflow"))?;
                let executable = entry.metadata.mode & 0o111 != 0 && *bytes != 0;
                if executable {
                    if entry.metadata.mode != 0o555 || root.tools.get(path) != Some(&hex(sha256)) {
                        return Err(fail("undeclared executable material"));
                    }
                    if *bytes > 64 * 1024 * 1024 {
                        return Err(fail("tool material exceeds bound"));
                    }
                } else {
                    if *bytes == 0 {
                        if !matches!(
                            path.as_str(),
                            "/.dockerenv"
                                | "/dev/console"
                                | "/etc/hostname"
                                | "/etc/hosts"
                                | "/etc/resolv.conf"
                        ) || entry.metadata.mode != 0o755
                        {
                            return Err(fail("unexpected empty daemon metadata entry"));
                        }
                    } else if entry.metadata.mode != 0o444 {
                        return Err(fail("readonly root material mode differs"));
                    }
                    data = data
                        .checked_add(*bytes)
                        .ok_or_else(|| fail("readonly material size overflow"))?;
                    if *bytes > 32 * 1024 * 1024 {
                        return Err(fail("readonly material exceeds bound"));
                    }
                    if root.tools.contains_key(path) {
                        return Err(fail("approved executable tool absent"));
                    }
                }
                if regular > 66 || total > 128 * 1024 * 1024 || data > 32 * 1024 * 1024 {
                    return Err(fail("tool root material inventory exceeds bound"));
                }
            }
            RootKind::Symlink { target } => {
                if entry.metadata.mode != 0o777 {
                    return Err(fail("fixed root alias mode differs"));
                }
                if !matches!(
                    (path.as_str(), target.as_str()),
                    ("/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2", "/lib64/ld-linux-x86-64.so.2")
                        | ("/etc/mtab", "/proc/mounts")
                ) {
                    return Err(fail("undeclared tool root alias"));
                }
            }
        }
    }
    Ok(())
}
pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    encoded
}
pub(super) fn verify(deadline: Instant, bytes: &[u8], root: &NativeToolRootApproval) -> Result<()> {
    super::super::archive::extract_root(deadline, bytes, &root.entries, 128 * 1024 * 1024)
        .map_err(|_| fail("actual complete tool root inventory differs from approval"))
}
