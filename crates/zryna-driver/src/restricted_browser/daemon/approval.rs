use super::{PlaygroundDaemon, PlaygroundDaemonMaterialPin, Result, materials, policy, reject};
use serde::Deserialize;

/// Independently selected host/image/policy expectations, never submission authority.
///
/// The caller must authenticate the entire record through its trusted operator channel.
/// Deserialization and matching checksums alone do not authenticate its origin.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaygroundDaemonOperatorApproval {
    /// Closed source-composition approval version, currently two.
    pub version: u32,
    /// Exactly `reviewed-source-v1`; no standalone toolkit publisher is implied.
    pub composition_kind: String,
    /// Independently reviewed finite source-wrapper/UI/helper composition identity.
    pub composition_sha256: String,
    /// Independently selected owner of the fixed local Engine socket and peer.
    pub engine_uid: u32,
    /// Full immutable compiler image SHA-256, without the `sha256:` prefix.
    pub compiler_image_sha256: String,
    /// Full immutable evaluation image SHA-256, without the prefix.
    pub evaluator_image_sha256: String,
    /// Complete canonical compiler material-manifest identity, distinct from image or binary.
    pub compiler_root_sha256: String,
    /// Complete canonical evaluator material-manifest identity, distinct from image or binary.
    pub evaluator_root_sha256: String,
    /// Operator-provisioned compiler materials; runtime receives read-only access only.
    pub compiler_materials: PlaygroundDaemonMaterialPin,
    /// Independently provisioned evaluation materials; never an owned writable volume.
    pub evaluator_materials: PlaygroundDaemonMaterialPin,
    /// Authenticated actual playground compiler executable, distinct from root inventory.
    pub compiler_executable_sha256: String,
    /// Authenticated existing deterministic binding template.
    pub binding_template_sha256: String,
    /// Authenticated compiler-owned complete WIT closure identity.
    pub wit_closure_sha256: String,
    /// Two distinct ascending host CPU indices approved for both fixed roles.
    pub cpus: [u16; 2],
    /// Independently reviewed canonical default-deny compiler seccomp document.
    pub compiler_seccomp_json: String,
    /// Independently reviewed canonical default-deny evaluator seccomp document.
    pub evaluator_seccomp_json: String,
    /// Exact independently approved composition policy identity.
    pub policy_sha256: String,
}

#[derive(Clone)]
pub(super) struct Approved {
    pub record: PlaygroundDaemonOperatorApproval,
    pub compiler_seccomp: serde_json::Value,
    pub evaluator_seccomp: serde_json::Value,
}

pub(super) fn hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Retains an explicitly selected fixed playground composition without executing anything.
///
/// # Errors
/// Rejects unknown records, malformed pins, unsafe seccomp shapes or a changed policy.
pub fn approve_playground_daemon(
    record: PlaygroundDaemonOperatorApproval,
) -> Result<PlaygroundDaemon> {
    let approved = validate(record)?;
    Ok(PlaygroundDaemon { approval: approved, poisoned: false })
}

pub(super) fn validate(record: PlaygroundDaemonOperatorApproval) -> Result<Approved> {
    if record.version != 2
        || record.composition_kind != "reviewed-source-v1"
        || record.cpus[0] >= record.cpus[1]
        || ![
            &record.compiler_image_sha256,
            &record.evaluator_image_sha256,
            &record.compiler_root_sha256,
            &record.evaluator_root_sha256,
            &record.compiler_executable_sha256,
            &record.binding_template_sha256,
            &record.wit_closure_sha256,
            &record.composition_sha256,
            &record.policy_sha256,
        ]
        .iter()
        .all(|value| hash(value))
    {
        return Err(reject("PLAYGROUND-HOST-APPROVAL", "closed independent host pins required"));
    }
    materials::validate_pin(&record.compiler_materials, false, &record.compiler_root_sha256)?;
    materials::validate_pin(&record.evaluator_materials, true, &record.evaluator_root_sha256)?;
    if record.compiler_materials.name == record.evaluator_materials.name {
        return Err(reject(
            "PLAYGROUND-HOST-APPROVAL",
            "distinct compiler and evaluation material roots required",
        ));
    }
    let compiler_seccomp = policy::seccomp(&record.compiler_seccomp_json)?;
    let evaluator_seccomp = policy::seccomp(&record.evaluator_seccomp_json)?;
    let approved = Approved { record, compiler_seccomp, evaluator_seccomp };
    if policy::identity(&approved)? != approved.record.policy_sha256 {
        return Err(reject("PLAYGROUND-HOST-POLICY", "independently selected policy differs"));
    }
    Ok(approved)
}
