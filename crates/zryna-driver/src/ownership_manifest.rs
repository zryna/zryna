//! Strict deterministic manifest v3 for internal DataOwnershipV1 candidate bundles.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zryna_abi::{ScalarOutcome, ScalarValue};
use zryna_diagnostics::{Diagnostic, PrimaryLocation};

use crate::{
    DATA_OWNERSHIP_CANDIDATE_PROFILE, DataOwnershipCandidateSuccess, ModuleEdge, ModuleRecord,
};

/// Exact filename of the candidate bundle manifest.
pub const OWNERSHIP_MANIFEST_NAME: &str = "zryna-manifest-v3.json";
/// Hard serialized manifest limit.
pub const MAX_OWNERSHIP_MANIFEST_BYTES: usize = 32 * 1_024 * 1_024;
const PROTOCOL_VERSION: u8 = 4;
const MANIFEST_VERSION: u8 = 3;
const GRAPH_DOMAIN: &[u8] = b"ZRYNA-M3-GRAPH\0";

mod validation;
use validation::{validate, validate_results};

#[cfg(test)]
mod tests;

/// Canonical target order in manifest v3.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnershipTarget {
    /// Deterministic ECMAScript module.
    JavaScript,
    /// Memory-bearing core WebAssembly module.
    WebAssembly,
    /// Linux x86-64 ELF artifact.
    Native,
}

/// One normalized typed target observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipManifestResult {
    target: OwnershipTarget,
    outcome: ScalarOutcome,
}

