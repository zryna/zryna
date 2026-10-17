use super::{PlaygroundDaemonOperatorApproval, Result, approval::hash, client::Client, reject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Independently selected persistent operator material volume; never guest-owned storage.
///
/// A matching name or digest alone is not authentication. The operator must protect the volume
/// from writers, independently approve this record and the immutable image's complete manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlaygroundDaemonMaterialPin {
    /// Exact preprovisioned bounded lowercase local volume name.
    pub name: String,
    /// Exact independently observed daemon creation timestamp.
    pub created_at: String,
    /// Fresh operator provisioning nonce, 32 lowercase hexadecimal digits.
    pub nonce: String,
    /// SHA-256 of the canonical complete manifest fixed inside the immutable image.
    pub root_sha256: String,
    /// Exact complete regular-file inventory count, excluding directories.
    pub files: u32,
    /// Exact sum of complete regular-file byte lengths.
    pub bytes: u64,
}

pub(super) fn validate_pin(
    pin: &PlaygroundDaemonMaterialPin,
    evaluate: bool,
    expected_root: &str,
) -> Result<()> {
    let file_limit = if evaluate { 512 } else { 64 };
    let byte_limit = if evaluate { 1_073_741_824 } else { 201_326_592 };
    if !pin.name.starts_with("zryna410-materials-")
        || pin.name.len() > 96
        || !pin
            .name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !(20..=64).contains(&pin.created_at.len())
        || !pin.created_at.bytes().all(|byte| byte.is_ascii_digit() || b"-:TZ.+".contains(&byte))
        || pin.nonce.len() != 32
        || !pin.nonce.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || !hash(&pin.root_sha256)
        || pin.root_sha256 != expected_root
        || pin.files == 0
        || pin.files > file_limit
        || pin.bytes == 0
        || pin.bytes > byte_limit
    {
        return Err(reject(
            "PLAYGROUND-HOST-MATERIALS",
            "finite independently pinned material volume required",
        ));
    }
    Ok(())
}

pub(super) fn inspect(
    client: Client<'_>,
    record: &PlaygroundDaemonOperatorApproval,
    evaluate: bool,
) -> Result<()> {
    let pin = if evaluate { &record.evaluator_materials } else { &record.compiler_materials };
    let value = client.lookup("volumes", &pin.name)?.ok_or_else(|| {
        reject("PLAYGROUND-HOST-MATERIALS", "approved operator material volume unavailable")
    })?;
    validate_volume(&value, pin)
}

pub(super) fn validate_volume(value: &Value, pin: &PlaygroundDaemonMaterialPin) -> Result<()> {
    let no_options = value["Options"].is_null()
        || value["Options"].as_object().is_some_and(serde_json::Map::is_empty);
    if value["Name"] != pin.name
        || value["CreatedAt"] != pin.created_at
        || value["Driver"] != "local"
        || value["Scope"] != "local"
        || !no_options
        || value["Labels"]
            != json!({"zryna410.materials-nonce":pin.nonce,"zryna410.materials-root":pin.root_sha256})
    {
        return Err(reject(
            "PLAYGROUND-HOST-MATERIALS",
            "operator volume provenance or local read-only composition differs",
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    version: u32,
    #[serde(rename = "rootSha256")]
    root_sha256: String,
    files: u32,
    bytes: u64,
}

pub(super) fn audit_receipt(bytes: &[u8], pin: &PlaygroundDaemonMaterialPin) -> Result<()> {
    if bytes.is_empty() || bytes.len() > 1024 {
        return Err(reject(
            "PLAYGROUND-HOST-MATERIALS",
            "finite complete material audit receipt required",
        ));
    }
    let receipt: Audit = serde_json::from_slice(bytes).map_err(|_| {
        reject("PLAYGROUND-HOST-MATERIALS", "closed actual material audit receipt required")
    })?;
    if receipt.version != 1
        || receipt.root_sha256 != pin.root_sha256
        || receipt.files != pin.files
        || receipt.bytes != pin.bytes
    {
        return Err(reject(
            "PLAYGROUND-HOST-MATERIALS",
            "actual complete material audit differs from approval",
        ));
    }
    Ok(())
}