impl OwnershipManifestResult {
    /// Creates one target result for canonical manifest rendering.
    #[must_use]
    pub const fn new(target: OwnershipTarget, outcome: ScalarOutcome) -> Self {
        Self { target, outcome }
    }
    /// Returns the observed target.
    #[must_use]
    pub const fn target(&self) -> OwnershipTarget {
        self.target
    }
    /// Returns the normalized typed outcome.
    #[must_use]
    pub const fn outcome(&self) -> ScalarOutcome {
        self.outcome
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ManifestCommand {
    Build,
    Run,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestSource {
    id: u32,
    path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestEdge {
    importer: String,
    target: String,
    specifier: String,
    imported: String,
    local: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LayoutIdentity {
    type_universe_sha256: String,
    linear32_sha256: String,
    linux_x86_64_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RuntimeAbiIdentity {
    identifier: String,
    version: u8,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "profile", rename_all = "kebab-case", deny_unknown_fields)]
enum TargetMetadata {
    EcmascriptModule {
        scalar_abi: String,
    },
    CoreWebassembly {
        memory: String,
        imports: bool,
        scalar_abi: String,
    },
    LinuxX86_64Elf {
        triple: String,
        format: String,
        scalar_abi: String,
        program_object_sha256: String,
        runtime_object_sha256: Option<String>,
        harness_sha256: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestArtifact {
    target: OwnershipTarget,
    kind: String,
    filename: String,
    bytes: u64,
    sha256: String,
    metadata: TargetMetadata,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestInvocation {
    export: String,
    arguments: Vec<ScalarValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ManifestLocation {
    Global,
    WorkspacePath { path: String },
    Source { file: u32, start: u32, end: u32 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestDiagnostic {
    code: String,
    severity: zryna_diagnostics::Severity,
    location: ManifestLocation,
    message: String,
    guidance: String,
}

/// Complete strict candidate manifest schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipManifestV3 {
    version: u8,
    profile: String,
    protocol_version: u8,
    command: ManifestCommand,
    entrypoint: String,
    graph_sha256: String,
    sources: Vec<ManifestSource>,
    edges: Vec<ManifestEdge>,
    layouts: LayoutIdentity,
    runtime_abi: RuntimeAbiIdentity,
    stem: String,
    targets: Vec<OwnershipTarget>,
    artifacts: Vec<ManifestArtifact>,
    invocation: Option<ManifestInvocation>,
    results: Vec<OwnershipManifestResult>,
    diagnostics: Vec<ManifestDiagnostic>,
}

impl OwnershipManifestV3 {
    /// Returns the exact version.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }
    /// Returns the exact candidate profile.
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }
    /// Returns selected targets in canonical order.
    #[must_use]
    pub fn targets(&self) -> &[OwnershipTarget] {
        &self.targets
    }
    /// Returns typed run results.
    #[must_use]
    pub fn results(&self) -> &[OwnershipManifestResult] {
        &self.results
    }
}

/// Renders exact pretty JSON plus one trailing newline from a sealed candidate authority.
///
/// # Errors
/// Returns one stable diagnostic for incompatible results, identity, names, or resource bounds.
pub fn render_ownership_manifest_v3(
    success: &DataOwnershipCandidateSuccess,
    stem: &str,
    results: &[OwnershipManifestResult],
) -> Result<Vec<u8>, Diagnostic> {
    crate::javascript::validate_artifact_stem(stem)?;
    if stem != success.artifact_stem() {
        return Err(error("manifest stem does not match the authenticated candidate request"));
    }
    let command = if success.logical_export().is_some() {
        ManifestCommand::Run
    } else {
        ManifestCommand::Build
    };
    let sources = success.closure().modules().iter().map(source_record).collect();
    let edges = success.closure().edges().iter().map(edge_record).collect();
    let ir = success.program().verified_ir();
    let abi = success.program().runtime_abi();
    let layouts = LayoutIdentity {
        type_universe_sha256: hex(&ir.type_universe_identity().as_bytes()),
        linear32_sha256: hex(ir.linear32_layouts().fingerprint()),
        linux_x86_64_sha256: hex(ir.linux_x86_64_layouts().fingerprint()),
    };
    let runtime_abi = RuntimeAbiIdentity {
        identifier: abi.identifier().to_owned(),
        version: 1,
        sha256: hex(&abi.identity().as_bytes()),
    };
    let artifacts = artifact_records(success, stem)?;
    let targets = artifacts.iter().map(|artifact| artifact.target).collect::<Vec<_>>();
    validate_results(command, &targets, results)?;
    let invocation = success.logical_export().map(|name| ManifestInvocation {
        export: name.to_owned(),
        arguments: success.arguments().to_vec(),
    });
    let manifest = OwnershipManifestV3 {
        version: MANIFEST_VERSION,
        profile: DATA_OWNERSHIP_CANDIDATE_PROFILE.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        command,
        entrypoint: success.closure().entrypoint().as_str().to_owned(),
        graph_sha256: hex(success.closure().graph_sha256()),
        sources,
        edges,
        layouts,
        runtime_abi,
        stem: stem.to_owned(),
        targets,
        artifacts,
        invocation,
        results: results.to_vec(),
        diagnostics: success.diagnostics().iter().map(diagnostic_record).collect(),
    };
    validate(&manifest)?;
    encode(&manifest)
}

/// Decodes only the exact canonical manifest-v3 byte representation.
///
/// # Errors
/// Rejects oversized, malformed, reordered, duplicate, unknown, or inconsistent fields.
pub fn decode_ownership_manifest_v3(bytes: &[u8]) -> Result<OwnershipManifestV3, Diagnostic> {
    enforce_byte_limit(bytes.len())?;
    if !bytes.ends_with(b"\n") {
        return Err(error("manifest is not exact bounded newline-terminated v3 JSON"));
    }
    let manifest: OwnershipManifestV3 =
        serde_json::from_slice(bytes).map_err(|_| error("manifest does not match schema v3"))?;
    validate(&manifest)?;
    if encode(&manifest)? != bytes {
        return Err(error("manifest fields are not in canonical v3 byte order"));
    }
    Ok(manifest)
}

fn source_record(module: &ModuleRecord) -> ManifestSource {
    ManifestSource {
        id: module.id(),
        path: module.path().as_str().to_owned(),
        sha256: hex(module.source_sha256()),
    }
}

fn edge_record(edge: &ModuleEdge) -> ManifestEdge {
    ManifestEdge {
        importer: edge.importer().as_str().to_owned(),
        target: edge.target().as_str().to_owned(),
        specifier: edge.specifier().to_owned(),
        imported: edge.imported().to_owned(),
        local: edge.local().to_owned(),
    }
}

fn artifact_records(
    success: &DataOwnershipCandidateSuccess,
    stem: &str,
) -> Result<Vec<ManifestArtifact>, Diagnostic> {
    let mut records = Vec::with_capacity(3);
    let artifacts = success.artifacts();
    if let Some(artifact) = artifacts.javascript() {
        records.push(artifact_record(
            OwnershipTarget::JavaScript,
            "ecmascript-module",
            format!("{stem}.mjs"),
            artifact.source.as_bytes(),
            TargetMetadata::EcmascriptModule { scalar_abi: "v1".to_owned() },
        )?);
    }
    if let Some(artifact) = artifacts.webassembly() {
        records.push(artifact_record(
            OwnershipTarget::WebAssembly,
            "core-webassembly-module",
            format!("{stem}.wasm"),
            artifact.bytes(),
            TargetMetadata::CoreWebassembly {
                memory: "linear32-v1".to_owned(),
                imports: false,
                scalar_abi: "v1".to_owned(),
            },
        )?);
    }
    if let Some(artifact) = artifacts.native_object() {
        records.push(artifact_record(
            OwnershipTarget::Native,
            "linux-x86-64-relocatable-object",
            format!("{stem}.o"),
            artifact.bytes(),
            TargetMetadata::LinuxX86_64Elf {
                triple: "x86_64-unknown-linux-gnu".to_owned(),
                format: "elf-relocatable".to_owned(),
                scalar_abi: "v1".to_owned(),
                program_object_sha256: hex(&Sha256::digest(artifact.bytes()).into()),
                runtime_object_sha256: None,
                harness_sha256: None,
            },
        )?);
    }
    if let Some(artifact) = artifacts.native_executable() {
        let identity = artifact.identity();
        records.push(artifact_record(
            OwnershipTarget::Native,
            "linux-x86-64-invocation-executable",
            format!("{stem}.elf"),
            artifact.executable_bytes(),
            TargetMetadata::LinuxX86_64Elf {
                triple: identity.target().to_owned(),
                format: "elf-executable".to_owned(),
                scalar_abi: "v1".to_owned(),
                program_object_sha256: hex(identity.program_object_sha256()),
                runtime_object_sha256: Some(hex(identity.runtime_object_sha256())),
                harness_sha256: Some(hex(identity.harness_sha256())),
            },
        )?);
    }
    if records.is_empty() || records.windows(2).any(|pair| pair[0].target >= pair[1].target) {
        return Err(error("candidate artifacts are empty, duplicated, or out of canonical order"));
    }
    Ok(records)
}

fn artifact_record(
    target: OwnershipTarget,
    kind: &str,
    filename: String,
    bytes: &[u8],
    metadata: TargetMetadata,
) -> Result<ManifestArtifact, Diagnostic> {
    Ok(ManifestArtifact {
        target,
        kind: kind.to_owned(),
        filename,
        bytes: u64::try_from(bytes.len()).map_err(|_| error("artifact length exceeds u64"))?,
        sha256: hex(&Sha256::digest(bytes).into()),
        metadata,
    })
}

fn diagnostic_record(item: &Diagnostic) -> ManifestDiagnostic {
    let location = match item.primary() {
        PrimaryLocation::Global => ManifestLocation::Global,
        PrimaryLocation::WorkspacePath { path } => {
            ManifestLocation::WorkspacePath { path: path.clone() }
        }
        PrimaryLocation::Source { span } => ManifestLocation::Source {
            file: span.file().index(),
            start: span.start(),
            end: span.end(),
        },
    };
    ManifestDiagnostic {
        code: item.code().to_owned(),
        severity: item.severity(),
        location,
        message: item.message().to_owned(),
        guidance: item.guidance().to_owned(),
    }
}

fn encode(manifest: &OwnershipManifestV3) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes =
        serde_json::to_vec_pretty(manifest).map_err(|_| error("manifest encoding failed"))?;
    bytes.push(b'\n');
    enforce_byte_limit(bytes.len())?;
    Ok(bytes)
}

fn enforce_byte_limit(length: usize) -> Result<(), Diagnostic> {
    if length > MAX_OWNERSHIP_MANIFEST_BYTES {
        Err(error("manifest exceeds the fixed v3 byte budget"))
    } else {
        Ok(())
    }
}

fn push_text(bytes: &mut Vec<u8>, value: &str) -> Result<(), Diagnostic> {
    push_u32(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn push_u32(bytes: &mut Vec<u8>, value: impl TryInto<u32>) -> Result<(), Diagnostic> {
    let value = value.try_into().map_err(|_| error("manifest count exceeds u32"))?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    output
}

fn is_hex(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_hex(value: &str) -> Result<[u8; 32], Diagnostic> {
    if !is_hex(value) {
        return Err(error("manifest contains a malformed SHA-256 value"));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(output)
}

fn hex_nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}

fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C3101",
        None,
        message,
        "use the exact canonical authenticated DataOwnershipV1 manifest v3 schema",
    )
}
